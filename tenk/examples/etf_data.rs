//! ETF data example.

use tenk::sources::{EastMoneySource, SinaSource, THSSource};
use tenk::{DataClient, KLineType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== tenk ETF Data Example ===\n");

    // Create client with ETF sources
    let client = DataClient::new()
        .with_fund_source(EastMoneySource::default())
        .with_fund_source(THSSource::default())
        .with_fund_source(SinaSource::default());

    // 1. Get all ETF codes
    println!("1. Fetching all ETF codes...");
    let etfs = client.get_all_etf_codes().await?;
    println!("   Found {} ETFs", etfs.len());

    // Show top ETFs by NAV
    let mut sorted_etfs: Vec<_> = etfs.iter().filter(|e| e.net_value.is_some()).collect();
    sorted_etfs.sort_by(|a, b| {
        b.net_value
            .unwrap_or(0.0)
            .partial_cmp(&a.net_value.unwrap_or(0.0))
            .unwrap()
    });

    println!("\n   Top 10 ETFs by NAV:");
    println!("   {:10} {:20} {:>10}", "Code", "Name", "NAV");
    println!("   {}", "-".repeat(44));
    for etf in sorted_etfs.iter().take(10) {
        println!(
            "   {:10} {:20} {:>10.3}",
            etf.fund_code,
            truncate_str(&etf.short_name, 18),
            etf.net_value.unwrap_or(0.0)
        );
    }
    println!();

    // 2. Get ETF historical data
    let popular_etfs = [("510300", "沪深300ETF"), ("159915", "创业板ETF")];

    println!("2. Fetching historical data for popular ETFs...");
    for (code, name) in &popular_etfs {
        match client
            .get_etf_market(
                code,
                Some("2025-01-01"),
                Some("2025-01-31"),
                KLineType::Daily,
            )
            .await
        {
            Ok(data) => {
                if !data.is_empty() {
                    let first = data.first().unwrap();
                    let last = data.last().unwrap();
                    let change = (last.close - first.open) / first.open * 100.0;
                    println!("   {} ({}):", code, name);
                    println!(
                        "      Records: {}, Period change: {:+.2}%",
                        data.len(),
                        change
                    );
                }
            }
            Err(e) => println!("   {} ({}): Error - {}", code, name, e),
        }
    }
    println!();

    // 3. Get ETF current prices
    println!("3. Fetching current ETF prices...");
    let etf_codes: Vec<&str> = popular_etfs.iter().map(|(c, _)| *c).collect();
    match client.get_etf_current(&etf_codes).await {
        Ok(current) => {
            println!(
                "   {:10} {:15} {:>10} {:>10}",
                "Code", "Name", "Price", "Change%"
            );
            println!("   {}", "-".repeat(50));
            for data in &current {
                let change_pct = data.change_pct.unwrap_or(0.0);
                let arrow = if change_pct >= 0.0 { "↑" } else { "↓" };
                println!(
                    "   {:10} {:15} {:>10.3} {:>9.2}%{}",
                    data.fund_code,
                    truncate_str(&data.short_name, 13),
                    data.price,
                    change_pct.abs(),
                    arrow
                );
            }
        }
        Err(e) => println!("   Error fetching current prices: {}", e),
    }
    println!();

    // 4. Get ETF minute data
    println!("4. Fetching minute data for 510300 (沪深300ETF)...");
    match client.get_etf_min("510300").await {
        Ok(minutes) => {
            println!("   Fetched {} minute records", minutes.len());
            if !minutes.is_empty() {
                if let Some(first) = minutes.first() {
                    println!(
                        "   First: {} - {:.3}",
                        first.trade_time.format("%H:%M"),
                        first.price
                    );
                }
                if let Some(last) = minutes.last() {
                    println!(
                        "   Last:  {} - {:.3}",
                        last.trade_time.format("%H:%M"),
                        last.price
                    );
                }
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    println!("\nDone!");
    Ok(())
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max_len - 3).collect::<String>())
    }
}
