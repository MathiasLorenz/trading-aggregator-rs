use futures::TryStreamExt;
use std::env;
use std::sync::Arc;
use std::time::Instant;

mod db;
mod report;
mod trade;

use anyhow::Result;
use chrono::prelude::*;
use chrono_tz::{Europe::Copenhagen, Tz};
use db::{
    get_auction_trades_stream, get_imbalance_trades_stream, get_intraday_trades_stream, get_trades,
    get_trades_for_report, get_trades_stream, init_db_pool,
};
use report::Report;
use sqlx::PgPool;
use tokio::{sync::mpsc, task};
use trade::Trade;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().expect("Could not load .env");
    let db_url = env::var("DATABASE_URL")?;

    println!("Initialising sqlx ...");

    let pool = init_db_pool(&db_url).await?;

    let delivery_from = NaiveDate::from_ymd_opt(2024, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let delivery_from = Copenhagen.from_local_datetime(&delivery_from).unwrap();

    let delivery_to = NaiveDate::from_ymd_opt(2024, 11, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let delivery_to = Copenhagen.from_local_datetime(&delivery_to).unwrap();

    println!("Create report, standard");
    let now = Instant::now();
    let report = create_report(&pool, delivery_from, delivery_to).await?;
    report.print_key_metrics();
    println!("Generating report, standard, took: {:.2?}", now.elapsed());
    println!();

    println!("Create report, simple trade structure (TradeForReport)");
    let now = Instant::now();
    let report = create_report_from_simple_trade(&pool, delivery_from, delivery_to).await?;
    report.print_key_metrics();
    println!("Generating report, standard, took: {:.2?}", now.elapsed());
    println!();

    let now = Instant::now();
    println!("Create report, stream");
    let report = create_report_stream(&pool, delivery_from, delivery_to).await?;
    report.print_key_metrics();
    println!("Generating report, stream, took: {:.2?}", now.elapsed());
    println!();

    let now = Instant::now();
    println!("Create report, channels -> Vec<Trace> -> Report::new(trades)");
    // As we're creating threads for each trade type, we need to use an Arc to share the PgPool reference
    let arc_pool = Arc::new(pool);
    let report = create_report_channels(arc_pool, delivery_from, delivery_to).await?;
    report.print_key_metrics();
    println!("Generating report, stream, took: {:.2?}", now.elapsed());
    println!();

    println!("Done :)");
    Ok(())
}

async fn create_report(
    pool: &PgPool,
    delivery_from: DateTime<Tz>,
    delivery_to: DateTime<Tz>,
) -> Result<Report> {
    println!("Getting from db");

    let now = Instant::now();
    let trades = get_trades(pool, &delivery_from, &delivery_to).await?;
    let elapsed = now.elapsed();
    println!("Getting trades took: {:.2?}", elapsed);

    let now = Instant::now();
    // In an async-sense, this is a compute heavy task, so we spawn it in a blocking thread
    let report =
        task::spawn_blocking(move || Report::new(&delivery_from, &delivery_to, trades)).await??;
    println!("Report part took: {:.2?}", now.elapsed());

    Ok(report)
}

async fn create_report_from_simple_trade(
    pool: &PgPool,
    delivery_from: DateTime<Tz>,
    delivery_to: DateTime<Tz>,
) -> Result<Report> {
    println!("Getting from db");

    let now = Instant::now();
    let trades_for_report = get_trades_for_report(pool, &delivery_from, &delivery_to).await?;
    let elapsed = now.elapsed();
    println!("Getting trades took: {:.2?}", elapsed);

    let now = Instant::now();
    // In an async-sense, this is a compute heavy task, so we spawn it in a blocking thread
    let report = task::spawn_blocking(move || {
        Report::new_from_trade_for_report(&delivery_from, &delivery_to, trades_for_report)
    })
    .await??;
    println!("Report part took: {:.2?}", now.elapsed());

    Ok(report)
}

async fn create_report_stream(
    pool: &PgPool,
    delivery_from: DateTime<Tz>,
    delivery_to: DateTime<Tz>,
) -> Result<Report> {
    let trades_stream = get_trades_stream(pool, &delivery_from, &delivery_to);

    let now = Instant::now();
    let report = Report::new_from_stream(&delivery_from, &delivery_to, trades_stream).await?;
    println!("Creating report, stream, took: {:.2?}", now.elapsed());

    Ok(report)
}

async fn create_report_channels(
    pool: Arc<PgPool>,
    delivery_from: DateTime<Tz>,
    delivery_to: DateTime<Tz>,
) -> Result<Report> {
    // This is pretty slow as we have to get all trades (send them over the channels as well)
    // and then collect them into a vector.
    // It should be pretty doable to create a stream directly from the channels, with something like
    // https://docs.rs/tokio/latest/tokio/stream/index.html or
    // https://docs.rs/tokio-stream/latest/tokio_stream/
    // The next version should be a Channels -> Stream<Trade> -> Report
    // Then one should be able to create a Channels -> Stream<(quantity_mw, cash_flow)> -> Report to send
    // as little data over the wire as possible.

    let now = Instant::now();

    let (tx, mut rx) = mpsc::channel(100);

    let intraday_tx = tx.clone();
    let pool_cloned = Arc::clone(&pool);
    tokio::spawn(async move {
        let mut stream = get_intraday_trades_stream(&pool_cloned, &delivery_from, &delivery_to);
        while let Some(trade) = stream.try_next().await.unwrap() {
            intraday_tx.send(trade).await.unwrap();
        }
    });

    let auction_tx = tx.clone();
    let pool_cloned = Arc::clone(&pool);
    tokio::spawn(async move {
        let mut stream = get_auction_trades_stream(&pool_cloned, &delivery_from, &delivery_to);
        while let Some(trade) = stream.try_next().await.unwrap() {
            auction_tx.send(trade).await.unwrap();
        }
    });

    let imbalance_tx = tx.clone();
    let pool_cloned = Arc::clone(&pool);
    tokio::spawn(async move {
        let mut stream = get_imbalance_trades_stream(&pool_cloned, &delivery_from, &delivery_to);
        while let Some(trade) = stream.try_next().await.unwrap() {
            imbalance_tx.send(trade).await.unwrap();
        }
    });

    // The `rx` half of the channel returns `None` once **all** `tx` clones
    // drop. To ensure `None` is returned, drop the handle owned by the
    // current task. If this `tx` handle is not dropped, there will always
    // be a single outstanding `tx` handle.
    drop(tx);

    println!("Creating channels took: {:.2?}", now.elapsed());

    let now = Instant::now();
    let mut trades: Vec<Trade> = Vec::new();
    while let Some(trade) = rx.recv().await {
        trades.push(trade);
    }
    println!("Getting trades took: {:.2?}", now.elapsed());

    let now = Instant::now();
    let report = Report::new(&delivery_from, &delivery_to, trades)?;
    println!("Creating report, Vec<Trade>, took: {:.2?}", now.elapsed());

    Ok(report)
}
