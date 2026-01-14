# tenk-cli

A command-line interface for fetching market data from multiple sources using the `tenk` library.

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
