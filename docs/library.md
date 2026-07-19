# Library Guide

## Setup

```toml
[dependencies]
tenk = "0.2"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## ClientBuilder

Preferred entry point. Registers all trait impls for each selected provider.

```rust
use tenk::ClientBuilder;

let client = ClientBuilder::new().build()?;

let client = ClientBuilder::with_sources(&[tenk::SourceKind::Sina])
    .with_proxy("http://127.0.0.1:7890")
    .build()?;
```

| `SourceKind` | Registers |
|--------------|-----------|
| `Eastmoney` | Stock, ETF, bond, news, extended market (valuation, flow, billboard, …) |
| `Sina` | Stock, ETF, bond market |
| `Ths` | Stock, ETF, bond info |

Default: all three providers.

## DataClient

Unified async API. Methods delegate to configured sources with automatic fallback.

### Stock

| Method | Returns |
|--------|---------|
| `get_all_codes(limit)` | `Vec<StockCode>` |
| `get_stock_info(code)` | `StockInfo` |
| `get_market(code, start, end, k_type)` | `Vec<MarketData>` |
| `get_market_current(codes)` | `Vec<CurrentMarketData>` |
| `get_market_min(code)` | `Vec<MinuteData>` |
| `get_order_book(code)` | `OrderBookData` |
| `get_ticks(code)` | `Vec<TickData>` |
| `get_valuation(code)` | `StockValuation` |
| `get_top_holders(code)` | `Vec<TopHolder>` |
| `get_fund_holdings(code, limit)` | `Vec<FundHolding>` |
| `get_dividends(code)` | `Vec<DividendData>` |

### ETF

| Method | Returns |
|--------|---------|
| `get_all_etf_codes(limit)` | `Vec<ETFCode>` |
| `get_etf_market(code, start, end, k_type)` | `Vec<ETFMarketData>` |
| `get_etf_current(codes)` | `Vec<ETFCurrentData>` |
| `get_etf_min(code)` | `Vec<ETFMinuteData>` |

### Bond

| Method | Returns |
|--------|---------|
| `get_all_bond_codes(limit)` | `Vec<ConvertibleBondCode>` |
| `get_bond_current(codes)` | `Vec<BondCurrentData>` |

Pass `None` for `codes` to fetch all quotes.

### News

| Method | Returns |
|--------|---------|
| `get_news(category, page, limit)` | `Vec<NewsArticle>` |
| `search_news(keyword, page, limit)` | `Vec<NewsArticle>` |
| `get_news_content(id)` | `NewsContent` |

### Extended market

| Method | Returns |
|--------|---------|
| `get_capital_flow(codes)` | `Vec<CapitalFlowData>` |
| `get_capital_flow_history(code, limit)` | `Vec<CapitalFlowHistory>` |
| `get_billboard_list(date)` | `Vec<BillboardItem>` |
| `get_billboard_detail(code, date)` | `BillboardDetail` |
| `get_earnings_forecast(period, page, limit)` | `Vec<EarningsForecast>` |
| `get_stock_connect(limit)` | `Vec<StockConnectData>` |
| `get_margin_trading(code, limit)` | `Vec<MarginTradingData>` |
| `get_ipo_list(limit)` | `Vec<IPOData>` |
| `get_block_trades(limit)` | `Vec<BlockTradeData>` |
| `get_institutional_research(limit)` | `Vec<InstitutionalResearchData>` |
| `get_research_reports(code, limit)` | `Vec<ResearchReportData>` |

## Manual wiring

Use when you need fine-grained source control:

```rust
use tenk::{DataClient, sources::EastMoneySource};

let em = EastMoneySource::try_new(None)?;
let client = DataClient::new()
    .with_source(em.clone())
    .with_extended_market(em);
```

`with_source` requires `StockMarketSource + StockInfoSource + Clone`.

## Errors

All methods return `DataResult<T>` (`Result<T, DataError>`).

| Error | Recoverable | Effect |
|-------|-------------|--------|
| `Network`, `Parse`, `SourceUnavailable`, `RateLimitExceeded`, `NotSupported` | Yes | Try next source |
| `NoDataAvailable` | No | All sources exhausted |
| `InvalidStockCode`, `InvalidDate`, `Config`, `Custom` | No | Return immediately |

## Examples

```bash
cargo run --example quick_start
cargo run --example stock_data
cargo run --example etf_data
cargo run --example bond_data
```

## Environment

| Variable | Used by |
|----------|---------|
| `TENK_PROXY` | MCP server (`tenk --mcp`) |

CLI proxy: `--proxy` flag.
