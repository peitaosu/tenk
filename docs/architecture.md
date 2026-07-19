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
| `client` | `DataClient` — unified API with multi-source fallback |
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

## Request flow

```
Application (library / CLI / MCP)
        │
        ▼
   DataClient::get_*()
        │
        ▼
   try_sources_* macro
   (priority order, skip unavailable,
    retry on recoverable errors)
        │
        ▼
   Source trait impl
   (EastMoney / Sina / THS)
        │
        ▼
   RequestManager → HTTP API
```

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

- **Trait-based sources** — each provider implements focused traits; no monolithic interface.
- **Fallback chains** — `DataClient` tries configured sources in priority order.
- **Recoverable errors** — network, parse, and rate-limit errors trigger the next source; config and invalid-input errors stop immediately.
- **Builder defaults** — `ClientBuilder` registers the right trait impls per provider so callers rarely wire sources manually.
