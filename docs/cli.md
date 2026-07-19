# CLI

Binary name: `tenk`. Built from the `tenk-cli` crate.

## Global flags

| Flag | Description |
|------|-------------|
| `-f, --format` | `table` (default), `json`, `csv` |
| `-o, --output` | Write to file |
| `-s, --source` | `eastmoney`, `sina`, `ths` (repeatable, default: all) |
| `-L, --lang` | `en` (default), `zh-CN` |
| `--proxy` | HTTP proxy URL |
| `-v, --verbose` | Debug logging |
| `--mcp` | Run as MCP server |

Language fallback: when `-L en`, the `TENK_LANG` environment variable is used if set.

## Commands

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

Extended commands (index, board, futures, options, financial, macro, global, pool) require EastMoney in `--source` unless noted; Sina covers index quotes and futures quotes; THS covers board list/K-line/members.

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
