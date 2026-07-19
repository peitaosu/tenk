# MCP Server

Run tenk as a [Model Context Protocol](https://modelcontextprotocol.io) server for AI assistants.

## Start

```bash
tenk --mcp
```

stdio transport. Server name: `tenk-mcp`. Version from crate metadata.

## Cursor configuration

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

## Environment

| Variable | Description |
|----------|-------------|
| `TENK_PROXY` | HTTP proxy for the underlying `DataClient` |

## Implementation

- Entry: `tenk-cli/src/mcp.rs`
- Framework: `rmcp` 2.x with `#[tool_router]` / `#[tool_handler]`
- Client: shared `Arc<DataClient>` via `client::default_client()`
- Shared parsers: `tenk-cli/src/args.rs`

Each tool handler calls `DataClient` methods and returns JSON text in a `CallToolResult`.

## Tools

| Tool | Description |
|------|-------------|
| `stock_quote` | Current stock quotes |
| `stock_kline` | Historical K-line |
| `stock_minute` | Intraday minute data |
| `stock_orderbook` | Order book |
| `stock_ticks` | Tick trades |
| `stock_info` | Stock details |
| `stock_list` | All stock codes |
| `stock_valuation` | Valuation metrics |
| `stock_holders` | Top shareholders |
| `stock_funds` | Fund holdings |
| `stock_dividends` | Dividend history |
| `etf_quote` | ETF quotes |
| `etf_kline` | ETF K-line |
| `etf_minute` | ETF minute data |
| `etf_list` | All ETF codes |
| `bond_quote` | Bond quotes (filters: top gainers/losers/volume) |
| `bond_list` | Convertible bond list |
| `news_list` | News by category |
| `news_search` | Search news by keyword |
| `news_read` | Full article by ID |
| `capital_flow` | Real-time capital flow |
| `capital_flow_history` | Historical capital flow |
| `billboard_list` | Dragon-tiger board list |
| `billboard_detail` | Board detail for a stock |
| `earnings_forecast` | Earnings forecasts |
| `stock_connect` | Stock Connect data |
| `margin_trading` | Margin trading history |
| `ipo_list` | IPO list |
| `block_trades` | Block trade list |
| `institutional_research` | Institutional research |
| `research_reports` | Analyst reports |
| `index_list` | Index codes |
| `index_quote` | Index quotes |
| `index_kline` | Index K-line |
| `board_list` | Industry/concept boards |
| `board_kline` | Board K-line |
| `board_members` | Board constituents |
| `board_crosswalk` | EastMoney ↔ THS board mapping |
| `board_resolve` | Resolve THS code from EastMoney board |
| `futures_list` | Futures contracts |
| `futures_quote` | Futures quotes |
| `futures_kline` | Futures K-line |
| `options_list` | Options list |
| `options_quote` | Options quotes |
| `financial_statement` | Balance/income/cashflow/performance |
| `macro_cpi` | CPI data |
| `macro_gdp` | GDP data |
| `global_hk` | Hong Kong quotes |
| `global_us` | US quotes |
| `limit_pool` | Limit-up/down pools |

Tool parameters use `schemars` JSON Schema for MCP discovery.
