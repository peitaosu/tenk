# TUI

Default interface: run `tenk` with no subcommand. CLI subcommands and `--mcp` unchanged.

## Layout

```
┌─ Watchlist (~30%) ─┬─ Quote / Detail (~70%) ─────────────────┐
│                    ├─ K-line ───────────────────────────────┤
├────────────────────┴──────────────────────────────────────────┤
│ Feed 1 │ Feed 2 │ … │ Feed N   (count & types depend on -s)  │
└────────┴────────┴───┴────────────────────────────────────────┘
{time} · {symbol} · {source} · {status}          {key hints}
```

Footer is **outside** all panel borders (1 line, full width).

Ratatui constraints: main `Length(h-1)` + footer `Length(1)`; upper `Ratio(5,8)` / lower `Ratio(3,8)`; upper split `3:7`; right stack `Max(7)` + `Min(4)`; lower row splits evenly across active feed panels (`Ratio(1, N)`).

## Panels

| Panel | Data | Refresh |
|-------|------|---------|
| Watchlist | `get_market_current` | Poll 3s |
| Quote / Detail | `get_market_current`, `get_valuation` | On symbol change, `r` |
| K-line | `get_market` | On symbol / timeframe / scroll; **chart** (chandelier) or **table** (`v`) |

Chart mode: `CandlestickChart` + `VolumeChart` (67% / 33%), red up / green down. Table mode: OHLCV rows.

Feed panels (bottom row) are defined per source in `tenk-cli/src/tui/feed.rs`. Symbol-scoped feeds refresh on symbol change; market-scoped feeds load at startup and on `r` when focused.

List rows: **date · meta · title** on one line (meta = source, institution, or `code name`). Full-width block highlight on selection. TA / analyst panels show summary lines.

## Source selection

TUI uses **one source**: East Money by default, or exactly the source passed with `-s` / `--source`. No multi-source fallback. Bottom feed count and types follow that source.

## Feed panels by source

| Source | Panels (left → right) |
|--------|------------------------|
| East Money (7) | News · Research · Market · Finance · US · 产经 · Institutional |
| Sina (5) | News · Research · Market · US · Industry |
| THS (3) | News · Research · Market |
| TradingView (5) | News · Market · US · TA · Analyst |

| Panel | API | Scope |
|-------|-----|-------|
| News | `search_news_for_symbol` (+ `search_news` fallback) | Symbol |
| Research | `get_research_reports` | Symbol (A-share / SH·SZ·BJ per source) |
| Market / Finance / US / 产经 / … | `get_news(NewsCategory)` | Market |
| Institutional | `get_institutional_research` | Market |
| TA | `get_technical_analysis` | Symbol |
| Analyst | `get_analyst` | Symbol |

Symbol news for Sina / THS uses `search_news` keyword fallback when `search_news_for_symbol` is unavailable.

## Dialogs

| Trigger | Content |
|---------|---------|
| `Enter` on news feeds | `get_news_content` → scrollable `body_text` |
| `Enter` on research feed | Metadata (no full report body in API) |

Modal overlay ~80% terminal; `Esc` closes. Footer shows dialog hints.

## Keys

| Key | Action |
|-----|--------|
| `Tab` / `1–N` | Focus panel (`1` watchlist, `2` quote, `3` kline, `4+` feeds) |
| `j/k` | Scroll list in focused panel |
| `Enter` | Select symbol / open dialog |
| `t` | Cycle kline period: day → 5d → D/W/M/5m…/60m → wrap (kline focus) |
| `←/→` | Cycle kline mode: timeline → chart → table (kline focus); feed page when feed focused |
| `v` | Same as `→` for kline mode (kline focus) |
| `j/k` | Scroll chart/table/timeline window (kline focus) |
| `/` | Search symbol (Enter to add to watchlist) |
| `d` | Remove selected symbol (watchlist focus) |
| `r` | Refresh focused panel |
| `:` | Command line (`add`, `remove`, `refresh`, `quit`) |
| `?` | Help overlay |
| `Esc` | Close dialog / command line |
| `q` | Quit |

## Config

`~/.config/tenk/watchlist.txt` — `CODE:EXCHANGE` or `code\tEXCHANGE[\tname]` per line. Defaults: `600519:SH`, `000001:SZ`, `510300:SH`.

## Flags

Same globals as CLI: `--lang`, `--proxy`, `-s` / `--source`.

## Implementation

`tenk-cli/src/tui/` — `app`, `feed`, `fetch`, `render`, `kline`, `dialog`, `config`, `style`. K-line charts: [chandelier](https://docs.rs/chandelier).
