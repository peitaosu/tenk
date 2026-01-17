//! Quick start example.

use tenk::sources::{EastMoneySource, SinaSource, THSSource};
use tenk::{DataClient, KLineType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== tenk Quick Start ===\n");

    // Create client and add sources
    let client = DataClient::new()
        .with_source(EastMoneySource::default())
        .with_source(SinaSource::default())
        .with_fund_source(EastMoneySource::default())
        .with_fund_source(SinaSource::default())
        .with_fund_market_source(THSSource::default())
        .with_bond_source(EastMoneySource::default())
        .with_bond_info_source(THSSource::default())
        .with_bond_market_source(SinaSource::default());

    // 1. Get stock codes
    let codes = client.get_all_codes(None).await?;
    println!("📊 Total stocks: {}", codes.len());

    // 2. Get current prices
    let prices = client.get_market_current(&["300059", "600519"]).await?;
    for p in &prices {
        println!("💰 {} ({}): ¥{:.2}", p.stock_code, p.short_name, p.price);
    }

    // 3. Get K-line data
    let kline = client
        .get_market(
            "600519",
            Some("2025-01-01"),
            Some("2025-01-10"),
            KLineType::Daily,
        )
        .await?;
    println!("📈 K-line records: {}", kline.len());

    // 4. Get ETF codes
    let etfs = client.get_all_etf_codes(None).await?;
    println!("🏦 Total ETFs: {}", etfs.len());

    // 5. Get bond data
    let bonds = client.get_bond_current(None).await?;
    println!("📜 Total bonds: {}", bonds.len());

    println!("\n✅ All data fetched successfully!");
    Ok(())
}
