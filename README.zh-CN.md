# 10K (tenk)

[English](README.md) | 简体中文

> 大盘还要上去，要涨到一万点！

多数据源行情数据库和命令行工具，支持 A 股、ETF 和可转债。

## 构建

```bash
cargo build --release
```

## 库

```toml
[dependencies]
tenk = "0.1"
```

```rust
use tenk::sources::EastMoneySource;
use tenk::DataClient;

let client = DataClient::new().with_source(EastMoneySource::default());
let prices = client.get_market_current(&["600519"]).await?;
```

📖 [库文档](tenk/README.md)

## 命令行

```bash
# 股票行情
tenk stock quote 600519

# K线数据
tenk stock kline 600519 -l 10

# ETF行情
tenk etf quote 510300

# 可转债涨幅榜
tenk bond quote --top-gainers 10

# 输出为 JSON/CSV
tenk stock quote 600519 -f json
tenk stock list -f csv > stocks.csv
```

📖 [命令行文档](tenk-cli/README.md)

## MCP 服务

作为 MCP 服务运行，供 AI 助手调用：

```bash
tenk --mcp
```

添加到 Cursor `mcp.json`：
```json
{
  "mcpServers": {
    "tenk": {
      "command": "/path/to/tenk",
      "args": ["--mcp"]
    }
  }
}
```

📖 [MCP 工具列表](tenk-cli/README.md#available-mcp-tools)

## 开源许可

MIT

