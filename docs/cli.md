# CLI

Binary name: `tenk`. Built from the `tenk-cli` crate.

**Default:** `tenk` with no subcommand launches the [TUI](tui.md). Subcommands and `--mcp` behave as before.

## Global flags

| Flag | Description |
|------|-------------|
| `-f, --format` | `table` (default), `json`, `csv` |
| `-o, --output` | Write to file |
| `-s, --source` | `eastmoney`, `sina`, `ths`, `tradingview` (repeatable; default: all four) |
| `-L, --lang` | `en` (default), `zh-CN` |
| `--proxy` | HTTP proxy URL |
| `-v, --verbose` | Debug logging |
| `--mcp` | Run as MCP server |

`-L en`: reads `TENK_LANG` env when set.

## Commands

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

Source map:

| Commands | Source |
|----------|--------|
| `news list/read/search` | EastMoney, Sina, THS, TV |
| `market research`, `market forecast` | EastMoney |
| `market report` | EastMoney, Sina, THS |
| `stock search`, `stock ta`, `stock analyst`, `market screener/hotlist/indicator*/strategy/replay/drawings`, `macro calendar` | TV (`--proxy …` for WS) |
| `index`, `board`, `pool`, `financial`, `macro cpi/gdp` | EastMoney |
| Index/futures quotes | Sina |
| Boards | THS |

## Examples

```bash
tenk stock quote 600519 300059
tenk stock kline 600519 -k weekly --start 20250101 --end 20250131
tenk index quote 000001
tenk board list -t concept -l 10
tenk board members BK0428 -l 20
tenk board resolve BK0428
tenk futures quote ZN0 113.zns
tenk options list -e sse -l 10
tenk financial statement 600519 -k income -l 4
tenk macro cpi -l 12
tenk global hk 00700
tenk pool limit -t limit-up -l 20
tenk news search 茅台 -l 5
tenk stock analyst 600519 --proxy http://127.0.0.1:7890
tenk -s eastmoney stock quote 600519
```

## Output

- **Table** — formatted with `comfy-table`; headers localized via `rust-i18n`.
- **JSON** — `serde_json` pretty-print.
- **CSV** — UTF-8 BOM prefix for Excel compatibility.

Formatting helpers live in `output.rs`: `format_change_pct`, `change_pct_cell`, `price_cell`.

Shared CLI/MCP enums and parsers: `tenk-cli/src/args.rs`.

## i18n

Locale files: `tenk-cli/locales/en.yaml`, `tenk-cli/locales/zh-CN.yaml`.

Volume and amount labels use `format_volume_i18n` / `format_amount_i18n` (万 / 亿 units).

## Client wiring

`client::build_client()` maps CLI `--source` flags to `SourceKind` and passes `--proxy` to `ClientBuilder`.

Extended command examples: [tenk-cli/README.md](../tenk-cli/README.md).
