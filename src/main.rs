use std::env;
use std::str::FromStr;

use anyhow::Result;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
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
    let price = Decimal::from_str("242.2").unwrap();
    let quantity_mwh = Decimal::from_str("23.1").unwrap();
    let trade_side = TradeSide::Buy;
    let trade_type = TradeType::Intraday;
    // let currency = Currency::Eur;
    let contract_id = "23".to_string();
    let portfolio_id = "3434".to_string();
    let trade_id = "df".to_string();
    let order_id = Some("some id".to_string());
    let insertion_time = Some(OffsetDateTime::new_in_offset(
        date!(2024 - 09 - 02),
        time!(20:00:01),
        offset!(UTC),
    ));
    let label = Some("sup".to_string());
    let execution_time = Some(OffsetDateTime::new_in_offset(
        date!(2024 - 09 - 02),
        time!(20:00:01),
        offset!(UTC),
    ));

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
        // currency,
        contract_id,
        portfolio_id,
        trade_id,
        order_id,
        label,
        execution_time,
        insertion_time,
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

    let db_trade = sqlx::query_as!(
        Trade,
        "
    SELECT *
    FROM intraday_trades"
    )
    .fetch_one(&pool)
    .await?;

    // let conn = pool.acquire().await?;
    // let mut stream =
    //     sqlx::query_as::<_, Trade>("SELECT * FROM trades LIMIT 1").fetch_one(&mut conn);

    println!("My trade from the database is: {:?}", db_trade);

    println!("Done :)");
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, EnumString, AsRefStr)]
#[strum(serialize_all = "UPPERCASE")] // Optional: Define how the strings should be serialized
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

#[derive(Debug, Serialize, Deserialize, EnumString, AsRefStr)]
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

#[derive(Debug, Serialize, Deserialize, EnumString, AsRefStr)]
#[strum(serialize_all = "lowercase")]
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

impl From<String> for TradeType {
    fn from(item: String) -> Self {
        TradeType::from_str(&item).unwrap_or_else(|_| panic!("Invalid trade type: {}", item))
    }
}

// #[derive(Debug, Serialize, Deserialize)]
// enum Currency {
//     Eur,
//     Gbp,
// }

#[derive(Debug, Serialize, Deserialize)]
struct Trade {
    id: i32,
    area: Area,
    counter_part: CounterPart,
    delivery_end: OffsetDateTime,
    delivery_start: OffsetDateTime,
    price: Decimal,
    quantity_mwh: Decimal,
    #[serde(rename = "side")]
    trade_side: TradeSide,
    #[serde(rename = "type")]
    trade_type: TradeType,
    // currency: Currency,
    contract_id: String,
    portfolio_id: String,
    trade_id: String,
    order_id: Option<String>,
    label: Option<String>,
    execution_time: Option<OffsetDateTime>,
    insertion_time: Option<OffsetDateTime>,
}
