use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::trade::{Area, Market, Trade, TradeSide};

#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    delivery_from: OffsetDateTime,
    delivery_to: OffsetDateTime,
    areas: HashMap<Area, ReportEntry>,
}

impl Report {
    pub fn new(
        delivery_from: OffsetDateTime,
        delivery_to: OffsetDateTime,
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
            delivery_from,
            delivery_to,
            areas,
        };

        Ok(report)
    }

    pub fn revenue(&self, market: Option<Market>, area: Option<Area>) -> Decimal {
        let summed: Decimal;

        if let Some(area) = area {
            if !self.areas.contains_key(&area) {
                return Decimal::ZERO;
            }
            summed = self.areas.get(&area).unwrap().revenue(market);
        } else {
            summed = self.areas.values().map(|f| f.revenue(market)).sum();
        }

        summed.round_dp(2)
    }

    pub fn costs(&self, market: Option<Market>, area: Option<Area>) -> Decimal {
        let summed: Decimal;

        if let Some(area) = area {
            if !self.areas.contains_key(&area) {
                return Decimal::ZERO;
            }
            summed = self.areas.get(&area).unwrap().costs(market);
        } else {
            summed = self.areas.values().map(|f| f.costs(market)).sum();
        }

        summed.round_dp(2)
    }

    pub fn mw_sold(&self, market: Option<Market>, area: Option<Area>) -> Decimal {
        let summed: Decimal;

        if let Some(area) = area {
            if !self.areas.contains_key(&area) {
                return Decimal::ZERO;
            }
            summed = self.areas.get(&area).unwrap().mw_sold(market);
        } else {
            summed = self.areas.values().map(|f| f.mw_sold(market)).sum();
        }

        summed.round_dp(1)
    }

    pub fn mw_bought(&self, market: Option<Market>, area: Option<Area>) -> Decimal {
        let summed: Decimal;

        if let Some(area) = area {
            if !self.areas.contains_key(&area) {
                return Decimal::ZERO;
            }
            summed = self.areas.get(&area).unwrap().mw_bought(market);
        } else {
            summed = self.areas.values().map(|f| f.mw_bought(market)).sum();
        }

        summed.round_dp(1)
    }

    pub fn gross_profit(&self, market: Option<Market>, area: Option<Area>) -> Decimal {
        let summed: Decimal;

        if let Some(area) = area {
            if !self.areas.contains_key(&area) {
                return Decimal::ZERO;
            }
            summed = self.areas.get(&area).unwrap().gross_profit(market);
        } else {
            summed = self.areas.values().map(|f| f.gross_profit(market)).sum();
        }

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
    delivery_start: &OffsetDateTime,
    delivery_end: &OffsetDateTime,
) -> Result<Decimal> {
    // Todo: This probably won't work when summer/winter changes over the duration :sad-panda:
    // This has to be fixed.
    let duration = *delivery_end - *delivery_start;

    let delta_seconds = Decimal::from_i64(duration.whole_seconds())
        .ok_or(anyhow!("Could not convert duration to seconds"))?;
    let seconds_per_hour = Decimal::from_str_exact("3600.0")?;

    let contract_length = delta_seconds / seconds_per_hour;

    Ok(contract_length)
}
