use futures::TryStreamExt;
use std::{collections::HashMap, pin::Pin};
use strum::IntoEnumIterator;

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, FixedOffset};
use chrono_tz::Tz;
use futures::Stream;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use serde::{Deserialize, Serialize};
use sqlx::Error; // Should probably map/use anyhow::Error instead in the stream

use crate::trade::{
    Area, AreaSelection, Market, MarketSelection, Trade, TradeForReport, TradeSide,
};

#[derive(Debug)]
pub struct Report {
    _delivery_from: DateTime<Tz>,
    _delivery_to: DateTime<Tz>,
    areas: HashMap<Area, ReportEntry>,
}

impl Report {
    pub fn new(
        delivery_from: &DateTime<Tz>,
        delivery_to: &DateTime<Tz>,
        trades: Vec<Trade>,
    ) -> Result<Self> {
        if delivery_to < delivery_from {
            bail!("delivery_from has to be before delivery_to");
        }

        let mut areas = HashMap::new();

        for trade in trades.iter() {
            let area = trade.area;
            areas
                .entry(area)
                .or_insert(ReportEntry::new(area))
                .add_trade(trade)?;
        }

        let report = Report {
            _delivery_from: *delivery_from,
            _delivery_to: *delivery_to,
            areas,
        };

        Ok(report)
    }

    pub fn new_from_trade_for_report(
        delivery_from: &DateTime<Tz>,
        delivery_to: &DateTime<Tz>,
        trades: Vec<TradeForReport>,
    ) -> Result<Self> {
        if delivery_to < delivery_from {
            bail!("delivery_from has to be before delivery_to");
        }

        let mut areas = HashMap::new();

        for trade in trades.iter() {
            let area = trade.area;
            areas
                .entry(area)
                .or_insert(ReportEntry::new(area))
                .add_trade_for_report(trade)?;
        }

        let report = Report {
            _delivery_from: *delivery_from,
            _delivery_to: *delivery_to,
            areas,
        };

        Ok(report)
    }

