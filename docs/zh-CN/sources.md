# 数据源

Provider：EastMoney、Sina、THS、TradingView。各实现 `tenk::traits` 子集。

优先级：`DataSource::priority()`，数值越小越优先。TradingView = 4。

| Provider | 门户 | 说明 |
|----------|------|------|
| EastMoney | [eastmoney.com](https://www.eastmoney.com) | A 股 + 扩展行情 |
| Sina | [finance.sina.com.cn](https://finance.sina.com.cn) | A 股行情 / 新闻 / 研报 |
| THS | [10jqka.com.cn](https://www.10jqka.com.cn) | 板块 / 新闻 / 研报 |
| TradingView | [tradingview.com](https://www.tradingview.com) | 全球 analytics；WS/新闻需 `--proxy` |

## 能力矩阵

✓ = 已实现。— = 未实现。

### 行情

| 能力 | EM | Sina | THS | TV |
|------|:--:|:----:|:---:|:--:|
| 股票实时行情 | ✓ | ✓ | ✓ | ✓ |
| 股票 K 线（日 / 周 / 月） | ✓ | ✓ | ✓ | ✓ |
| 股票 K 线（5 / 15 分钟） | ✓ | ✓ | — | ✓ |
| 股票 K 线（30 / 60 分钟） | ✓ | ✓ | ✓ | ✓ |
| 股票分钟 | ✓ | ✓ | ✓ | ✓ |
| 买卖盘 | ✓ | ✓ | — | — |
| 分笔 | ✓ | ✓ | — | — |
| 股票列表 / 基本信息 | ✓ | ✓ | ✓ | — |
| ETF 行情 / K 线 / 分钟 | ✓ | ✓ | ✓ | ✓ |
| ETF 列表 | ✓ | ✓ | ✓ | — |
| 可转债列表 / 行情 | ✓ | ✓ | ✓ | — |
| 指数行情 | ✓ | ✓ | — | ✓ |
| 指数列表 / K 线 | ✓ | — | — | ✓ |
| 港美股行情 | ✓ | — | — | ✓ |
| 期货行情 | ✓ | ✓ | — | ✓ |
| 期货列表 | ✓ | 内置连续合约 | — | — |
| 期货 K 线 | ✓ | — | — | ✓ |
| 期权列表 / 行情 | ✓ | ETF 期权 | — | — |

### 板块 / 宏观 / 财报

| 能力 | EM | Sina | THS | TV |
|------|:--:|:----:|:---:|:--:|
| 行业 / 概念板块 | ✓ | — | ✓ | — |
| 板块 K 线 / 成分 | ✓ | — | ✓ | — |
| 板块代码映射 | ✓ | — | — | — |
| 涨跌停池 | ✓ | — | — | — |
| 宏观 CPI / GDP | ✓ | — | — | — |
| F10 财报 | ✓ | — | — | — |

### 新闻 / 研报

| 能力 | EM | Sina | THS | TV |
|------|:--:|:----:|:---:|:--:|
| 新闻列表 | ✓ | ✓ | ✓ | ✓ |
| 新闻正文 | ✓ | ✓ | ✓ | ✓ |
| 新闻搜索 | ✓ | ✓ | push 过滤 | 标题过滤 |
| 机构调研 | ✓ | — | — | — |
| 研报 | ✓ | ✓ | ✓ | — |
| 分析师共识 | — | — | — | ✓ |
| 业绩预告 | ✓ | — | — | — |

### 扩展（EastMoney）

| 能力 | EM | Sina | THS | TV |
|------|:--:|:----:|:---:|:--:|
| 资金流 | ✓ | — | — | — |
| 龙虎榜 | ✓ | — | — | — |
| 沪深港通 / 融资融券 | ✓ | — | — | — |
| IPO / 大宗交易 | ✓ | — | — | — |
| 估值 / 股东 / 分红 | ✓ | — | — | — |

### TradingView analytics

| 能力 | EM | Sina | THS | TV |
|------|:--:|:----:|:---:|:--:|
| Symbol 搜索 | — | — | — | ✓ |
| 技术分析 | — | — | — | ✓ |
| 分析师共识 | — | — | — | ✓ |
| 筛选器 / 热榜 | — | — | — | ✓ |
| 经济日历 | — | — | — | ✓ |
| 指标 search / spec / series | — | — | — | ✓ |
| 策略回测 | — | — | — | ✓ |
| K 线回放 | — | — | — | ✓ |
| 图表 drawings | — | — | — | ✓ |

TV WS / 新闻：`--proxy`。Drawings / pine perm：session cookies。策略回测：best-effort。

## API

### EastMoney

| 域名 | 端点 | 能力 |
|------|------|------|
| `push2.eastmoney.com` | `/api/qt/stock/get` 等 | 行情 |
| `push2his.eastmoney.com` | `/api/qt/stock/kline/get` | K 线 |
| `datacenter-web.eastmoney.com` | `RPT_*` | 宏观、F10、扩展 |
| `reportapi.eastmoney.com` | `/report/list` | 研报 |
| `newsapi.eastmoney.com` | `/kuaixun/v2/api/list` | 新闻列表 |
| `newsinfo.eastmoney.com` | `/kuaixun/v2/api/content` | 新闻正文 |
| `search-api-web.eastmoney.com` | JSONP | 新闻搜索 |

### Sina

| 域名 | 端点 | 能力 |
|------|------|------|
| `hq.sinajs.cn` | `/list=` | 行情 |
| `money.finance.sina.com.cn` | `CN_MarketData.getKLineData` | K 线 |
| `feed.mix.sina.com.cn` | `/api/roll/get` | 新闻列表 / 搜索 |
| `zhibo.sina.com.cn` | `/api/zhibo/feed` | 7×24 快讯 |
| 文章 `url` | HTML | 新闻正文 |
| `stock.finance.sina.com.cn` | `vReport_List`, `vReport_Show` | 研报（HTML） |

新闻 ID：`sina:roll:{docid}`、`sina:zhibo:{id}`。研报列表：`?t1=all&symbol=sh{code}`。

### THS

| 域名 | 端点 | 能力 |
|------|------|------|
| `d.10jqka.com.cn` | `/v6/line/` 等 | K 线、板块 |
| `q.10jqka.com.cn` | `/thshy/`、`/gn/` | 板块列表 |
| `news.10jqka.com.cn` | `/tapp/news/push/stock/` | 新闻列表 |
| | `/{date}/c{seq}.shtml` | 新闻正文 |
| `stockpage.10jqka.com.cn` | `/stock_page/api/v1/stockpage/reports` | 研报（`code`、`marketId`） |
| `data.10jqka.com.cn` | `/ipo/kzz/` | 可转债列表 |
| `hq.sinajs.cn` | 债券行情 | 可转债行情 |

### TradingView

| 域名 | 端点 | 能力 |
|------|------|------|
| `symbol-search.tradingview.com` | `/symbol_search/v3` | Symbol 搜索 |
| `scanner.tradingview.com` | `/{market}/scan` | 筛选器、热榜、TA、分析师共识 |
| `economic-calendar.tradingview.com` | `/events` | 经济日历 |
| `pine-facade.tradingview.com` | `/list`、`/translate/` | 指标、策略脚本 |
| `news-headlines.tradingview.com` | `/headlines/`、`/v2/story` | 新闻列表、正文 |
| `www.tradingview.com` | `/chart-token/` | Drawings JWT |
| `www.tradingview.com` | `/pine_perm/*` | 邀请制脚本 ACL |
| `data.tradingview.com` | WebSocket | 行情、K 线、指标、策略 |
| `charts-storage.tradingview.com` | `/get/layout/.../sources` | Drawings（`chart_id` 1 / 2 / `_shared`） |

新闻 ID：`tv:{headline_id}`。策略：WS `create_study` + `StrategyScript@tv-scripting-101!`。

环境变量：`TENK_TV_SESSION`、`TENK_TV_SIGNATURE`、`TENK_TV_AUTH_TOKEN`、`TENK_PROXY`。Session cookies 经 homepage 解析 WS token（含 geo 重定向）。

## ClientBuilder

| Provider | Traits |
|----------|--------|
| EastMoney | stock、fund、bond、news、extended market |
| Sina | stock、fund、bond、index quote、futures quote、news、research reports |
| THS | stock、fund、bond、board、news、research reports |
| TradingView | market（WS）、search、TA、analyst、screener、calendar、study、news |

默认：`SourceKind::DEFAULT`（与 `SourceKind::ALL` 相同）。MCP 使用 `SourceKind::ALL`。

## CLI 命令 ↔ Source

| 命令 | Source |
|------|--------|
| `stock quote/kline/minute/orderbook/ticks` | EM、Sina、THS、TV |
| `stock search`、`stock ta`、`stock analyst` | TV |
| `news list/read/search` | EM、Sina、THS、TV |
| `market research`、`market forecast`、扩展行情 | EM |
| `market report` | EM、Sina、THS |
| `board`、`index`、`pool`、`macro cpi/gdp`、`financial` | EM |
| `market screener/hotlist/indicator*/strategy/replay/drawings` | TV |
| `macro calendar` | TV |
| `global hk/us` | EM、TV |
| `futures quote/kline` | EM、Sina、TV |
