//! Quick start example.

use tenk::{ClientBuilder, KLineType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== tenk Quick Start ===\n");

    let client = ClientBuilder::new().build()?;

    let codes = client.get_all_codes(None).await?;
    println!("📊 Total stocks: {}", codes.len());

    let prices = client.get_market_current(&["300059", "600519"]).await?;
    for p in &prices {
        println!("💰 {} ({}): ¥{:.2}", p.stock_code, p.short_name, p.price);
    }

    let kline = client
        .get_market(
            "600519",
            Some("2025-01-01"),
            Some("2025-01-10"),
            KLineType::Daily,
        )
        .await?;
    println!("📈 K-line records: {}", kline.len());

    let etfs = client.get_all_etf_codes(None).await?;
    println!("🏦 Total ETFs: {}", etfs.len());

    let bonds = client.get_bond_current(None).await?;
    println!("📜 Total bonds: {}", bonds.len());

    println!("\n✅ All data fetched successfully!");
    Ok(())
}
