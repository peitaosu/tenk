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

// Stock valuation metrics
use tenk::sources::EastMoneySource;
let source = EastMoneySource::default();
let valuation = source.get_valuation("600519").await?;

// Top shareholders
use tenk::traits::HoldingsSource;
let holders = source.get_top_holders("600519", Some(10)).await?;

// Fund holdings
let funds = source.get_fund_holdings("600519", Some(10)).await?;

// Dividend history
use tenk::traits::DividendSource;
let dividends = source.get_dividends("600519").await?;
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

### Market Data

```rust
use tenk::traits::*;
use tenk::sources::EastMoneySource;

let source = EastMoneySource::default();

// Capital flow
let flow = source.get_capital_flow(&["600519", "300059"]).await?;

// Billboard list
let billboard = source.get_billboard_list(None).await?;

// Earnings forecast
let forecast = source.get_earnings_forecast(None, 1, 50).await?;

// Stock Connect data
let connect = source.get_stock_connect(Some(30)).await?;

// Margin trading
let margin = source.get_margin_trading("600519", Some(30)).await?;

// IPO list
let ipo = source.get_ipo_list(Some(10)).await?;

// Block trades
let blocks = source.get_block_trades(Some(10)).await?;

// Institutional research
let research = source.get_institutional_research(Some(10)).await?;

// Research reports
let reports = source.get_research_reports(None, Some(10)).await?;
```

### News Data

```rust
use tenk::traits::NewsSource;

// Get latest news
let news = source.get_news_list(tenk::NewsCategory::Finance, 1, 20).await?;

// Search news
let results = source.search_news("600519", 1, 10).await?;

// Read full content
let content = source.get_news_content("202601153620739638").await?;
```

## Data Sources
- EastMoney
- Sina
- THS

## License

MIT

