# 10K (tenk)

> The market index will raise to 10K! 大盘还要上去，要涨到一万点！

A Rust library for fetching market data from multiple sources.

## Installation

```toml
[dependencies]
tenk = "0.1"
tokio = { version = "1.35", features = ["full"] }
```

## Quick Start

```rust
use tenk::sources::{EastMoneySource, SinaSource};
use tenk::{DataClient, KLineType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client with data sources
    let client = DataClient::new()
        .with_source(EastMoneySource::default())
        .with_source(SinaSource::default())
        .with_fund_source(EastMoneySource::default())
        .with_bond_market_source(SinaSource::default());

    // Get stock codes
    let codes = client.get_all_codes().await?;
    println!("Total stocks: {}", codes.len());

    // Get current prices
    let prices = client.get_market_current(&["300059", "600519"]).await?;
    for p in &prices {
        println!("{}: ¥{:.2}", p.short_name, p.price);
    }

    // Get K-line data
    let kline = client
        .get_market("600519", Some("2025-01-01"), None, KLineType::Daily)
        .await?;
    
    // Get ETF codes
    let etfs = client.get_all_etf_codes().await?;

    // Get bond data
    let bonds = client.get_bond_current(None).await?;

    Ok(())
}
```

## Examples

```bash
cargo run --example quick_start   # Basic usage
cargo run --example stock_data    # Stock market data
cargo run --example etf_data      # ETF data
cargo run --example bond_data     # Convertible bond data
```

## Features

### Stock Data
```rust
// All stock codes
let codes = client.get_all_codes().await?;

// Historical K-line (Daily/Weekly/Monthly/Minute)
let data = client
    .get_market("600519", Some("2025-01-01"), None, KLineType::Daily)
    .await?;

// Current prices
let prices = client.get_market_current(&["300059", "600519"]).await?;

// Minute data
let minutes = client.get_market_min("600519").await?;
```

### ETF Data
```rust
// All ETF codes
let etfs = client.get_all_etf_codes().await?;

// ETF K-line
let data = client
    .get_etf_market("510300", Some("2025-01-01"), None, KLineType::Daily)
    .await?;

// ETF minute data
let minutes = client.get_etf_min("510300").await?;
```

### Bond Data
```rust
// All convertible bonds
let bonds = client.get_bond_current(None).await?;

// Specific bond
let bonds = client
    .get_bond_current(Some(&["127046"]))
    .await?;
```

## CLI

### Quick Examples

```bash
# Stock quotes
tenk stock quote 600519 300059

# K-line data (last 10 daily records)
tenk stock kline 600519 -l 10

# Weekly K-line
tenk stock kline 600519 -k weekly -l 5

# Stock info
tenk stock info 600519

# ETF quotes
tenk etf quote 510300 159915

# Bond top gainers
tenk bond quote --top-gainers 10

# Output as JSON
tenk stock quote 600519 -o json

# Output as CSV
tenk stock list -l 100 -o csv > stocks.csv
```

See [tenk-cli/README.md](tenk-cli/README.md) for full documentation.


## License

MIT
