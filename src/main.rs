use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

fn main() {
    println!("Hello, world!");
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

struct Trade {
    area: Area,
    counter_part: CounterPart,
    delivery_end: OffsetDateTime,
    delivery_start: OffsetDateTime,
    price: Decimal,
    quantity_mw: Decimal,
    trade_side: TradeSide, // obs, name changed from 'side'
    trade_type: TradeType, // obs, name changed from 'type' as this clashes with built in name
    currency: Currency,
    label: Option<String>,
    execution_time: Option<OffsetDateTime>,
}
