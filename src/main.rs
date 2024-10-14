use std::collections::HashMap;
use std::env;
use std::str::FromStr;
use std::time::Instant;

use anyhow::{anyhow, bail, Result};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use strum_macros::{AsRefStr, EnumString};
use time::macros::{date, offset, time};
use time::OffsetDateTime;

#[tokio::main]
async fn main() -> Result<()> {
    // I think I'll have to swap 'time' for 'chrono' as the 'chrono-tz' crate looks very cool for handling timezones
    // more properly, which I'll have to...

    // Load environment variables from .env
    dotenvy::dotenv()?;
    let db_url = env::var("DATABASE_URL")?;

    println!("Initialising sqlx ...");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    let delivery_from =
        OffsetDateTime::new_in_offset(date!(2024 - 10 - 01), time!(22:00:00), offset!(UTC));
    let delivery_to =
        OffsetDateTime::new_in_offset(date!(2024 - 10 - 04), time!(22:00:00), offset!(UTC));

    println!("Getting from db");

    let now = Instant::now();

    let trades = get_trades(&pool, &delivery_from, &delivery_to).await?;

    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);

    println!("Creating report");

    let now = Instant::now();
    let report = Report::new(delivery_from, delivery_to, trades)?;
    println!("Elapsed: {:.2?}", now.elapsed());

    println!("Total revenue: {:#?}", report.revenue(None, None));
    println!("Total costs: {:#?}", report.costs(None, None));
    println!("Total mw sold: {:#?}", report.mw_sold(None, None));
    println!("Total mw bought: {:#?}", report.mw_bought(None, None));
    println!("Total gross profit: {:#?}", report.gross_profit(None, None));

    println!("Done :)");
    Ok(())
}

async fn get_trades(
    pool: &PgPool,
    delivery_from: &OffsetDateTime,
    delivery_to: &OffsetDateTime,
) -> Result<Vec<Trade>> {
    let mut trades = sqlx::query_as!(
        Trade,
        "
    SELECT id, area, counter_part, delivery_start, delivery_end, price, quantity_mwh, trade_side, trade_type
    FROM intraday_trades
    WHERE delivery_start >= $1 AND delivery_start < $2",
        delivery_from,
        delivery_to,
    )
        .fetch_all(pool)
        .await?;

    let auction_trades = sqlx::query_as!(
        Trade,
        "
    SELECT id, area, counter_part, delivery_start, delivery_end, price, quantity_mwh, trade_side, trade_type
    FROM auction_trades
    WHERE delivery_start >= $1 AND delivery_start < $2",
        delivery_from,
        delivery_to,
    )
        .fetch_all(pool)
        .await?;
    trades.extend(auction_trades);

    let imbalance_trades = sqlx::query_as!(
        Trade,
        "
    SELECT id, area, counter_part, delivery_start, delivery_end, price, quantity_mwh, trade_side, trade_type
    FROM imbalance_trades
    WHERE delivery_start >= $1 AND delivery_start < $2",
        delivery_from,
        delivery_to,
    )
        .fetch_all(pool)
        .await?;
    trades.extend(imbalance_trades);

    Ok(trades)
}

#[derive(Debug, Serialize, Deserialize)]
struct Report {
    delivery_from: OffsetDateTime,
    delivery_to: OffsetDateTime,
    areas: HashMap<Area, ReportEntry>,
}

