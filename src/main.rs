use std::collections::HashMap;
use std::env;
use std::str::FromStr;
use std::time::Instant;

use anyhow::{bail, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use strum_macros::{AsRefStr, EnumString};
use time::macros::{date, offset, time};
use time::{Duration, OffsetDateTime};

#[tokio::main]
async fn main() -> Result<()> {
    // I think I'll have to swap 'time' for 'chrono' as the 'chrono-tz' crate looks very cool for handling timezones
    // more properly, which I'll have to...

    let id = 2;
    let area = Area::DK1;
    let counter_part = CounterPart::Nordpool;
    let delivery_start =
        OffsetDateTime::new_in_offset(date!(2024 - 09 - 02), time!(00:00:00), offset!(UTC));
    let delivery_end = delivery_start + Duration::DAY;
    let price = Some(Decimal::from_str("242.2").unwrap());
    let quantity_mwh = Decimal::from_str("23.1").unwrap();
    let trade_side = TradeSide::Buy;
    let trade_type = TradeType::Intraday;

    let trade = Trade {
        id,
        area,
        counter_part,
        delivery_end,
        delivery_start,
        price,
        quantity_mwh,
        trade_side,
        trade_type,
    };

    println!("My trade is: {:?}", trade);

    // Load environment variables from .env
    dotenvy::dotenv()?;
    let db_url = env::var("DATABASE_URL")?;

    println!("Initialising sqlx ...");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    println!("Getting from db");

    let now = Instant::now();

    let trades = get_trades(&pool).await?;

    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);

    println!("Number of trades from the database is: {:?}", trades.len());

    println!("Done :)");
    Ok(())
}

async fn get_trades(pool: &PgPool) -> Result<Vec<Trade>> {
    let mut trades = sqlx::query_as!(
        Trade,
        "
    SELECT id, area, counter_part, delivery_start, delivery_end, price, quantity_mwh, trade_side, trade_type
    FROM intraday_trades"
    )
        .fetch_all(pool)
        .await?;

    let auction_trades = sqlx::query_as!(
        Trade,
        "
    SELECT id, area, counter_part, delivery_start, delivery_end, price, quantity_mwh, trade_side, trade_type
    FROM auction_trades"
    )
        .fetch_all(pool)
        .await?;
    trades.extend(auction_trades);

    let imbalance_trades = sqlx::query_as!(
        Trade,
        "
    SELECT id, area, counter_part, delivery_start, delivery_end, price, quantity_mwh, trade_side, trade_type
    FROM imbalance_trades"
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
        if delivery_to > delivery_from {
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
        if trade.price.is_none() {
            return Ok(());
        }

        let trade_side = trade.trade_side;
        let market = Market::from(trade.trade_type);
        let contract_length = Decimal::from_str("1.0")?; // Todo, fix

        let abs_length_adjusted_quantity = trade.quantity_mwh.abs() * contract_length;

        *self
            .mw
            .entry((trade_side, market))
            .or_insert(Decimal::from_str("0.0").unwrap()) += abs_length_adjusted_quantity;
        *self
            .cash_flow
            .entry((trade_side, market))
            .or_insert(Decimal::from_str("0.0").unwrap()) +=
            abs_length_adjusted_quantity * trade.price.unwrap();

        Ok(())
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
