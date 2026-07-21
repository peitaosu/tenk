# 库指南

## 依赖

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

| `SourceKind` | 注册能力 |
|--------------|----------|
| `Eastmoney` | 股票、ETF、可转债、新闻、扩展行情 |
| `Sina` | 股票、ETF、可转债、指数行情、期货行情、新闻、研报 |
| `Ths` | 股票、ETF、可转债、板块、新闻、研报 |
| `Tradingview` | 全球行情（WS）、搜索、TA、分析师、筛选、日历、指标 / 策略 / 回放 |

默认：`SourceKind::DEFAULT`（与 `SourceKind::ALL` 相同）。MCP 使用 `SourceKind::ALL`。

## DataClient

异步 API。已配置多源时按 `priority()` 依次调用。

### 股票

| 方法 | 返回类型 |
|------|----------|
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

| 方法 | 返回类型 |
|------|----------|
| `get_all_etf_codes(limit)` | `Vec<ETFCode>` |
| `get_etf_market(code, start, end, k_type)` | `Vec<ETFMarketData>` |
| `get_etf_current(codes)` | `Vec<ETFCurrentData>` |
| `get_etf_min(code)` | `Vec<ETFMinuteData>` |

### 可转债

| 方法 | 返回类型 |
|------|----------|
| `get_all_bond_codes(limit)` | `Vec<ConvertibleBondCode>` |
| `get_bond_current(codes)` | `Vec<BondCurrentData>` |

`codes` 传 `None` 表示获取全部行情。

### 新闻

| 方法 | 返回类型 |
|------|----------|
| `get_news(category, page, limit)` | `Vec<NewsArticle>` |
| `search_news(keyword, page, limit)` | `Vec<NewsArticle>` |
| `get_news_content(id)` | `NewsContent` |

### 扩展行情

| 方法 | 返回类型 |
|------|----------|
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

WS 相关调用在多数网络环境下需要代理。

| 方法 | 返回类型 |
|------|----------|
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

`get_chart_drawings` 合并 `chart_id` 1、2、`_shared`。需 session cookies。`get_strategy_report` 为 best-effort。

### TradingViewSource

TV REST 直连 API。通过 `TradingViewSource::try_new(proxy)` 或已装配客户端中的 TV 源获取。

| 方法 | 返回类型 |
|------|----------|
| `resolve_auth_token()` | homepage 解析的 WS auth token |
| `login(username, password)` | `TvUserSession` |
| `private_indicators()` | `Vec<TvIndicatorMeta>` |
| `pine_perm(pine_id)` | `TvPinePerm` |

| `TvPinePerm` 方法 | 返回类型 |
|-------------------|----------|
| `list_users(limit)` | `Vec<TvPinePermUser>` |
| `add_user(username, expiration)` | 状态字符串 |
| `modify_expiration(username, expiration)` | 状态字符串 |
| `remove_user(username)` | 状态字符串 |

`private_indicators`、`pine_perm`、`get_chart_drawings` 需 session cookies。

## 手动装配

```rust
use tenk::{DataClient, sources::EastMoneySource};

let em = EastMoneySource::try_new(None)?;
let client = DataClient::new()
    .with_source(em.clone())
    .with_extended_market(em);
```

## 错误

所有方法返回 `DataResult<T>`（`Result<T, DataError>`）。

| 错误 | 多源 | 单源 |
|------|------|------|
| `Network`、`Parse`、`SourceUnavailable`、`RateLimitExceeded` | 下一源 | 直接返回 |
| `NotSupported` | 下一源 | 直接返回 |
| `NoDataAvailable` | 下一源 | 直接返回 |
| `InvalidStockCode`、`InvalidDate`、`Config`、`Custom` | 直接返回 | 直接返回 |

多源：该 trait 注册超过一个 provider。单源：仅一个 provider（如 `-s eastmoney`）。

## 示例

```bash
cargo run --example quick_start
cargo run --example stock_data
cargo run --example etf_data
cargo run --example bond_data
```

## 环境变量

| 变量 | 使用者 |
|------|--------|
| `TENK_PROXY` | MCP 服务（`tenk --mcp`） |
| `TENK_TV_SESSION`、`TENK_TV_SIGNATURE` | Drawings、私有指标、pine perm；WS token 解析 |
| `TENK_TV_AUTH_TOKEN` | 可选 WS auth token 覆盖 |

CLI 代理：`--proxy` 参数。
