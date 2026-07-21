# 数据类型

定义于 `tenk::data`，均在 crate 根 re-export。

## 枚举

### `Exchange`

| 变体 | 前缀 | 代码规则 |
|------|------|----------|
| `SH` | `sh` | `6xx`、`5xx`、`11xx` |
| `SZ` | `sz` | `0xx`、`3xx`、`1xx`（非 `11`） |
| `BJ` | `bj` | `4xx`、`8xx` |

辅助方法：`from_stock_code()`、`market_prefix()`、`eastmoney_secid()`。

### `KLineType`

| 变体 | API 值 |
|------|--------|
| `Daily` | 101 |
| `Weekly` | 102 |
| `Monthly` | 103 |
| `Quarterly` | 104 |
| `Min5` / `Min15` / `Min30` / `Min60` | 5 / 15 / 30 / 60 |

CLI 字符串解析：`KLineType::from_name("weekly")`。

### `NewsCategory`

| 变体 | 栏目代码 |
|------|----------|
| `Finance` | 102 |
| `Company` | 103 |
| `Stock` | 104 |
| `USMarket` | 105 |
| `Global` | 111 |
| `Domestic` | 106 |
| `Industry` | 110 |

解析：`NewsCategory::from_name("company")` 或按栏目代码。

### `AdjustType`

`None`、`Forward`（默认）、`Backward` — K 线复权方式。

## 主要结构体

### 股票

| 类型 | 用途 |
|------|------|
| `StockCode` | 代码、名称、交易所、上市日期 |
| `StockInfo` | 股本、行业等 |
| `MarketData` | OHLCV K 线 |
| `CurrentMarketData` | 实时行情 |
| `MinuteData` | 分时数据 |
| `OrderBookData` | 买卖盘口 |
| `TickData` | 逐笔成交 |
| `StockValuation` | PE、PB、市值等 |
| `TopHolder` | 股东信息 |
| `FundHolding` | 基金持仓 |
| `DividendData` | 分红记录 |

`MarketData::is_valid()` — 成交量与成交额均非零时返回 true。

### ETF

| 类型 | 用途 |
|------|------|
| `ETFCode` | 基金代码与元数据 |
| `ETFMarketData` | K 线 |
| `ETFCurrentData` | 实时行情 |
| `ETFMinuteData` | 分时 |

### 可转债

| 类型 | 用途 |
|------|------|
| `ConvertibleBondCode` | 转债与正股代码 |
| `BondCurrentData` | 实时行情 |

### 新闻

| 类型 | 用途 |
|------|------|
| `NewsArticle` | 列表项（标题、摘要、URL、时间） |
| `NewsContent` | 正文（HTML + 纯文本） |
| `NewsListResult` / `NewsSearchResult` | 分页包装 |

### 扩展行情

`CapitalFlowData`、`CapitalFlowHistory`、`BillboardItem`、`BillboardDetail`、`EarningsForecast`、`StockConnectData`、`MarginTradingData`、`IPOData`、`BlockTradeData`、`InstitutionalResearchData`、`ResearchReportData`。

### TradingView analytics

| 类型 | 用途 |
|------|------|
| `TvTechnicalAnalysis` | 各周期技术分析共识 |
| `TvAnalystData` | 分析师评级、目标价、预测、可选历史预估 |
| `TvAnalystRatings` | 买入 / 卖出 / 持有等计数 |
| `TvAnalystPriceTargets` | 平均、最高、最低、中位目标价 |
| `TvAnalystForecasts` | 下一财年 EPS / 营收预测 |
| `TvAnalystEstimates` | 季度与年度预估序列 |
| `TvEstimateSeries` / `TvEstimatePoint` | 历史预估点 |
| `TvSymbolMatch` | Symbol 搜索结果 |
| `TvScreenerResult` | 筛选器 / 热榜行 |
| `TvCalendarEvent` | 经济日历条目 |
| `TvIndicatorMeta` / `TvIndicatorSpec` / `TvIndicatorSeries` | 内置指标元数据与序列 |
| `TvStrategyReport` | 策略回测输出 |
| `TvReplayResult` | K 线回放结果 |
| `TvDrawing` | 图表 drawing |
| `TvUserSession` | 登录 session cookies 与 auth token |
| `TvPinePermUser` | 邀请制脚本 ACL 条目 |

### 板块

| 类型 | 用途 |
|------|------|
| `BoardItem` | 板块代码、名称、涨跌幅 |
| `BoardCrosswalkItem` | 东财 / 同花顺板块映射 |
| `BoardCrosswalkKind` | `Industry` 或 `Concept` |
| `LimitPoolItem` | 涨跌停池条目 |
| `MacroRecord` | CPI / GDP |
| `IndexCode` | 指数元数据 |

### 衍生品与财报

| 类型 | 用途 |
|------|------|
| `FuturesContract` | 期货合约 + `secid` |
| `OptionContract` | 期权合约 |
| `DerivativesQuote` | 期货/期权行情 |
| `FinancialRecord` | F10 财报行 |
| `FinancialReportKind` | 资产负债 / 利润 / 现金流 / 业绩摘要 |

### 相关标的

EastMoney 以 `"market.symbol"` 编码（如 `"1.600519"`、`"90.白酒"`）。

| 函数 | 输出 |
|------|------|
| `format_related_stocks(codes)` | `(Vec<RelatedStock>, Vec<String>)` — 股票与板块名 |
| `format_related_stocks_display(codes)` | 展示用标签字符串 |

市场码：`0` → 深，`1` → 沪，`90` → 板块，`116` → 港，等。

## 日期格式

API 边界支持 `YYYY-MM-DD` 或 `YYYYMMDD`。内部使用 `tenk::util::normalize_date_bound` 与 `parse_trade_date`。
