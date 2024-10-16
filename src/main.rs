use std::env;
use std::time::Instant;

mod db;
mod report;
mod trade;

use anyhow::Result;
use db::{get_trades, init_db_pool};
use report::Report;
use time::macros::{date, offset, time};
use time::OffsetDateTime;

#[tokio::main]
async fn main() -> Result<()> {
    // I think I'll have to swap 'time' for 'chrono' as the 'chrono-tz' crate looks very cool for handling timezones
    // more properly, which I'll have to...

    // Load environment variables from .env
    dotenvy::dotenv()?;
    let db_url = env::var("DATABASE_URL")?;

    println!("Initialising sqlx ...");

    let pool = init_db_pool(&db_url).await?;

    let delivery_from =
        OffsetDateTime::new_in_offset(date!(2024 - 10 - 01), time!(22:00:00), offset!(UTC));
    let delivery_to =
        OffsetDateTime::new_in_offset(date!(2024 - 10 - 10), time!(22:00:00), offset!(UTC));

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
