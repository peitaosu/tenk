# tenk

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
    let client = DataClient::new()
        .with_source(EastMoneySource::default())
        .with_source(SinaSource::default())
        .with_fund_source(EastMoneySource::default())
        .with_bond_market_source(SinaSource::default());

    // Get current prices
    let prices = client.get_market_current(&["300059", "600519"]).await?;
    for p in &prices {
        println!("{}: ¥{:.2}", p.short_name, p.price);
    }

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

## API Reference

### Stock Data

```rust
// All stock codes
let codes = client.get_all_codes().await?;

// Stock info
let info = client.get_stock_info("600519").await?;

// Historical K-line (Daily/Weekly/Monthly/Quarterly/Min5/Min15/Min30/Min60)
let data = client
    .get_market("600519", Some("2025-01-01"), None, KLineType::Daily)
    .await?;

// Current prices (batch)
let prices = client.get_market_current(&["300059", "600519"]).await?;

// Minute data (intraday)
let minutes = client.get_market_min("600519").await?;

// Order book
let orderbook = client.get_order_book("600519").await?;

// Tick data
let ticks = client.get_ticks("600519").await?;
```

### ETF Data

```rust
// All ETF codes
let etfs = client.get_all_etf_codes().await?;

// ETF current prices
let prices = client.get_etf_current(&["510300", "159915"]).await?;

// ETF K-line
let data = client
    .get_etf_market("510300", Some("2025-01-01"), None, KLineType::Daily)
    .await?;

// ETF minute data
let minutes = client.get_etf_min("510300").await?;
```

### Bond Data

```rust
// All convertible bond codes
let codes = client.get_all_bond_codes().await?;

// All bond quotes
let bonds = client.get_bond_current(None).await?;

// Specific bonds
let bonds = client.get_bond_current(Some(&["127046", "113050"])).await?;
```

## Data Sources
- EastMoney
- Sina
- THS

## License

MIT

