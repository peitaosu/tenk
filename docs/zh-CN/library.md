# 库指南

## 依赖

```toml
[dependencies]
tenk = "0.2"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## ClientBuilder

推荐入口。为所选 provider 注册全部相关 trait 实现。

```rust
use tenk::ClientBuilder;

let client = ClientBuilder::new().build()?;

let client = ClientBuilder::with_sources(&[tenk::SourceKind::Sina])
    .with_proxy("http://127.0.0.1:7890")
    .build()?;
```

| `SourceKind` | 注册能力 |
|--------------|----------|
| `Eastmoney` | 股票、ETF、可转债、新闻、扩展行情（估值、资金流、龙虎榜等） |
| `Sina` | 股票、ETF、可转债行情 |
| `Ths` | 股票、ETF、可转债信息 |

默认：三个 provider 全部启用。

## DataClient

统一异步 API。方法委托给已配置数据源，并自动回退。

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
| `get_billboard_detail(code, date)` | `BillboardDetail` |
| `get_earnings_forecast(period, page, limit)` | `Vec<EarningsForecast>` |
| `get_stock_connect(limit)` | `Vec<StockConnectData>` |
| `get_margin_trading(code, limit)` | `Vec<MarginTradingData>` |
| `get_ipo_list(limit)` | `Vec<IPOData>` |
| `get_block_trades(limit)` | `Vec<BlockTradeData>` |
| `get_institutional_research(limit)` | `Vec<InstitutionalResearchData>` |
| `get_research_reports(code, limit)` | `Vec<ResearchReportData>` |

## 手动装配

需要细粒度控制时使用：

```rust
use tenk::{DataClient, sources::EastMoneySource};

let em = EastMoneySource::try_new(None)?;
let client = DataClient::new()
    .with_source(em.clone())
    .with_extended_market(em);
```

`with_source` 要求 `StockMarketSource + StockInfoSource + Clone`。

## 错误

所有方法返回 `DataResult<T>`（`Result<T, DataError>`）。

| 错误 | 可恢复 | 行为 |
|------|--------|------|
| `Network`、`Parse`、`SourceUnavailable`、`RateLimitExceeded`、`NotSupported` | 是 | 尝试下一数据源 |
| `NoDataAvailable` | 否 | 所有源均无数据 |
| `InvalidStockCode`、`InvalidDate`、`Config`、`Custom` | 否 | 立即返回 |

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

CLI 代理：`--proxy` 参数。
