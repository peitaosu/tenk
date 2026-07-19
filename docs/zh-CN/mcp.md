# MCP 服务

以 [Model Context Protocol](https://modelcontextprotocol.io) 方式运行，供 AI 助手调用。

## 启动

```bash
tenk --mcp
```

stdio 传输。服务名：`tenk-mcp`。版本取自 crate 元数据。

## Cursor 配置

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

## 环境变量

| 变量 | 说明 |
|------|------|
| `TENK_PROXY` | 底层 `DataClient` 的 HTTP 代理 |

## 实现

- 入口：`tenk-cli/src/mcp.rs`
- 框架：`rmcp` 2.x，`#[tool_router]` / `#[tool_handler]`
- 客户端：`client::default_client()` 提供的共享 `Arc<DataClient>`
- 共享解析：`tenk-cli/src/args.rs`

各工具 handler 调用 `DataClient` 方法，以 JSON 文本返回 `CallToolResult`。

## 工具列表

| 工具 | 说明 |
|------|------|
| `stock_quote` | 股票实时行情 |
| `stock_kline` | 历史 K 线 |
| `stock_minute` | 分时数据 |
| `stock_orderbook` | 买卖盘口 |
| `stock_ticks` | 逐笔成交 |
| `stock_info` | 股票详情 |
| `stock_list` | 股票代码列表 |
| `stock_valuation` | 估值指标 |
| `stock_holders` | 十大股东 |
| `stock_funds` | 基金持仓 |
| `stock_dividends` | 分红历史 |
| `etf_quote` | ETF 行情 |
| `etf_kline` | ETF K 线 |
| `etf_minute` | ETF 分时 |
| `etf_list` | ETF 代码列表 |
| `bond_quote` | 可转债行情 |
| `bond_list` | 可转债列表 |
| `news_list` | 分类新闻 |
| `news_search` | 关键词搜索 |
| `news_read` | 新闻正文 |
| `capital_flow` | 实时资金流 |
| `capital_flow_history` | 历史资金流 |
| `billboard_list` | 龙虎榜列表 |
| `billboard_detail` | 个股龙虎榜明细 |
| `earnings_forecast` | 业绩预告 |
| `stock_connect` | 沪深港通 |
| `margin_trading` | 融资融券历史 |
| `ipo_list` | 新股列表 |
| `block_trades` | 大宗交易 |
| `institutional_research` | 机构调研 |
| `research_reports` | 研报 |
| `index_list` | 指数列表 |
| `index_quote` | 指数行情 |
| `index_kline` | 指数 K 线 |
| `board_list` | 行业/概念板块 |
| `board_kline` | 板块 K 线 |
| `board_members` | 板块成分股 |
| `board_crosswalk` | 东财 ↔ 同花顺板块映射 |
| `board_resolve` | 东财板块解析为同花顺代码 |
| `futures_list` | 期货合约列表 |
| `futures_quote` | 期货行情 |
| `futures_kline` | 期货 K 线 |
| `options_list` | 期权列表 |
| `options_quote` | 期权行情 |
| `financial_statement` | 财报（三表 + 业绩摘要） |
| `macro_cpi` | CPI |
| `macro_gdp` | GDP |
| `global_hk` | 港股行情 |
| `global_us` | 美股行情 |
| `limit_pool` | 涨跌停池 |

工具参数通过 `schemars` 生成 JSON Schema，供 MCP 发现。
