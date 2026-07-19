# Data Sources

Three providers: **EastMoney**, **Sina**, **THS**. Each implements a subset of traits in `tenk::traits`.

Priority is set per source (`priority()` on `DataSource`). Lower value = tried first.

## Websites

| Provider | Portal | Data center |
|----------|--------|-------------|
| EastMoney | [eastmoney.com](https://www.eastmoney.com) | [data.eastmoney.com](https://data.eastmoney.com) |
| Sina | [finance.sina.com.cn](https://finance.sina.com.cn) | [vip.stock.finance.sina.com.cn](https://vip.stock.finance.sina.com.cn) |
| THS | [10jqka.com.cn](https://www.10jqka.com.cn) | [data.10jqka.com.cn](https://data.10jqka.com.cn) |

## Capability matrix

| Capability | EastMoney | Sina | THS |
|------------|:---------:|:----:|:---:|
| Stock K-line | ✓ | ✓ | ✓ (5m/15m via Sina fallback) |
| Stock quote (current) | ✓ | ✓ | ✓ |
| Stock minute | ✓ | ✓ | ✓ |
| Stock list / info | ✓ | ✓ | ✓† |
| Order book / ticks | ✓ | ✓ | — |
| ETF list | ✓ | ✓ | ✓† |
| ETF K-line | ✓ | ✓ | partial |
| ETF quote | ✓ | ✓ | ✓ |
| ETF minute | ✓ | ✓ | ✓ |
| Bond list | ✓ | ✓ | ✓ |
| Bond quotes | ✓ | ✓ | ✓‡ |
| Index list / K-line / quote | ✓ | quote | — |
| Industry / concept boards | ✓ | — | ✓‡ |
| Board constituents | ✓ (`BK*`) | — | ✓ (`881*`/`885*`) |
| Board crosswalk | name + overlap | — | name + overlap |
| Limit-up / limit-down pools | ✓ | — | — |
| Macro CPI / GDP | ✓ | — | — |
| HK / US quotes | ✓ | — | — |
| Futures list / quote / K-line | ✓ | quote‡ | — |
| Options list / quote | ✓ | ETF options‡ | — |
| F10 financials (3 statements + summary) | ✓ | — | — |
| News list / content / search | ✓ | — | list + content‡ / filter‡ |
| Capital flow | ✓ | — | — |
| Billboard (龙虎榜) | ✓ | — | — |
| Earnings forecast | ✓ | — | — |
| Stock Connect | ✓ | — | — |
| Margin trading | ✓ | — | — |
| IPO / block trades | ✓ | — | — |
| Institutional research / reports | ✓ | — | — |
| Valuation / holders / dividends | ✓ | — | — |

Extended market features require EastMoney via `ClientBuilder` or `with_extended_market()`.

† THS stock/ETF lists use `q.10jqka.com.cn` board pages; may fail when anti-bot redirects block the request.

‡ Sina futures: `hq.sinajs.cn/list=nf_{symbol}` (continuous contracts). THS industry boards: `q.10jqka.com.cn/thshy/` (GBK); concept boards: `gnSection` JSON on `q.10jqka.com.cn/gn/`. THS news: push list + SSR HTML content; search filters push pages locally. THS board K-line uses THS codes (`881*`/`885*`), not EastMoney `BK*`.

## Verified free API endpoints

### EastMoney

| Domain | Endpoint / reportName | Used for |
|--------|----------------------|----------|
| `push2.eastmoney.com` | `/api/qt/stock/get` | Quotes (stock, index, HK/US, futures) |
| | `/api/qt/clist/get` | Lists (index, boards, futures, options) |
| `push2his.eastmoney.com` | `/api/qt/stock/kline/get` | K-line (stock, index, board, futures) |
| `push2ex.eastmoney.com` | `getTopicZTPool`, `getTopicDTPool`, … | Limit pools |
| `29.push2.eastmoney.com` | `/api/qt/clist/get?fs=b:BK1051` | Board constituents |
| `31.push2.eastmoney.com` | `/api/qt/clist/get?fs=m:10\|m:11\|m:12` | SSE / CFFEX / SZSE options |
| `datacenter-web.eastmoney.com` | `RPT_DMSK_FN_BALANCE` | Balance sheet |
| | `RPT_DMSK_FN_INCOME` | Income statement |
| | `RPT_DMSK_FN_CASHFLOW` | Cash flow |
| | `RPT_LICO_FN_CPD` | Performance summary |
| | `RPT_ECONOMY_CPI`, `RPT_ECONOMY_GDP` | Macro |

Futures `fs`: `m:113,m:114,m:115,m:8,m:142,m:225` (SHFE, DCE, CZCE, CFFEX, INE, GFEX). Secid: `{f13}.{f12}` e.g. `113.zns`.

Options: `m:10` SSE, `m:11` CFFEX (index options), `m:12` SZSE.

### Sina

| Domain | Endpoint | Used for |
|--------|----------|----------|
| `hq.sinajs.cn` | `/list=nf_{code}` | Futures quotes (continuous) |
| | `/list=CON_OP_{code}` | ETF option quotes |
| | `/list=OP_UP_{underlying}{YYMM}` | Call chain codes |
| `stock.finance.sina.com.cn` | `StockOptionService.getStockName` | Option expiry months |
| `money.finance.sina.com.cn` | `CN_MarketData.getKLineData` | K-line |
| `vip.stock.finance.sina.com.cn` | `Market_Center.getHQFuturesData` | Futures contract list (often empty) |

### THS

| Domain | Endpoint | Used for |
|--------|----------|----------|
| `news.10jqka.com.cn` | `/tapp/news/push/stock/` | News list |
| | `/{date}/c{seq}.shtml` | News content (SSR HTML) |
| `q.10jqka.com.cn` | `/thshy/`, `/gn/` | Industry/concept board lists |
| `d.10jqka.com.cn` | `/v4/line/bk_{code}/{period}/last.js` | Board K-line (~140 days) |
| | `/v2/blockrank/{code}/199112/d1000.js` | Board constituents |
| `data.10jqka.com.cn` | `/ipo/kzz/` | Bond list |
| `d.10jqka.com.cn` | `/v6/line/hs_{code}/{period}/last36000.js` | Stock K-line |

Requires `Referer` + `Cookie: v=1` on news requests.

## ClientBuilder wiring

| Provider | Traits registered |
|----------|-------------------|
| EastMoney | stock, fund, bond, news, extended market (index, boards, pools, macro, HK/US, futures, options, financials) |
| Sina | stock, fund, bond, index quotes, futures quotes |
| THS | stock, fund, bond, board list/K-line/constituents, news list/content/search |

## Library API (extended)

| Method | Description |
|--------|-------------|
| `get_index_list` / `get_index_current` / `get_index_market` | Index data |
| `get_industry_boards` / `get_concept_boards` / `get_board_market` | Boards |
| `get_board_constituents` | Board member stocks |
| `resolve_board_crosswalk` / `resolve_ths_board_for_eastmoney` | Board code mapping |
| `get_limit_pool` | Limit-up/down pools |
| `get_macro_cpi` / `get_macro_gdp` | Macro |
| `get_hk_current` / `get_us_current` | HK / US |
| `get_futures_list` / `get_futures_current` / `get_futures_market` | Futures |
| `get_options_list` / `get_options_current` | Options |
| `get_financial_statement` | Balance / income / cashflow / performance |

CLI: `tenk index`, `tenk board`, `tenk futures`, etc. MCP: matching `index_*`, `board_*`, `futures_*` tools.

## Limitations

| Gap | Notes |
|-----|-------|
| Cross-vendor board codes | No public BK ↔ 885 API; use `resolve_board_crosswalk` (name) or `resolve_ths_board_for_eastmoney` (constituents) |
| THS board K-line history | `last.js` returns ~140 recent bars only |
| THS news search | Filters push-list pages; not full-text search API |
| Sina futures list | `Market_Center.getHQFuturesData` often empty; library uses curated continuous symbols |
| Expired futures (Sina) | `nf_` returns empty for delisted contract codes |

## Adding a source

1. Implement relevant traits in `tenk/src/sources/`.
2. Export from `sources/mod.rs`.
3. Add a `SourceKind` variant and match arm in `builder.rs`.
4. Add CLI `Source` enum variant in `tenk-cli/src/main.rs` if exposed to users.
