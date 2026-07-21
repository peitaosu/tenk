# 10K (tenk)

English | [简体中文](README.zh-CN.md)

> The market index will raise to 10K!

Multi-source market data library and CLI for Chinese stocks, ETFs, and bonds.

## Documentation

| Doc | Description |
|-----|-------------|
| [Architecture](docs/architecture.md) | Workspace layout, modules, request flow |
| [Library](docs/library.md) | `ClientBuilder`, `DataClient`, API reference |
| [Data Sources](docs/sources.md) | EastMoney, Sina, THS, TradingView — capability matrix |
| [Data Types](docs/data-types.md) | Core enums and structs |
| [CLI](docs/cli.md) | Command-line interface |
| [TUI](docs/tui.md) | Terminal UI (default) |
| [MCP Server](docs/mcp.md) | Model Context Protocol integration |
| [Development](docs/development.md) | Build, test, examples |

中文：[架构](docs/zh-CN/architecture.md) · [库](docs/zh-CN/library.md) · [数据源](docs/zh-CN/sources.md) · [命令行](docs/zh-CN/cli.md) · [MCP](docs/zh-CN/mcp.md) · [开发](docs/zh-CN/development.md)

## Build

```bash
cargo build --release
```

## Quick start

**Library**

```toml
[dependencies]
tenk = "0.2"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

```rust
use tenk::ClientBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientBuilder::new().build()?;
    let prices = client.get_market_current(&["600519"]).await?;
    Ok(())
}
```

**CLI**

```bash
tenk
tenk stock quote 600519
tenk stock kline 600519 -l 10
tenk etf quote 510300
tenk bond quote --top-gainers 10
tenk stock quote 600519 -f json
```

**MCP**

```bash
tenk --mcp
```

See [docs/mcp.md](docs/mcp.md) for Cursor configuration.

## License

MIT
