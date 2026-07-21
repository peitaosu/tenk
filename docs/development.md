# Development

## Requirements

- Rust 1.80+ (edition 2024)
- Network access for integration tests against live APIs

## Build

```bash
cargo build              # debug
cargo build --release    # release
cargo build --examples   # library examples
```

## Test

```bash
cargo test                        # all crates
cargo test -p tenk                # library only
cargo test -p tenk-cli            # CLI only
```

Tests cover parsers, enums, builder wiring, client dispatch, and CLI formatting. Live API calls are not required for unit tests.

## Examples

```bash
cargo run --example quick_start
cargo run --example stock_data
cargo run --example etf_data
cargo run --example bond_data
```

## Project layout

```
tenk/src/
  builder.rs          ClientBuilder
  client.rs           DataClient + try_sources macros
  traits.rs           Source trait definitions
  request.rs          HTTP layer
  error.rs            Error types
  data/               Domain types
  sources/
    eastmoney/        EastMoney provider
    sina.rs           Sina provider
    ths.rs            THS provider
    tradingview/      TradingView provider (pine_perm.rs, analyst.rs, …)
  util/               Shared helpers

tenk-cli/src/
  main.rs             CLI entry
  client.rs           CLI → DataClient
  commands/           Subcommand handlers
  output.rs           Formatting
  i18n.rs             Localization
  mcp.rs              MCP server
  locales/            Translation files
```

## Key dependencies

| Crate | Version | Used for |
|-------|---------|----------|
| `tokio` | 1.53 | Async runtime |
| `reqwest` | 0.13 | HTTP (rustls) |
| `rmcp` | 2.2 | MCP server |
| `clap` | 4.5 | CLI parsing |
| `rust-i18n` | 4 | CLI localization |
| `strum` | 0.27 | Enum parsing (`KLineType`) |

## Conventions

- Sources implement traits; never call HTTP from `DataClient` directly.
- Use `ClientBuilder` in examples and application code.
- Date strings: `YYYY-MM-DD` at API boundaries.
- Stock codes: bare numeric strings (`600519`, not `sh600519`) unless noted.

## Docs

- English: `docs/`
- 简体中文: `docs/zh-CN/`

Crate READMEs (`tenk/README.md`, `tenk-cli/README.md`) are brief pointers to this directory.
