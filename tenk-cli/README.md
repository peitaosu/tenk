# tenk-cli

A command-line interface for fetching market data from multiple sources using the `tenk` library.

## Usage

```bash
tenk-cli [OPTIONS] <COMMAND>
```

### Global Options

| Option | Description |
|--------|-------------|
| `-o, --output <FORMAT>` | Output format: `json`, `table` (default), `csv` |
| `-s, --source <SOURCE>` | Data sources: `eastmoney`, `sina`, `ths` (default: all) |
| `--proxy <URL>` | HTTP proxy URL |
| `-v, --verbose` | Enable verbose/debug output |

## Commands

### Stock Commands

```bash
# Get real-time quotes for stocks
tenk-cli stock quote 600519 300059

# Get daily K-line data
tenk-cli stock kline 600519

# Get weekly K-line for date range
tenk-cli stock kline 600519 -k weekly --start 2025-01-01 --end 2025-01-31

# Get last 10 daily records
tenk-cli stock kline 600519 -l 10

# Get 5-minute K-line data
tenk-cli stock kline 600519 -k min5 -l 20

# Get minute-level data for today
tenk-cli stock minute 600519

# Get order book (5-level bid/ask)
tenk-cli stock orderbook 600519

# Get tick data (last 50 ticks)
tenk-cli stock ticks 600519 -l 50

# Get stock information
tenk-cli stock info 600519

# List all stocks
tenk-cli stock list -l 20

# List stocks by exchange
tenk-cli stock list -e sh -l 10
tenk-cli stock list -e sz -l 10
```

### ETF Commands

```bash
# Get real-time ETF quotes
tenk-cli etf quote 510300 159915

# Get ETF K-line data
tenk-cli etf kline 510300

# Get ETF K-line for date range
tenk-cli etf kline 510300 --start 2025-01-01 --end 2025-01-31

# Get ETF minute data
tenk-cli etf minute 510300

# List all ETFs
tenk-cli etf list -l 20

# List ETFs by exchange
tenk-cli etf list -e sh -l 10
```

### Bond Commands

```bash
# Get all convertible bond quotes
tenk-cli bond quote

# Get specific bond quotes
tenk-cli bond quote 127046 113050

# Get top 10 gainers
tenk-cli bond quote --top-gainers 10

# Get top 10 losers
tenk-cli bond quote --top-losers 10

# Get top 10 by volume
tenk-cli bond quote --top-volume 10

# List all convertible bonds
tenk-cli bond list -l 20
```

### Output Formats

```bash
# Table output (default) - pretty formatted
tenk-cli stock quote 600519

# JSON output - for scripting
tenk-cli stock quote 600519 -o json

# CSV output - for spreadsheets
tenk-cli stock quote 600519 -o csv > quotes.csv
```

### Select Data Source

```bash
# Use only EastMoney source
tenk-cli -s eastmoney stock quote 600519

# Use only Sina source
tenk-cli -s sina stock quote 600519

# Use THS then Sina as fallback
tenk-cli -s ths -s sina stock quote 600519
```

## License

MIT
