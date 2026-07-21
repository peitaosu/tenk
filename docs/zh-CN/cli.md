# 命令行

二进制名：`tenk`，由 `tenk-cli` crate 构建。

**默认：** 无子命令运行 `tenk` 启动 [TUI](tui.md)。子命令与 `--mcp` 不变。

## 全局参数

| 参数 | 说明 |
|------|------|
| `-f, --format` | `table`（默认）、`json`、`csv` |
| `-o, --output` | 输出到文件 |
| `-s, --source` | `eastmoney`、`sina`、`ths`、`tradingview`（可重复；默认四项全开） |
| `-L, --lang` | `en`（默认）、`zh-CN` |
| `--proxy` | HTTP 代理 URL |
| `-v, --verbose` | 调试日志 |
| `--mcp` | 以 MCP 服务模式运行 |

`-L en`：若设置 `TENK_LANG` 则使用该值。

## 命令

```
tenk stock     quote | kline | minute | orderbook | ticks | info | valuation | holders | funds | dividend | list | search | ta | analyst
tenk etf       quote | kline | minute | list
tenk bond      quote | list
tenk market    flow | flow-history | billboard | billboard-detail | forecast | connect | margin | ipo | block | research | report | screener | hotlist | indicator-search | indicator | indicator-series | strategy | replay | drawings
tenk news      list | search | read
tenk index     list | quote | kline
tenk board     list | kline | members | crosswalk | resolve
tenk futures   list | quote | kline
tenk options   list | quote
tenk financial statement
tenk macro     cpi | gdp | calendar
tenk global    hk | us
tenk pool      limit
```

| 命令 | Source |
|------|--------|
| `news list/read/search` | EastMoney、Sina、THS、TV |
| `market research`、`market forecast` | EastMoney |
| `market report` | EastMoney、Sina、THS |
| `stock search`、`stock ta`、`stock analyst`、`market screener/hotlist/indicator*/strategy/replay/drawings`、`macro calendar` | TV（WS 需 `--proxy …`） |
| `index`、`board`、`pool`、`financial`、`macro cpi/gdp` | EastMoney |
| 指数/期货行情 | Sina |
| 板块 | THS |

## 示例

```bash
tenk stock quote 600519 300059
tenk index quote 000001
tenk board list -t concept -l 10
tenk board resolve BK0428
tenk futures quote ZN0
tenk financial statement 600519 -k income -l 4
tenk pool limit -t limit-up -l 20
tenk news search 茅台 -l 5
tenk -s eastmoney stock quote 600519
```

## 输出

- **Table** — `comfy-table` 格式化；表头通过 `rust-i18n` 本地化。
- **JSON** — `serde_json` 美化输出。
- **CSV** — UTF-8 BOM，便于 Excel 打开。

共享枚举与解析：`tenk-cli/src/args.rs`。

## 国际化

语言文件：`tenk-cli/locales/en.yaml`、`tenk-cli/locales/zh-CN.yaml`。

## 客户端装配

`client::build_client()` 将 `--source` 映射为 `SourceKind`，并将 `--proxy` 传给 `ClientBuilder`。

更多示例：[tenk-cli/README.md](../../tenk-cli/README.md)。
