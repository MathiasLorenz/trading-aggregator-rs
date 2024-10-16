use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use strum_macros::EnumString;
use time::OffsetDateTime;

#[derive(
    Debug, Serialize, Deserialize, EnumString, Hash, PartialEq, PartialOrd, Eq, Clone, Copy,
)]
#[strum(serialize_all = "UPPERCASE")]
pub enum Area {
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

#[derive(Debug, Serialize, Deserialize, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum CounterPart {
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
    Debug, Serialize, Deserialize, EnumString, Hash, PartialEq, PartialOrd, Eq, Copy, Clone,
)]
#[strum(serialize_all = "lowercase")]
pub enum TradeSide {
    Buy,
    Sell,
}

impl From<String> for TradeSide {
    fn from(item: String) -> Self {
        TradeSide::from_str(&item).unwrap_or_else(|_| panic!("Invalid trade side: {}", item))
    }
}

#[derive(Debug, Serialize, Deserialize, EnumString, Clone, Copy)]
#[strum(serialize_all = "snake_case")]
pub enum TradeType {
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
    Debug, Serialize, Deserialize, EnumString, Hash, PartialEq, PartialOrd, Eq, Clone, Copy,
)]
#[strum(serialize_all = "lowercase")]
pub enum Market {
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
pub struct Trade {
    pub id: i32,
    pub area: Area,
    pub counter_part: CounterPart,
    pub delivery_end: OffsetDateTime,
    pub delivery_start: OffsetDateTime,
    pub price: Option<Decimal>,
    pub quantity_mwh: Decimal,
    #[serde(rename = "side")]
    pub trade_side: TradeSide,
    #[serde(rename = "type")]
    pub trade_type: TradeType,
}
