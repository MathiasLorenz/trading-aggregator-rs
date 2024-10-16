use std::env;
use std::time::Instant;

mod db;
mod report;
mod trade;

use anyhow::Result;
use chrono::prelude::*;
use chrono_tz::Europe::Copenhagen;
use db::{get_trades, init_db_pool};
use report::Report;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env
    dotenvy::dotenv()?;
    let db_url = env::var("DATABASE_URL")?;

    println!("Initialising sqlx ...");

    let pool = init_db_pool(&db_url).await?;

    let delivery_from = NaiveDate::from_ymd_opt(2023, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let delivery_from = Copenhagen.from_local_datetime(&delivery_from).unwrap();

    let delivery_to = NaiveDate::from_ymd_opt(2024, 11, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let delivery_to = Copenhagen.from_local_datetime(&delivery_to).unwrap();

    println!("Delivery from: {:#?}", delivery_from);
    println!("Delivery to: {:#?}", delivery_to);

    println!("Getting from db");

    let now = Instant::now();

    let trades = get_trades(&pool, &delivery_from, &delivery_to).await?;

    let elapsed = now.elapsed();
    println!("Got {} trades from database", trades.len());
    println!("Elapsed: {:.2?}", elapsed);

    println!("Creating report");

    let now = Instant::now();
    let report = Report::new(delivery_from, delivery_to, trades)?;
    println!("Elapsed: {:.2?}", now.elapsed());

    println!("Total gross profit: {:?}", report.gross_profit(None, None));
    println!("Total revenue: {:?}", report.revenue(None, None));
    println!("Total costs: {:?}", report.costs(None, None));
    println!("Total mw sold: {:?}", report.mw_sold(None, None));
    println!("Total mw bought: {:?}", report.mw_bought(None, None));

    println!("Done :)");
    Ok(())
}
