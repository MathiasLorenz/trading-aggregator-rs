use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, FixedOffset};
use chrono_tz::Tz;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use serde::{Deserialize, Serialize};

use crate::trade::{Area, Market, Trade, TradeSide};

#[derive(Debug)]
pub struct Report {
    _delivery_from: DateTime<Tz>,
    _delivery_to: DateTime<Tz>,
    areas: HashMap<Area, ReportEntry>,
}

impl Report {
    pub fn new(
        delivery_from: DateTime<Tz>,
        delivery_to: DateTime<Tz>,
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
            _delivery_from: delivery_from,
            _delivery_to: delivery_to,
            areas,
        };

        Ok(report)
    }

    fn aggregate_metric<F>(
        &self,
        market: Option<Market>,
        area: Option<Area>,
        aggregator: F,
    ) -> Decimal
    where
        F: Fn(&ReportEntry, Option<Market>) -> Decimal,
    {
        if let Some(area) = area {
            self.areas
                .get(&area)
                .map_or(Decimal::ZERO, |entry| aggregator(entry, market))
        } else {
            self.areas
                .values()
                .map(|entry| aggregator(entry, market))
                .sum()
        }
    }

    pub fn revenue(&self, market: Option<Market>, area: Option<Area>) -> Decimal {
        let summed = self.aggregate_metric(market, area, |entry, market| entry.revenue(market));
        summed.round_dp(2)
    }

    pub fn costs(&self, market: Option<Market>, area: Option<Area>) -> Decimal {
        let summed = self.aggregate_metric(market, area, |entry, market| entry.costs(market));
        summed.round_dp(2)
    }

    pub fn mw_sold(&self, market: Option<Market>, area: Option<Area>) -> Decimal {
        let summed = self.aggregate_metric(market, area, |entry, market| entry.mw_sold(market));
        summed.round_dp(1)
    }

    pub fn mw_bought(&self, market: Option<Market>, area: Option<Area>) -> Decimal {
        let summed = self.aggregate_metric(market, area, |entry, market| entry.mw_bought(market));
        summed.round_dp(1)
    }

    pub fn gross_profit(&self, market: Option<Market>, area: Option<Area>) -> Decimal {
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

    fn revenue(&self, market: Option<Market>) -> Decimal {
        if let Some(market) = market {
            return *self.cash_flow.get(&(TradeSide::Sell, market)).unwrap();
        }

        *self
            .cash_flow
            .get(&(TradeSide::Sell, Market::Auction))
            .unwrap_or(&Decimal::ZERO)
            + *self
                .cash_flow
                .get(&(TradeSide::Sell, Market::Imbalance))
                .unwrap_or(&Decimal::ZERO)
            + *self
                .cash_flow
                .get(&(TradeSide::Sell, Market::Intraday))
                .unwrap_or(&Decimal::ZERO)
    }

    fn costs(&self, market: Option<Market>) -> Decimal {
        if let Some(market) = market {
            return *self.cash_flow.get(&(TradeSide::Buy, market)).unwrap();
        }

        *self
            .cash_flow
            .get(&(TradeSide::Buy, Market::Auction))
            .unwrap_or(&Decimal::ZERO)
            + *self
                .cash_flow
                .get(&(TradeSide::Buy, Market::Imbalance))
                .unwrap_or(&Decimal::ZERO)
            + *self
                .cash_flow
                .get(&(TradeSide::Buy, Market::Intraday))
                .unwrap_or(&Decimal::ZERO)
    }

    fn mw_sold(&self, market: Option<Market>) -> Decimal {
        if let Some(market) = market {
            return *self.mw.get(&(TradeSide::Sell, market)).unwrap();
        }

        *self
            .mw
            .get(&(TradeSide::Sell, Market::Auction))
            .unwrap_or(&Decimal::ZERO)
            + *self
                .mw
                .get(&(TradeSide::Sell, Market::Imbalance))
                .unwrap_or(&Decimal::ZERO)
            + *self
                .mw
                .get(&(TradeSide::Sell, Market::Intraday))
                .unwrap_or(&Decimal::ZERO)
    }

    fn mw_bought(&self, market: Option<Market>) -> Decimal {
        if let Some(market) = market {
            return *self.mw.get(&(TradeSide::Buy, market)).unwrap();
        }

        *self
            .mw
            .get(&(TradeSide::Buy, Market::Auction))
            .unwrap_or(&Decimal::ZERO)
            + *self
                .mw
                .get(&(TradeSide::Buy, Market::Imbalance))
                .unwrap_or(&Decimal::ZERO)
            + *self
                .mw
                .get(&(TradeSide::Buy, Market::Intraday))
                .unwrap_or(&Decimal::ZERO)
    }

    fn gross_profit(&self, market: Option<Market>) -> Decimal {
        self.revenue(market) - self.costs(market)
    }
}

fn contract_length(
    delivery_start: &DateTime<FixedOffset>,
    delivery_end: &DateTime<FixedOffset>,
) -> Result<Decimal> {
    // Todo: This probably won't work when summer/winter changes over the duration :sad-panda:
    // This has to be fixed.
    let time_delta = *delivery_end - *delivery_start;

    let delta_seconds = Decimal::from_i64(time_delta.num_seconds())
        .ok_or(anyhow!("Could not convert duration to seconds"))?;
    let seconds_per_hour = Decimal::from_str_exact("3600.0")?;

    let contract_length = delta_seconds / seconds_per_hour;

    Ok(contract_length)
}
