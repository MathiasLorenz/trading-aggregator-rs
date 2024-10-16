use sqlx::{postgres::PgPoolOptions, PgPool};
use time::OffsetDateTime;

use crate::trade::Trade;
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