    pub async fn new_from_stream<'a>(
        delivery_from: &DateTime<Tz>,
        delivery_to: &DateTime<Tz>,
        mut trades_iter: Pin<Box<dyn Stream<Item = Result<Trade, Error>> + Send + 'a>>,
    ) -> Result<Self> {
        if delivery_to < delivery_from {
            bail!("delivery_from has to be before delivery_to");
        }

        let mut areas = HashMap::new();

        while let Some(trade) = trades_iter.try_next().await? {
            let area = trade.area;
            areas
                .entry(area)
                .or_insert(ReportEntry::new(area))
                .add_trade(&trade)?;
        }

        let report = Report {
            _delivery_from: *delivery_from,
            _delivery_to: *delivery_to,
            areas,
        };

        Ok(report)
    }

    pub fn print_key_metrics(self) {
        println!(
            "Total gross profit: {:?}",
            self.gross_profit(MarketSelection::All, AreaSelection::All)
        );
        println!(
            "Total revenue: {:?}",
            self.revenue(MarketSelection::All, AreaSelection::All)
        );
        println!(
            "Total costs: {:?}",
            self.costs(MarketSelection::All, AreaSelection::All)
        );
        println!(
            "Total mw sold: {:?}",
            self.mw_sold(MarketSelection::All, AreaSelection::All)
        );
        println!(
            "Total mw bought: {:?}",
            self.mw_bought(MarketSelection::All, AreaSelection::All)
        );
    }

    fn aggregate_metric<F>(
        &self,
        market: MarketSelection,
        area_selection: AreaSelection,
        aggregator: F,
    ) -> Decimal
    where
        F: Fn(&ReportEntry, MarketSelection) -> Decimal,
    {
        match area_selection {
            AreaSelection::Specific(area) => self
                .areas
                .get(&area)
                .map_or(Decimal::ZERO, |entry| aggregator(entry, market)),
            AreaSelection::All => self
                .areas
                .values()
                .map(|entry| aggregator(entry, market))
                .sum(),
        }
    }

    pub fn revenue(&self, market: MarketSelection, area: AreaSelection) -> Decimal {
        let summed = self.aggregate_metric(market, area, |entry, market| entry.revenue(market));
        summed.round_dp(2)
    }

    pub fn costs(&self, market: MarketSelection, area: AreaSelection) -> Decimal {
        let summed = self.aggregate_metric(market, area, |entry, market| entry.costs(market));
        summed.round_dp(2)
    }

    pub fn mw_sold(&self, market: MarketSelection, area: AreaSelection) -> Decimal {
        let summed = self.aggregate_metric(market, area, |entry, market| entry.mw_sold(market));
        summed.round_dp(1)
    }

    pub fn mw_bought(&self, market: MarketSelection, area: AreaSelection) -> Decimal {
        let summed = self.aggregate_metric(market, area, |entry, market| entry.mw_bought(market));
        summed.round_dp(1)
    }

    pub fn gross_profit(&self, market: MarketSelection, area: AreaSelection) -> Decimal {
        let summed =
            self.aggregate_metric(market, area, |entry, market| entry.gross_profit(market));
        summed.round_dp(2)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ReportEntry {
    area: Area,
    mw: HashMap<(TradeSide, Market), Decimal>,
    cash_flow: HashMap<(TradeSide, Market), Decimal>,
}

impl ReportEntry {
    fn new(area: Area) -> Self {
        Self {
            area,
            mw: HashMap::new(),
            cash_flow: HashMap::new(),
        }
    }

    fn add_trade(&mut self, trade: &Trade) -> Result<()> {
        if trade.area != self.area {
            bail!("Trade area has to match ReportEntry area");
        }
        let Some(trade_price) = trade.price else {
            return Ok(());
        };

        let trade_side = trade.trade_side;
        let market = Market::from(trade.trade_type);
        let contract_length = contract_length(&trade.delivery_start, &trade.delivery_end)?;

        let abs_length_adjusted_quantity = trade.quantity_mwh.abs() * contract_length;

        *self.mw.entry((trade_side, market)).or_insert(Decimal::ZERO) +=
            abs_length_adjusted_quantity;
        *self
            .cash_flow
            .entry((trade_side, market))
            .or_insert(Decimal::ZERO) += abs_length_adjusted_quantity * trade_price;

        Ok(())
    }

    fn add_trade_for_report(&mut self, trade: &TradeForReport) -> Result<()> {
        // This and 'add_trade' should not be used at the same time
        if trade.area != self.area {
            bail!("Trade area has to match ReportEntry area");
        }
        let Some(trade_price) = trade.price else {
            return Ok(());
        };

        let trade_side = if trade.quantity_mwh < Decimal::ZERO {
            TradeSide::Sell
        } else {
            TradeSide::Buy
        };
        let market = Market::from(trade.trade_type);
        let contract_length = contract_length(&trade.delivery_start, &trade.delivery_end)?;

        let abs_length_adjusted_quantity = trade.quantity_mwh.abs() * contract_length;

        *self.mw.entry((trade_side, market)).or_insert(Decimal::ZERO) +=
            abs_length_adjusted_quantity;
        *self
            .cash_flow
            .entry((trade_side, market))
            .or_insert(Decimal::ZERO) += abs_length_adjusted_quantity * trade_price;

        Ok(())
    }

    fn revenue(&self, market: MarketSelection) -> Decimal {
        match market {
            MarketSelection::Specific(market) => {
                *self.cash_flow.get(&(TradeSide::Sell, market)).unwrap()
            }
            MarketSelection::All => Market::iter()
                .map(|market| {
                    *self
                        .cash_flow
                        .get(&(TradeSide::Sell, market))
                        .unwrap_or(&Decimal::ZERO)
                })
                .sum(),
        }
    }

    fn costs(&self, market: MarketSelection) -> Decimal {
        match market {
            MarketSelection::Specific(market) => {
                *self.cash_flow.get(&(TradeSide::Buy, market)).unwrap()
            }
            MarketSelection::All => Market::iter()
                .map(|market| {
                    *self
                        .cash_flow
                        .get(&(TradeSide::Buy, market))
                        .unwrap_or(&Decimal::ZERO)
                })
                .sum(),
        }
    }

    fn mw_sold(&self, market: MarketSelection) -> Decimal {
        match market {
            MarketSelection::Specific(market) => *self.mw.get(&(TradeSide::Sell, market)).unwrap(),
            MarketSelection::All => Market::iter()
                .map(|market| {
                    *self
                        .mw
                        .get(&(TradeSide::Sell, market))
                        .unwrap_or(&Decimal::ZERO)
                })
                .sum(),
        }
    }

    fn mw_bought(&self, market: MarketSelection) -> Decimal {
        match market {
            MarketSelection::Specific(market) => *self.mw.get(&(TradeSide::Buy, market)).unwrap(),
            MarketSelection::All => Market::iter()
                .map(|market| {
                    *self
                        .mw
                        .get(&(TradeSide::Buy, market))
                        .unwrap_or(&Decimal::ZERO)
                })
                .sum(),
        }
    }

    fn gross_profit(&self, market: MarketSelection) -> Decimal {
        self.revenue(market) - self.costs(market)
    }
}

fn contract_length(
    delivery_start: &DateTime<FixedOffset>,
    delivery_end: &DateTime<FixedOffset>,
) -> Result<Decimal> {
    let time_delta = *delivery_end - *delivery_start;

    let delta_seconds = Decimal::from_i64(time_delta.num_seconds())
        .ok_or(anyhow!("Could not convert duration to seconds"))?;
    let seconds_per_hour = Decimal::from_str_exact("3600.0")?;

    let contract_length = delta_seconds / seconds_per_hour;

    Ok(contract_length)
}
