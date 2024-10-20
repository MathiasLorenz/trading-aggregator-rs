use std::pin::Pin;

use chrono::DateTime;
use chrono_tz::Tz;
use futures::{Stream, StreamExt};
use sqlx::{postgres::PgPoolOptions, Error, PgPool};

use crate::trade::{Trade, TradeForReport};
use anyhow::{Context, Result};

pub async fn init_db_pool(db_url: &str) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await
        .context("Failed to create database pool")
}

pub async fn get_trades(
    pool: &PgPool,
    delivery_from: &DateTime<Tz>,
    delivery_to: &DateTime<Tz>,
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

pub async fn get_trades_for_report(
    pool: &PgPool,
    delivery_from: &DateTime<Tz>,
    delivery_to: &DateTime<Tz>,
) -> Result<Vec<TradeForReport>> {
    let mut trades = sqlx::query_as!(
        TradeForReport,
        "
    SELECT area, delivery_start, delivery_end, price, quantity_mwh, trade_type
    FROM intraday_trades
    WHERE delivery_start >= $1 AND delivery_start < $2",
        delivery_from,
        delivery_to,
    )
    .fetch_all(pool)
    .await?;

    let auction_trades = sqlx::query_as!(
        TradeForReport,
        "
    SELECT area, delivery_start, delivery_end, price, quantity_mwh, trade_type
    FROM auction_trades
    WHERE delivery_start >= $1 AND delivery_start < $2",
        delivery_from,
        delivery_to,
    )
    .fetch_all(pool)
    .await?;
    trades.extend(auction_trades);

    let imbalance_trades = sqlx::query_as!(
        TradeForReport,
        "
    SELECT area, delivery_start, delivery_end, price, quantity_mwh, trade_type
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

pub fn get_trades_stream<'a>(
    pool: &'a PgPool,
    delivery_from: &'a DateTime<Tz>,
    delivery_to: &'a DateTime<Tz>,
) -> Pin<Box<dyn Stream<Item = Result<Trade, Error>> + Send + 'a>> {
    let intraday_trades = sqlx::query_as!(
        Trade,
        "
    SELECT id, area, counter_part, delivery_start, delivery_end, price, quantity_mwh, trade_side, trade_type
    FROM intraday_trades
    WHERE delivery_start >= $1 AND delivery_start < $2",
        delivery_from,
        delivery_to,
    )
        .fetch(pool);

    let auction_trades = sqlx::query_as!(
        Trade,
        "
    SELECT id, area, counter_part, delivery_start, delivery_end, price, quantity_mwh, trade_side, trade_type
    FROM auction_trades
    WHERE delivery_start >= $1 AND delivery_start < $2",
        delivery_from,
        delivery_to,
    )
        .fetch(pool);

    let imbalance_trades = sqlx::query_as!(
        Trade,
        "
    SELECT id, area, counter_part, delivery_start, delivery_end, price, quantity_mwh, trade_side, trade_type
    FROM imbalance_trades
    WHERE delivery_start >= $1 AND delivery_start < $2",
        delivery_from,
        delivery_to,
    )
        .fetch(pool);

    Box::pin(
        intraday_trades
            .chain(auction_trades)
            .chain(imbalance_trades),
    )
}
