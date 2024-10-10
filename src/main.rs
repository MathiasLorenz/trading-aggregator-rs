use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::macros::{date, offset, time};
use time::{Duration, OffsetDateTime};

fn main() {
    let area = Area::DK1;
    let counter_part = CounterPart::Nordpool;
    let delivery_start =
        OffsetDateTime::new_in_offset(date!(2024 - 09 - 02), time!(00:00:00), offset!(UTC));
    let delivery_end = delivery_start + Duration::DAY;
    let price = Decimal::from_str("242.2").unwrap();
    let quantity_mw = Decimal::from_str("23.1").unwrap();
    let trade_side = TradeSide::Buy;
    let trade_type = TradeType::Intraday;
    let currency = Currency::Eur;
    let label = Some("sup".to_string());
    let execution_time = Some(OffsetDateTime::new_in_offset(
        date!(2024 - 09 - 01),
        time!(20:00:00),
        offset!(UTC),
    ));

    let trade = Trade {
        area,
        counter_part,
        delivery_end,
        delivery_start,
        price,
        quantity_mw,
        trade_side,
        trade_type,
        currency,
        label,
        execution_time,
    };

    println!("My trade is: {:?}", trade)
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
enum TradeSide {
    Buy,
    Sell,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Serialize, Deserialize)]
enum Currency {
    Eur,
    Gbp,
}

#[derive(Debug, Serialize, Deserialize)]
struct Trade {
    area: Area,
    counter_part: CounterPart,
    delivery_end: OffsetDateTime,
    delivery_start: OffsetDateTime,
    price: Decimal,
    #[serde(rename = "quantity_mwh")]
    quantity_mw: Decimal,
    #[serde(rename = "side")]
    trade_side: TradeSide,
    #[serde(rename = "type")]
    trade_type: TradeType,
    currency: Currency,
    label: Option<String>,
    execution_time: Option<OffsetDateTime>,
}
