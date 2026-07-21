# Data Sources

Providers: EastMoney, Sina, THS, TradingView. Each implements a subset of `tenk::traits`.

Priority: `DataSource::priority()` — lower value first. TradingView = 4.

| Provider | Portal | Notes |
|----------|--------|-------|
| EastMoney | [eastmoney.com](https://www.eastmoney.com) | CN market + extended |
| Sina | [finance.sina.com.cn](https://finance.sina.com.cn) | CN market + news + reports |
| THS | [10jqka.com.cn](https://www.10jqka.com.cn) | CN boards + news + research reports |
| TradingView | [tradingview.com](https://www.tradingview.com) | Global analytics; WS/news need `--proxy` |

## Capability matrix

✓ = implemented. — = not implemented.

### Market

| Feature | EM | Sina | THS | TV |
|---------|:--:|:----:|:---:|:--:|
| Stock quote | ✓ | ✓ | ✓ | ✓ |
| Stock kline (daily / weekly / monthly) | ✓ | ✓ | ✓ | ✓ |
| Stock kline (5m / 15m) | ✓ | ✓ | — | ✓ |
| Stock kline (30m / 60m) | ✓ | ✓ | ✓ | ✓ |
| Stock minute | ✓ | ✓ | ✓ | ✓ |
| Stock order book | ✓ | ✓ | — | — |
| Stock ticks | ✓ | ✓ | — | — |
| Stock list | ✓ | ✓ | ✓ | — |
| Stock info | ✓ | ✓ | ✓ | — |
| ETF quote / kline / minute | ✓ | ✓ | ✓ | ✓ |
| ETF list | ✓ | ✓ | ✓ | — |
| Bond list / quote | ✓ | ✓ | ✓ | — |
| Index quote | ✓ | ✓ | — | ✓ |
| Index list / kline | ✓ | — | — | ✓ |
| HK / US quote | ✓ | — | — | ✓ |
| Futures quote | ✓ | ✓ | — | ✓ |
| Futures list | ✓ | curated | — | — |
| Futures kline | ✓ | — | — | ✓ |
| Options list / quote | ✓ | ETF options | — | — |

### Boards / macro / F10

| Feature | EM | Sina | THS | TV |
|---------|:--:|:----:|:---:|:--:|
| Industry / concept boards | ✓ | — | ✓ | — |
| Board kline | ✓ | — | ✓ | — |
| Board constituents | ✓ | — | ✓ | — |
| Board crosswalk | ✓ | — | — | — |
| Limit-up / limit-down pools | ✓ | — | — | — |
| Macro CPI / GDP | ✓ | — | — | — |
| F10 financial statements | ✓ | — | — | — |

### News / research

| Feature | EM | Sina | THS | TV |
|---------|:--:|:----:|:---:|:--:|
| News list | ✓ | ✓ | ✓ | ✓ |
| News content | ✓ | ✓ | ✓ | ✓ |
| News search | ✓ | ✓ | push filter | filter |
| Institutional research | ✓ | — | — | — |
| Analyst reports | ✓ | ✓ | ✓ | — |
| Analyst consensus | — | — | — | ✓ |
| Earnings forecast | ✓ | — | — | — |

### Extended (EastMoney)

| Feature | EM | Sina | THS | TV |
|---------|:--:|:----:|:---:|:--:|
| Capital flow / history | ✓ | — | — | — |
| Billboard | ✓ | — | — | — |
| Stock Connect | ✓ | — | — | — |
| Margin trading | ✓ | — | — | — |
| IPO list | ✓ | — | — | — |
| Block trades | ✓ | — | — | — |
| Valuation | ✓ | — | — | — |
| Top holders / fund holdings | ✓ | — | — | — |
| Dividends | ✓ | — | — | — |

### TradingView analytics

| Feature | EM | Sina | THS | TV |
|---------|:--:|:----:|:---:|:--:|
| Symbol search | — | — | — | ✓ |
| Technical analysis | — | — | — | ✓ |
| Analyst consensus | — | — | — | ✓ |
| Market screener | — | — | — | ✓ |
| Market hotlist | — | — | — | ✓ |
| Economic calendar | — | — | — | ✓ |
| Indicator search / spec | — | — | — | ✓ |
| Indicator series | — | — | — | ✓ |
| Strategy backtest | — | — | — | ✓ |
| Chart replay | — | — | — | ✓ |
| Chart drawings | — | — | — | ✓ |

TV WS / news: `--proxy`. Drawings / pine perm: session cookies. Strategy backtest: best-effort.

## APIs

### EastMoney

| Domain | Endpoint | Feature |
|--------|----------|---------|
| `push2.eastmoney.com` | `/api/qt/stock/get`, `clist/get`, `trends2/get` | Quote, list, minute |
| `push2his.eastmoney.com` | `/api/qt/stock/kline/get` | K-line |
| `push2ex.eastmoney.com` | limit pool APIs | Limit pools |
| `datacenter-web.eastmoney.com` | `RPT_*` | Macro, F10, extended |
| `reportapi.eastmoney.com` | `/report/list` | Analyst reports |
| `newsapi.eastmoney.com` | `/kuaixun/v2/api/list` | News list |
| `newsinfo.eastmoney.com` | `/kuaixun/v2/api/content` | News content |
| `search-api-web.eastmoney.com` | search JSONP | News search |

### Sina

| Domain | Endpoint | Feature |
|--------|----------|---------|
| `hq.sinajs.cn` | `/list=`, `nf_zn0` | Quote |
| `money.finance.sina.com.cn` | `CN_MarketData.getKLineData` | K-line |
| `cn.finance.sina.com.cn` | `getMinlineData` | Minute |
| `vip.stock.finance.sina.com.cn` | `CN_TransListV2.php`, `Market_Center.*` | Ticks, lists |
| `feed.mix.sina.com.cn` | `/api/roll/get` | News list / search (`lid`, `k`) |
| `zhibo.sina.com.cn` | `/api/zhibo/feed` | 7×24 flash (`zhibo_id=152`) |
| article `url` | HTML | News content |
| `stock.finance.sina.com.cn` | `vReport_List`, `vReport_Show` | Analyst reports (HTML) |

News IDs: `sina:roll:{docid}`, `sina:zhibo:{id}`. Report list: `?t1=all&symbol=sh{code}`.

### THS

| Domain | Endpoint | Feature |
|--------|----------|---------|
| `d.10jqka.com.cn` | `/v6/line/`, `/v4/line/bk_`, `/v2/blockrank/` | K-line, boards |
| `q.10jqka.com.cn` | `/thshy/`, `/gn/` | Board lists |
| `news.10jqka.com.cn` | `/tapp/news/push/stock/` | News list |
| | `/{date}/c{seq}.shtml` | News content |
| `stockpage.10jqka.com.cn` | `/stock_page/api/v1/stockpage/reports` | Analyst reports (`code`, `marketId`) |
| `data.10jqka.com.cn` | `/ipo/kzz/` | Bond list |
| `hq.sinajs.cn` | bond quotes | Bond quote |

### TradingView

| Domain | Endpoint | Feature |
|--------|----------|---------|
| `symbol-search.tradingview.com` | `/symbol_search/v3` | Symbol search |
| `scanner.tradingview.com` | `/{market}/scan` | Screener, hotlist, TA, analyst consensus |
| `economic-calendar.tradingview.com` | `/events` | Calendar |
| `pine-facade.tradingview.com` | `/list`, `/translate/` | Indicators, strategy script |
| `news-headlines.tradingview.com` | `/headlines/`, `/v2/story` | News list, content |
| `www.tradingview.com` | `/chart-token/` | Layout JWT for drawings |
| `www.tradingview.com` | `/pine_perm/*` | Invite-only script ACL |
| `data.tradingview.com` | WebSocket | Quote, kline, study, strategy |
| `charts-storage.tradingview.com` | `/get/layout/.../sources` | Drawings (`chart_id` 1 / 2 / `_shared`) |

News IDs: `tv:{headline_id}`. Strategy: WS `create_study` + `StrategyScript@tv-scripting-101!`.

Env: `TENK_TV_SESSION`, `TENK_TV_SIGNATURE`, `TENK_TV_AUTH_TOKEN`, `TENK_PROXY`. Session cookies resolve the WS token from the homepage (geo redirects).

## ClientBuilder

| Provider | Traits |
|----------|--------|
| EastMoney | stock, fund, bond, news, extended market |
| Sina | stock, fund, bond, index quote, futures quote, news, research reports |
| THS | stock, fund, bond, board, news, research reports |
| TradingView | market (WS), search, TA, analyst, screener, calendar, study, news |

Default: `SourceKind::DEFAULT` (`SourceKind::ALL` is identical). MCP uses `SourceKind::ALL`.

## CLI source map

| Commands | Source |
|----------|--------|
| `stock quote/kline/minute/orderbook/ticks` | EM, Sina, THS, TV |
| `stock search`, `stock ta`, `stock analyst` | TV |
| `news list/read/search` | EM, Sina, THS, TV |
| `market research`, `market forecast`, extended market | EM |
| `market report` | EM, Sina, THS |
| `board`, `index`, `pool`, `macro cpi/gdp`, `financial` | EM |
| `market screener/hotlist/indicator*/strategy/replay/drawings` | TV |
| `macro calendar` | TV |
| `global hk/us` | EM, TV |
| `futures quote/kline` | EM, Sina, TV |
