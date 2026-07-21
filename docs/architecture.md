# Architecture

## Workspace

```
tenk/                 # Library crate
tenk-cli/             # Binary: CLI + MCP server
```

Shared dependencies live in the root `Cargo.toml` under `[workspace.dependencies]`.

## Library modules

| Module | Role |
|--------|------|
| `builder` | `ClientBuilder`, `SourceKind` — wires default sources |
| `client` | `DataClient` — unified API, multi-source by priority |
| `data` | Domain types: stock, fund, bond, news, related |
| `traits` | Source trait definitions |
| `sources` | Provider implementations |
| `request` | HTTP client, retries, proxy |
| `error` | `DataError`, `DataResult` |
| `util` | JSONP parsing, date helpers |

## CLI modules

| Module | Role |
|--------|------|
| `main` | Clap entry, global flags, command dispatch |
| `client` | Builds `DataClient` from CLI flags |
| `commands/` | `stock`, `etf`, `bond`, `market`, `news` handlers |
| `output` | Table, JSON, CSV formatting |
| `i18n` | Locale selection (`en`, `zh-CN`) |
| `mcp` | MCP server and tool handlers |
| `tui` | Terminal UI — ratatui 0.30, chandelier charts |

## Request flow

```
Application (library / CLI / MCP)
        │
        ▼
   DataClient::get_*()
        │
        ▼
   try_sources_* macro
   (priority order, skip unavailable sources)
        │
        ▼
   Source trait impl
   (EastMoney / Sina / THS / TradingView)
        │
        ▼
   RequestManager → HTTP API
   TradingView WS → chart / quote / study
```

## TradingView layout

Priority 4. Included in `SourceKind::DEFAULT`.

```
sources/tradingview/
  mod.rs       # TradingViewSource facade (REST + WS)
  rest.rs      # symbol search, TA, screener, calendar, indicators, analyst snapshot
  ws.rs        # quotes, chart bars, studies, replay, analyst estimates (HTTP CONNECT proxy)
  market.rs    # StockMarketSource, Fund/Index/Global/Futures traits
  study.rs     # TechnicalAnalysis, SymbolSearch, Screener, Calendar, Study traits
  analyst.rs   # AnalystSource (scanner + WS estimates)
  pine_perm.rs # Invite-only Pine script ACL
  convert.rs   # TvQuote/TvChartBar → domain types
  protocol.rs  # WS packet format + JSON helpers
  symbol.rs    # A-share / pass-through symbol mapping
```

`ClientBuilder` → `with_tradingview_capabilities()`:

| Trait bucket | CLI / library entry |
|--------------|---------------------|
| `StockMarketSource` | `tenk stock quote/kline/minute` |
| `GlobalMarketSource` | `tenk global hk/us` |
| `TechnicalAnalysisSource` | `tenk stock ta` |
| `AnalystSource` | `tenk stock analyst` |
| `SymbolSearchSource` | `tenk stock search` |
| `ScreenerSource` | `tenk market screener/hotlist` |
| `EconomicCalendarSource` | `tenk macro calendar` |
| `StudySource` | `tenk market indicator*/strategy/replay/drawings` |

Env: `TENK_TV_SESSION`, `TENK_TV_SIGNATURE`, `TENK_TV_AUTH_TOKEN`. Session cookies resolve the WS token from the homepage (geo redirects). Proxy: `--proxy`, `TENK_PROXY`.

## EastMoney layout

```
sources/eastmoney/
  mod.rs       # struct, API response types
  stock.rs     # stock + ETF market traits
  bond.rs      # bond traits
  news.rs      # news traits
  extended.rs  # capital flow, billboard, valuation, etc.
```

## Design principles

- Trait-based sources per provider
- Multi-source dispatch by `priority()`
- Default sources: `SourceKind::DEFAULT` (all four providers). MCP: `SourceKind::ALL`
