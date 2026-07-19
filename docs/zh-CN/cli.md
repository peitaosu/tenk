# 命令行

二进制名：`tenk`，由 `tenk-cli` crate 构建。

## 全局参数

| 参数 | 说明 |
|------|------|
| `-f, --format` | `table`（默认）、`json`、`csv` |
| `-o, --output` | 输出到文件 |
| `-s, --source` | `eastmoney`、`sina`、`ths`（可重复，默认全部） |
| `-L, --lang` | `en`（默认）、`zh-CN` |
| `--proxy` | HTTP 代理 URL |
| `-v, --verbose` | 调试日志 |
| `--mcp` | 以 MCP 服务模式运行 |

语言回退：`-L en` 时若设置了 `TENK_LANG` 环境变量则使用该值。

## 命令

```
tenk stock     quote | kline | minute | orderbook | ticks | info | valuation | holders | funds | dividend | list
tenk etf       quote | kline | minute | list
tenk bond      quote | list
tenk market    flow | flow-history | billboard | billboard-detail | forecast | connect | margin | ipo | block | research | report
tenk news      list | search | read
tenk index     list | quote | kline
tenk board     list | kline | members | crosswalk | resolve
tenk futures   list | quote | kline
tenk options   list | quote
tenk financial statement
tenk macro     cpi | gdp
tenk global    hk | us
tenk pool      limit
```

扩展命令（index、board、futures 等）通常需要 EastMoney；新浪提供指数/期货行情；同花顺提供板块列表、K 线、成分股。

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
