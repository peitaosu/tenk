# Data Types

Defined in `tenk::data`. All public types are re-exported from the crate root.

## Enums

### `Exchange`

| Variant | Prefix | Code pattern |
|---------|--------|--------------|
| `SH` | `sh` | `6xx`, `5xx`, `11xx` |
| `SZ` | `sz` | `0xx`, `3xx`, `1xx` (not `11`) |
| `BJ` | `bj` | `4xx`, `8xx` |

Helpers: `from_stock_code()`, `market_prefix()`, `eastmoney_secid()`.

### `KLineType`

| Variant | API value |
|---------|-----------|
| `Daily` | 101 |
| `Weekly` | 102 |
| `Monthly` | 103 |
| `Quarterly` | 104 |
| `Min5` / `Min15` / `Min30` / `Min60` | 5 / 15 / 30 / 60 |

Parse from CLI strings: `KLineType::from_name("weekly")`.

### `NewsCategory`

| Variant | Column code |
|---------|-------------|
| `Finance` | 102 |
| `Company` | 103 |
| `Stock` | 104 |
| `USMarket` | 105 |
| `Global` | 111 |
| `Domestic` | 106 |
| `Industry` | 115 |

Parse: `NewsCategory::from_name("company")` or by column code.

### `AdjustType`

`None`, `Forward` (default), `Backward` — K-line adjustment mode.

## Key structs

### Stock

| Type | Purpose |
|------|---------|
| `StockCode` | Code, name, exchange, list date |
| `StockInfo` | Shares, industry, etc. |
| `MarketData` | OHLCV K-line bar |
| `CurrentMarketData` | Real-time quote |
| `MinuteData` | Intraday tick |
| `OrderBookData` | Bid/ask levels |
| `TickData` | Trade-by-trade |
| `StockValuation` | PE, PB, market cap |
| `TopHolder` | Shareholder record |
| `FundHolding` | Fund position |
| `DividendData` | Dividend history |

`MarketData::is_valid()` — true when volume and amount are non-zero.

### ETF

| Type | Purpose |
|------|---------|
| `ETFCode` | Fund code and metadata |
| `ETFMarketData` | K-line bar |
| `ETFCurrentData` | Real-time quote |
| `ETFMinuteData` | Intraday bar |

### Bond

| Type | Purpose |
|------|---------|
| `ConvertibleBondCode` | Bond and underlying stock code |
| `BondCurrentData` | Real-time bond quote |

### News

| Type | Purpose |
|------|---------|
| `NewsArticle` | List item (title, digest, URL, time) |
| `NewsContent` | Full article (HTML + plain text) |
| `NewsListResult` / `NewsSearchResult` | Paginated wrappers |

### Market analytics

`CapitalFlowData`, `CapitalFlowHistory`, `BillboardItem`, `BillboardDetail`, `EarningsForecast`, `StockConnectData`, `MarginTradingData`, `IPOData`, `BlockTradeData`, `InstitutionalResearchData`, `ResearchReportData`.

### Boards

| Type | Purpose |
|------|---------|
| `BoardItem` | Board code, name, price, change |
| `BoardCrosswalkItem` | Matched EastMoney / THS board codes |
| `BoardCrosswalkKind` | `Industry` or `Concept` |
| `LimitPoolItem` | Limit-up/down pool entry |
| `MacroRecord` | CPI / GDP macro series |
| `IndexCode` | Index metadata |

### Derivatives & financials

| Type | Purpose |
|------|---------|
| `FuturesContract` | Futures contract metadata + `secid` |
| `OptionContract` | Listed option contract |
| `DerivativesQuote` | Futures/options quote |
| `FinancialRecord` | F10 statement row |
| `FinancialReportKind` | Balance / income / cashflow / performance |

### Related stocks

EastMoney encodes related symbols as `"market.symbol"` strings (e.g. `"1.600519"`, `"90.白酒"`).

| Function | Output |
|----------|--------|
| `format_related_stocks(codes)` | `(Vec<RelatedStock>, Vec<String>)` — stocks and sector names |
| `format_related_stocks_display(codes)` | Display-ready label strings |

Market codes: `0` → SZ, `1` → SH, `90` → sector, `116` → HK, etc.

## Date format

API date strings use `YYYY-MM-DD` or `YYYYMMDD`. Internal parsing via `tenk::util::normalize_date_bound` and `parse_trade_date`.
