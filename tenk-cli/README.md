# tenk-cli

A command-line interface and MCP server for fetching market data from multiple sources.

## Build and Run

```bash
# Build
cargo build --release

# Run
./target/release/tenk --help
```

## MCP Server

Run as MCP server:
```bash
tenk --mcp
```

### Cursor Configuration

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

### Available MCP Tools

| Tool | Description |
|------|-------------|
| `stock_quote` | Get current quotes for stocks |
| `stock_kline` | Get historical K-line data |
| `stock_minute` | Get intraday minute data |
| `stock_orderbook` | Get order book |
| `stock_ticks` | Get tick-by-tick trades |
| `stock_info` | Get stock details |
| `stock_list` | List all stock codes |
| `etf_quote` | Get current ETF quotes |
| `etf_kline` | Get ETF K-line data |
| `etf_minute` | Get ETF minute data |
| `etf_list` | List all ETF codes |
| `bond_quote` | Get bond quotes (with top gainers/losers/volume filters) |
| `bond_list` | List convertible bonds |
| `news_list` | Get latest finance news by category |
| `news_read` | Read full news content by ID |
| `capital_flow` | Get real-time capital flow data for stocks |
| `capital_flow_history` | Get historical capital flow data for a stock |
| `billboard_list` | Get Billboard list |
| `billboard_detail` | Get Billboard details for a stock |
| `earnings_forecast` | Get earnings forecast data |
| `stock_connect` | Get Stock Connect data |
| `margin_trading` | Get margin trading data for a stock |
| `ipo_list` | Get IPO list |
| `block_trades` | Get block trade list |
| `institutional_research` | Get institutional research list |
| `research_reports` | Get research reports |

## Usage

```bash
tenk [OPTIONS] <COMMAND>
```

### Global Options

| Option | Description |
|--------|-------------|
| `-f, --format <FORMAT>` | Output format: `json`, `table` (default), `csv` |
| `-o, --output <FILE>` | Save output to file instead of stdout |
| `-s, --source <SOURCE>` | Data sources: `eastmoney`, `sina`, `ths` (default: all) |
| `--proxy <URL>` | HTTP proxy URL |
| `-v, --verbose` | Enable verbose/debug output |

## Commands

### Stock Commands

```bash
# Get real-time quotes for stocks
tenk stock quote 600519 300059

# Get daily K-line data
tenk stock kline 600519

# Get weekly K-line for date range
tenk stock kline 600519 -k weekly --start 2025-01-01 --end 2025-01-31

# Get last 10 daily records
tenk stock kline 600519 -l 10

# Get 5-minute K-line data
tenk stock kline 600519 -k min5 -l 20

# Get minute-level data for today
tenk stock minute 600519

# Get order book (5-level bid/ask)
tenk stock orderbook 600519

# Get tick data (last 50 ticks)
tenk stock ticks 600519 -l 50

# Get stock information
tenk stock info 600519

# Get stock valuation metrics (PE, PB, Market Cap, EPS, ROE, etc.)
tenk stock valuation 600519

# Get top 10 shareholders
tenk stock holders 600519

# Get fund holdings
tenk stock funds 600519 -l 10

# Get dividend history
tenk stock dividend 600519

# List all stocks
tenk stock list -l 20

# List stocks by exchange
tenk stock list -e sh -l 10
tenk stock list -e sz -l 10
```

### ETF Commands

```bash
# Get real-time ETF quotes
tenk etf quote 510300 159915

# Get ETF K-line data
tenk etf kline 510300

# Get ETF K-line for date range
tenk etf kline 510300 --start 2025-01-01 --end 2025-01-31

# Get ETF minute data
tenk etf minute 510300

# List all ETFs
tenk etf list -l 20

# List ETFs by exchange
tenk etf list -e sh -l 10
```

### Bond Commands

```bash
# Get all convertible bond quotes
tenk bond quote

# Get specific bond quotes
tenk bond quote 127046 113050

# Get top 10 gainers
tenk bond quote --top-gainers 10

# Get top 10 losers
tenk bond quote --top-losers 10

# Get top 10 by volume
tenk bond quote --top-volume 10

# List all convertible bonds
tenk bond list -l 20
```

### Market Commands

```bash
# Get real-time capital flow for stocks
tenk market flow 600519 300059

# Get historical capital flow for a stock
tenk market flow-history 600519 -l 30

# Get Billboard list (defaults to today)
tenk market billboard

# Get Billboard list for specific date
tenk market billboard -d 2026-01-16

# Get Billboard details for a stock
tenk market billboard-detail 600519 -d 2026-01-16

# Get earnings forecast
tenk market forecast -l 10

# Get earnings forecast for specific period
tenk market forecast -r 2024-12-31 -l 20

# Get Stock Connect data
tenk market connect -l 30

# Get margin trading data for a stock
tenk market margin 600519 -l 30

# Get IPO list
tenk market ipo -l 10

# Get block trade list
tenk market block -l 10

# Get institutional research list
tenk market research -l 10

# Get research reports
tenk market report -l 10

# Get research reports for a specific stock
tenk market report -c 600519 -l 10
```

### News Commands

```bash
# Get latest finance news
tenk news list

# Get news by category
tenk news list -c company -l 20

# Search news by keyword
tenk news search 600519 -l 10

# Read full news content
tenk news read 202601153620739638
```

### Output Formats

```bash
# Table output (default) - pretty formatted
tenk stock quote 600519

# JSON output - for scripting
tenk stock quote 600519 -f json

# CSV output - for spreadsheets
tenk stock quote 600519 -f csv > quotes.csv

# Save output to file
tenk stock quote 600519 -f json -o quotes.json
tenk stock kline 600519 -l 30 -o kline_data.csv -f csv
```

### Select Data Source

```bash
# Use only EastMoney source
tenk -s eastmoney stock quote 600519

# Use only Sina source
tenk -s sina stock quote 600519

# Use THS then Sina as fallback
tenk -s ths -s sina stock quote 600519
```

## License

MIT
