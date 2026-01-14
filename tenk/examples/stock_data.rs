//! Stock data example.

use tenk::sources::{EastMoneySource, SinaSource, THSSource};
use tenk::{DataClient, KLineType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== TenK Stock Data Example ===\n");

    // Create client with all sources for fallback
    let client = DataClient::new()
        .with_source(EastMoneySource::default())
        .with_source(SinaSource::default())
        .with_source(THSSource::default());

    // 1. Get all stock codes
    println!("1. Fetching all stock codes...");
    let codes = client.get_all_codes().await?;
    println!("   Found {} stocks", codes.len());
    
    // Show stocks by exchange
    let sh_count = codes.iter().filter(|c| c.exchange.to_string() == "SH").count();
    let sz_count = codes.iter().filter(|c| c.exchange.to_string() == "SZ").count();
    println!("   - Shanghai (SH): {}", sh_count);
    println!("   - Shenzhen (SZ): {}", sz_count);
    println!();

    // 2. Get historical K-line data (Daily)
    println!("2. Fetching daily K-line for 300059 (东方财富)...");
    let daily = client
        .get_market("300059", Some("2025-01-01"), Some("2025-01-31"), KLineType::Daily)
        .await?;
    println!("   Fetched {} daily records", daily.len());
    if let Some(first) = daily.first() {
        println!("   First: {} O:{:.2} H:{:.2} L:{:.2} C:{:.2}", 
            first.trade_date, first.open, first.high, first.low, first.close);
    }
    println!();

    // 3. Get weekly K-line data
    println!("3. Fetching weekly K-line for 600519 (贵州茅台)...");
    let weekly = client
        .get_market("600519", Some("2025-01-01"), Some("2025-01-31"), KLineType::Weekly)
        .await?;
    println!("   Fetched {} weekly records", weekly.len());
    for record in weekly.iter().take(3) {
        println!("   {} - Close: {:.2}, Change: {:+.2}%", 
            record.trade_date, record.close, record.change_pct);
    }
    println!();

    // 4. Get current market prices
    println!("4. Fetching current prices for multiple stocks...");
    let stocks = ["300059", "600519"];
    let current = client.get_market_current(&stocks).await?;
    println!("   {:12} {:12} {:>10} {:>10}", "Code", "Name", "Price", "Change%");
    println!("   {}", "-".repeat(48));
    for data in &current {
        let arrow = if data.change_pct >= 0.0 { "↑" } else { "↓" };
        println!("   {:12} {:12} {:>10.2} {:>9.2}%{}", 
            data.stock_code, data.short_name, data.price, data.change_pct.abs(), arrow);
    }
    println!();

    // 5. Get minute data
    println!("5. Fetching minute data for 600519...");
    let minutes = client.get_market_min("600519").await?;
    println!("   Fetched {} minute records", minutes.len());
    if !minutes.is_empty() {
        if let Some(first) = minutes.first() {
            println!("   First: {} - Price: {:.2}", 
                first.trade_time.format("%H:%M"), first.price);
        }
        if let Some(last) = minutes.last() {
            println!("   Last:  {} - Price: {:.2}", 
                last.trade_time.format("%H:%M"), last.price);
        }
        
        // Calculate price range
        let prices: Vec<f64> = minutes.iter().map(|m| m.price).collect();
        let min_price = prices.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_price = prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        println!("   Range: {:.2} - {:.2}", min_price, max_price);
    }
    println!();

    // 6. Get stock info
    println!("6. Fetching stock info from different sources...");
    let test_stocks = ["300059", "600519"];
    for code in test_stocks {
        match client.get_stock_info(code).await {
            Ok(info) => {
                println!("   {} ({}):", code, info.short_name);
                if let Some(shares) = info.total_shares {
                    println!("      Total shares: {:.0}", shares);
                }
                if let Some(circ) = info.circulating_shares {
                    println!("      Circulating: {:.0}", circ);
                }
            }
            Err(e) => println!("   {}: Error - {}", code, e),
        }
    }
    println!();

    // 7. Test THS source directly
    println!("7. Testing THS source directly...");
    let ths = THSSource::default();
    use tenk::traits::StockMarketSource;
    
    // Get K-line from THS
    match ths.get_market("600519", Some("2025-01-01"), Some("2025-01-31"), KLineType::Daily).await {
        Ok(data) => {
            println!("   THS K-line for 600519: {} records", data.len());
            if let Some(first) = data.first() {
                println!("   First: {} - Close: {:.2}", first.trade_date, first.close);
            }
        }
        Err(e) => println!("   THS K-line error: {}", e),
    }
    
    // Get current from THS
    match ths.get_market_current(&["600519", "300059"]).await {
        Ok(data) => {
            println!("   THS current quotes: {} records", data.len());
            for d in &data {
                println!("   {} ({}): {:.2}", d.stock_code, d.short_name, d.price);
            }
        }
        Err(e) => println!("   THS current error: {}", e),
    }
    
    // Get minute from THS
    match ths.get_market_min("600519").await {
        Ok(data) => {
            println!("   THS minute for 600519: {} records", data.len());
        }
        Err(e) => println!("   THS minute error: {}", e),
    }

    println!("\nDone!");
    Ok(())
}
