# Library Guide

## Setup

```toml
[dependencies]
tenk = "0.2"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## ClientBuilder

```rust
use tenk::ClientBuilder;

let client = ClientBuilder::new().build()?;

let client = ClientBuilder::with_sources(&[tenk::SourceKind::Sina])
    .with_proxy("http://127.0.0.1:7890")
    .build()?;
```

| `SourceKind` | Registers |
|--------------|-----------|
| `Eastmoney` | Stock, ETF, bond, news, extended market |
| `Sina` | Stock, ETF, bond, index quote, futures quote, news, research reports |
| `Ths` | Stock, ETF, bond, board, news, research reports |
| `Tradingview` | Global market (WS), search, TA, analyst, screener, calendar, indicators / strategy / replay |

Default: `SourceKind::DEFAULT` (`SourceKind::ALL` is identical). MCP uses `SourceKind::ALL`.

## DataClient

Async API. Multiple sources tried by `priority()` when configured.

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
| `get_billboard_detail(code, date)` | `Vec<BillboardDetail>` |
| `get_earnings_forecast(period, page, limit)` | `Vec<EarningsForecast>` |
| `get_stock_connect(limit)` | `Vec<StockConnectData>` |
| `get_margin_trading(code, limit)` | `Vec<MarginTradingData>` |
| `get_ipo_list(limit)` | `Vec<IPOData>` |
| `get_block_trades(limit)` | `Vec<BlockTradeData>` |
| `get_institutional_research(limit)` | `Vec<InstitutionalResearchData>` |
| `get_research_reports(code, limit)` | `Vec<ResearchReportData>` |

### TradingView

WS-backed calls need a proxy in many regions.

| Method | Returns |
|--------|---------|
| `get_technical_analysis(symbol)` | `TvTechnicalAnalysis` |
| `get_analyst(symbol)` | `TvAnalystData` |
| `search_symbols(query, filter, offset)` | `Vec<TvSymbolMatch>` |
| `run_screener(request)` | `TvScreenerResult` |
| `get_hotlist(market, kind, limit)` | `TvScreenerResult` |
| `get_economic_calendar(from, to, countries)` | `Vec<TvCalendarEvent>` |
| `search_indicators(query)` | `Vec<TvIndicatorMeta>` |
| `get_indicator_spec(id, version)` | `TvIndicatorSpec` |
| `get_indicator_series(symbol, id, version, options)` | `TvIndicatorSeries` |
| `get_strategy_report(symbol, id, version, options)` | `TvStrategyReport` |
| `get_chart_replay(symbol, replay_from, steps, options)` | `TvReplayResult` |
| `get_chart_drawings(layout, symbol, user_id)` | `Vec<TvDrawing>` |

`get_chart_drawings` merges `chart_id` 1, 2, and `_shared`. Session cookies required. `get_strategy_report` is best-effort.

### TradingViewSource

Direct TV REST helpers. Obtain via `TradingViewSource::try_new(proxy)` or from a built client’s TV source.

| Method | Returns |
|--------|---------|
| `resolve_auth_token()` | WS auth token from homepage |
| `login(username, password)` | `TvUserSession` |
| `private_indicators()` | `Vec<TvIndicatorMeta>` |
| `pine_perm(pine_id)` | `TvPinePerm` |

| `TvPinePerm` method | Returns |
|---------------------|---------|
| `list_users(limit)` | `Vec<TvPinePermUser>` |
| `add_user(username, expiration)` | status string |
| `modify_expiration(username, expiration)` | status string |
| `remove_user(username)` | status string |

Session cookies required for `private_indicators`, `pine_perm`, and `get_chart_drawings`.

## Manual wiring

```rust
use tenk::{DataClient, sources::EastMoneySource};

let em = EastMoneySource::try_new(None)?;
let client = DataClient::new()
    .with_source(em.clone())
    .with_extended_market(em);
```

## Errors

All methods return `DataResult<T>` (`Result<T, DataError>`).

| Error | Multi-source | Single-source |
|-------|--------------|---------------|
| `Network`, `Parse`, `SourceUnavailable`, `RateLimitExceeded` | Next source | Return error |
| `NotSupported` | Next source | Return error |
| `NoDataAvailable` | Next source | Return error |
| `InvalidStockCode`, `InvalidDate`, `Config`, `Custom` | Return error | Return error |

Multi-source: more than one provider registered for the trait. Single-source: exactly one provider (e.g. `-s eastmoney`).

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
| `TENK_TV_SESSION`, `TENK_TV_SIGNATURE` | Drawings, private indicators, pine perm; WS token resolution |
| `TENK_TV_AUTH_TOKEN` | Optional WS auth token override |

CLI proxy: `--proxy` flag.
