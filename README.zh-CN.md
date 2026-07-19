# 10K (tenk)

[English](README.md) | 简体中文

> 大盘还要上去，要涨到一万点！

多数据源 A 股、ETF、可转债行情库与命令行工具。

## 文档

| 文档 | 说明 |
|------|------|
| [架构](docs/zh-CN/architecture.md) | 工作区结构、模块划分、请求流程 |
| [库](docs/zh-CN/library.md) | `ClientBuilder`、`DataClient`、回退机制 |
| [数据源](docs/zh-CN/sources.md) | 东方财富、新浪、同花顺 — 能力矩阵 |
| [数据类型](docs/zh-CN/data-types.md) | 核心枚举与结构体 |
| [命令行](docs/zh-CN/cli.md) | CLI 用法 |
| [MCP 服务](docs/zh-CN/mcp.md) | Model Context Protocol 集成 |
| [开发](docs/zh-CN/development.md) | 构建、测试、示例 |

English: [architecture](docs/architecture.md) · [library](docs/library.md) · [sources](docs/sources.md) · [CLI](docs/cli.md) · [MCP](docs/mcp.md) · [development](docs/development.md)

## 构建

```bash
cargo build --release
```

## 快速开始

**库**

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

**命令行**

```bash
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

Cursor 配置见 [docs/zh-CN/mcp.md](docs/zh-CN/mcp.md)。

## 开源许可

MIT
