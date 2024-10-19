use std::str::FromStr;

use chrono::{DateTime, FixedOffset};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};

#[derive(
    Debug, Serialize, Deserialize, EnumString, Hash, PartialEq, PartialOrd, Eq, Clone, Copy,
)]
#[strum(serialize_all = "UPPERCASE")]
pub enum Area {
    Amp,
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum AreaSelection {
    All,
    Specific(Area),
}

#[derive(Debug, Serialize, Deserialize, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum CounterPart {
    Nordpool,
    Epex,
    Esett,
    Elexon,
    Rte,
    Semo,
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
    Debug,
    Serialize,
    Deserialize,
    EnumString,
    EnumIter,
    Hash,
    PartialEq,
    PartialOrd,
    Eq,
    Clone,
    Copy,
)]
#[strum(serialize_all = "lowercase")]
pub enum Market {
    Auction,
    Intraday,
    Imbalance,
}

impl From<TradeType> for Market {
    fn from(value: TradeType) -> Self {
        match value {
            TradeType::Intraday => Self::Intraday,
            TradeType::Imbalance => Self::Imbalance,
            TradeType::AuctionGbDahH
            | TradeType::AuctionGbDahHh
            | TradeType::AuctionGbId1Hh
            | TradeType::AuctionGbId2Hh
            | TradeType::AuctionEurDahH
            | TradeType::AuctionEurId1H
            | TradeType::AuctionEurId2H
            | TradeType::AuctionEurId3H => Self::Auction,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum MarketSelection {
    All,
    Specific(Market),
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
    pub delivery_end: DateTime<FixedOffset>,
    pub delivery_start: DateTime<FixedOffset>,
    pub price: Option<Decimal>,
    pub quantity_mwh: Decimal,
    #[serde(rename = "side")]
    pub trade_side: TradeSide,
    #[serde(rename = "type")]
    pub trade_type: TradeType,
}
