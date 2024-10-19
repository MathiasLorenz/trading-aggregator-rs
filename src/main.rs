use std::env;
use std::time::Instant;

mod db;
mod report;
mod trade;

use anyhow::Result;
use chrono::prelude::*;
use chrono_tz::{Europe::Copenhagen, Tz};
use db::{get_trades, get_trades_stream, init_db_pool};
use report::Report;
use sqlx::PgPool;
use tokio::task;
use trade::{AreaSelection, MarketSelection};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;
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
    create_report(&pool, delivery_from, delivery_to).await?;
    println!("Generating report, standard, took: {:.2?}", now.elapsed());
    println!();

    let now = Instant::now();
    println!("Create report, stream");
    create_report_stream(&pool, delivery_from, delivery_to).await?;
    println!("Generating report, stream, took: {:.2?}", now.elapsed());
    println!();

    println!("Done :)");
    Ok(())
}

async fn create_report(
    pool: &PgPool,
    delivery_from: DateTime<Tz>,
    delivery_to: DateTime<Tz>,
) -> Result<()> {
    println!("Delivery from: {:#?}", delivery_from);
    println!("Delivery to: {:#?}", delivery_to);

    println!("Getting from db");

    let now = Instant::now();
    let trades = get_trades(pool, &delivery_from, &delivery_to).await?;
    let elapsed = now.elapsed();
    println!("Getting trades took: {:.2?}", elapsed);

    let now = Instant::now();
    let report =
        task::spawn_blocking(move || Report::new(&delivery_from, &delivery_to, trades)).await??;
    println!("Report part took: {:.2?}", now.elapsed());

    println!(
        "Total gross profit: {:?}",
        report.gross_profit(MarketSelection::All, AreaSelection::All)
    );
    println!(
        "Total revenue: {:?}",
        report.revenue(MarketSelection::All, AreaSelection::All)
    );
    println!(
        "Total costs: {:?}",
        report.costs(MarketSelection::All, AreaSelection::All)
    );
    println!(
        "Total mw sold: {:?}",
        report.mw_sold(MarketSelection::All, AreaSelection::All)
    );
    println!(
        "Total mw bought: {:?}",
        report.mw_bought(MarketSelection::All, AreaSelection::All)
    );

    Ok(())
}

async fn create_report_stream(
    pool: &PgPool,
    delivery_from: DateTime<Tz>,
    delivery_to: DateTime<Tz>,
) -> Result<()> {
    println!("Delivery from: {:#?}", delivery_from);
    println!("Delivery to: {:#?}", delivery_to);

    let trades_stream = get_trades_stream(pool, &delivery_from, &delivery_to);

    let now = Instant::now();
    let report = Report::new_from_stream(&delivery_from, &delivery_to, trades_stream).await?;
    println!("Creating report, stream, took: {:.2?}", now.elapsed());

    println!(
        "Total gross profit: {:?}",
        report.gross_profit(MarketSelection::All, AreaSelection::All)
    );
    println!(
        "Total revenue: {:?}",
        report.revenue(MarketSelection::All, AreaSelection::All)
    );
    println!(
        "Total costs: {:?}",
        report.costs(MarketSelection::All, AreaSelection::All)
    );
    println!(
        "Total mw sold: {:?}",
        report.mw_sold(MarketSelection::All, AreaSelection::All)
    );
    println!(
        "Total mw bought: {:?}",
        report.mw_bought(MarketSelection::All, AreaSelection::All)
    );

    Ok(())
}