impl Report {
    fn new(
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

    fn revenue(&self, market: Option<Market>, area: Option<Area>) -> Decimal {
        let summed: Decimal;

        if let Some(area) = area {
            if !self.areas.contains_key(&area) {
                return Decimal::ZERO;
            }
            summed = self.areas.get(&area).unwrap().revenue(market);
        } else {
            summed = self.areas.values().map(|f| f.revenue(market)).sum();
        }

        summed
    }

    fn costs(&self, market: Option<Market>, area: Option<Area>) -> Decimal {
        let summed: Decimal;

        if let Some(area) = area {
            if !self.areas.contains_key(&area) {
                return Decimal::ZERO;
            }
            summed = self.areas.get(&area).unwrap().costs(market);
        } else {
            summed = self.areas.values().map(|f| f.costs(market)).sum();
        }

        summed
    }

    fn mw_sold(&self, market: Option<Market>, area: Option<Area>) -> Decimal {
        let summed: Decimal;

        if let Some(area) = area {
            if !self.areas.contains_key(&area) {
                return Decimal::ZERO;
            }
            summed = self.areas.get(&area).unwrap().mw_sold(market);
        } else {
            summed = self.areas.values().map(|f| f.mw_sold(market)).sum();
        }

        summed
    }

    fn mw_bought(&self, market: Option<Market>, area: Option<Area>) -> Decimal {
        let summed: Decimal;

        if let Some(area) = area {
            if !self.areas.contains_key(&area) {
                return Decimal::ZERO;
            }
            summed = self.areas.get(&area).unwrap().mw_bought(market);
        } else {
            summed = self.areas.values().map(|f| f.mw_bought(market)).sum();
        }

        summed
    }

    fn gross_profit(&self, market: Option<Market>, area: Option<Area>) -> Decimal {
        let summed: Decimal;

        if let Some(area) = area {
            if !self.areas.contains_key(&area) {
                return Decimal::ZERO;
            }
            summed = self.areas.get(&area).unwrap().gross_profit(market);
        } else {
            summed = self.areas.values().map(|f| f.gross_profit(market)).sum();
        }

        summed
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

        *self
            .mw
            .entry((trade_side, market))
            .or_insert(Decimal::from_str("0.0")?) += abs_length_adjusted_quantity;
        *self
            .cash_flow
            .entry((trade_side, market))
            .or_insert(Decimal::from_str("0.0")?) += abs_length_adjusted_quantity * trade_price;

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

#[derive(
    Debug,
    Serialize,
    Deserialize,
    EnumString,
    AsRefStr,
    Hash,
    PartialEq,
    PartialOrd,
    Eq,
    Clone,
    Copy,
)]
#[strum(serialize_all = "UPPERCASE")]
enum Area {
    AMP,
    DK1,
    DK2,
    FR,
    GB,
    NL,
    NO2,
    SE1,
    SE3,
}

impl From<String> for Area {
    fn from(item: String) -> Self {
        Area::from_str(&item).unwrap_or_else(|_| panic!("Invalid area: {}", item))
    }
}

#[derive(Debug, Serialize, Deserialize, EnumString, AsRefStr)]
#[strum(serialize_all = "lowercase")]
enum CounterPart {
    Nordpool,
    Epex,
    Esett,
    Elexon,
    RTE,
    SEMO,
    Tennet,
    Amprion,
}

impl From<String> for CounterPart {
    fn from(item: String) -> Self {
        CounterPart::from_str(&item).unwrap_or_else(|_| panic!("Invalid counter part: {}", item))
    }
}

#[derive(
    Debug,
    Serialize,
    Deserialize,
    EnumString,
    AsRefStr,
    Hash,
    PartialEq,
    PartialOrd,
    Eq,
    Copy,
    Clone,
)]
#[strum(serialize_all = "lowercase")]
enum TradeSide {
    Buy,
    Sell,
}

impl From<String> for TradeSide {
    fn from(item: String) -> Self {
        TradeSide::from_str(&item).unwrap_or_else(|_| panic!("Invalid trade side: {}", item))
    }
}

#[derive(Debug, Serialize, Deserialize, EnumString, AsRefStr, Clone, Copy)]
#[strum(serialize_all = "snake_case")]
enum TradeType {
    Intraday,
    Imbalance,
    AuctionGbDahH,
    AuctionGbDahHh,
    AuctionGbId1Hh,
    AuctionGbId2Hh,
    AuctionEurDahH,
    AuctionEurId1H,
    AuctionEurId2H,
    AuctionEurId3H,
}

#[derive(
    Debug,
    Serialize,
    Deserialize,
    EnumString,
    AsRefStr,
    Hash,
    PartialEq,
    PartialOrd,
    Eq,
    Clone,
    Copy,
)]
#[strum(serialize_all = "lowercase")]
enum Market {
    Auction,
    Intraday,
    Imbalance,
}

impl From<TradeType> for Market {
    fn from(value: TradeType) -> Self {
        return match value {
            TradeType::Intraday => Self::Intraday,
            TradeType::Imbalance => Self::Imbalance,
            TradeType::AuctionGbDahH => Self::Auction,
            TradeType::AuctionGbDahHh => Self::Auction,
            TradeType::AuctionGbId1Hh => Self::Auction,
            TradeType::AuctionGbId2Hh => Self::Auction,
            TradeType::AuctionEurDahH => Self::Auction,
            TradeType::AuctionEurId1H => Self::Auction,
            TradeType::AuctionEurId2H => Self::Auction,
            TradeType::AuctionEurId3H => Self::Auction,
        };
    }
}

impl From<String> for TradeType {
    fn from(item: String) -> Self {
        TradeType::from_str(&item).unwrap_or_else(|_| panic!("Invalid trade type: {}", item))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Trade {
    id: i32,
    area: Area,
    counter_part: CounterPart,
    delivery_end: OffsetDateTime,
    delivery_start: OffsetDateTime,
    price: Option<Decimal>,
    quantity_mwh: Decimal,
    #[serde(rename = "side")]
    trade_side: TradeSide,
    #[serde(rename = "type")]
    trade_type: TradeType,
}
