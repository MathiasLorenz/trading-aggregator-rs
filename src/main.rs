use std::env;
use std::time::Instant;

mod db;
mod report;
mod trade;

use anyhow::Result;
use chrono::prelude::*;
use chrono_tz::{Europe::Copenhagen, Tz};
use db::{get_trades, get_trades_for_report, get_trades_stream, init_db_pool};
use report::Report;
use sqlx::PgPool;
use tokio::task;

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
