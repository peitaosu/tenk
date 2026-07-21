use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use tenk::{DataClient, KLineType, SourceKind};
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

rust_i18n::i18n!("locales", fallback = "en");

mod args;
mod client;
mod commands;
mod i18n;
mod mcp;
mod output;
mod tv_util;
mod tui;

use commands::{board, bond, etf, financial, futures, global, index, macro_data, market, news, options, pool, stock};
use output::OutputFormat;

/// CLI application entry point.
#[derive(Parser)]
#[command(name = "tenk")]
#[command(author = "Tony Su <peitaosu@163.com>")]
#[command(version = "0.2.0")]
#[command(about = "CLI for fetching market data from multiple sources", long_about = None)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Output format
    #[arg(
        short = 'f',
        long = "format",
        global = true,
        value_enum,
        default_value = "table"
    )]
    format: OutputFormat,

    /// Output file path
    #[arg(short = 'o', long = "output", global = true)]
    output_file: Option<String>,

    /// Data sources to use
    #[arg(short, long, global = true, value_enum)]
    source: Vec<Source>,

    /// HTTP proxy URL for TradingView only
    #[arg(long, global = true)]
    proxy: Option<String>,

    /// Output language (en, zh-CN)
    #[arg(short = 'L', long = "lang", global = true, default_value = "en")]
    lang: String,

    /// Subcommand to execute
    #[command(subcommand)]
    command: Option<Commands>,

    /// Run as MCP server
    #[arg(long)]
    mcp: bool,
}

/// Data source provider.
#[derive(Clone, Copy, ValueEnum, Debug, PartialEq)]
pub enum Source {
    /// East Money
    Eastmoney,
    /// Sina Finance
    Sina,
    /// THS
    Ths,
    /// TradingView
    Tradingview,
}

impl Source {
    fn as_kind(self) -> SourceKind {
        match self {
            Source::Eastmoney => SourceKind::Eastmoney,
            Source::Sina => SourceKind::Sina,
            Source::Ths => SourceKind::Ths,
            Source::Tradingview => SourceKind::Tradingview,
        }
    }
}

/// K-line type for CLI arguments.
#[derive(Clone, Copy, ValueEnum, Debug)]
pub enum KLineArg {
    /// Daily K-line
    Daily,
    /// Weekly K-line
    Weekly,
    /// Monthly K-line
    Monthly,
    /// Quarterly K-line
    Quarterly,
    /// 5-minute K-line
    Min5,
    /// 15-minute K-line
    Min15,
    /// 30-minute K-line
    Min30,
    /// 60-minute K-line
    Min60,
}

impl From<KLineArg> for KLineType {
    fn from(arg: KLineArg) -> Self {
        match arg {
            KLineArg::Daily => KLineType::Daily,
            KLineArg::Weekly => KLineType::Weekly,
            KLineArg::Monthly => KLineType::Monthly,
            KLineArg::Quarterly => KLineType::Quarterly,
            KLineArg::Min5 => KLineType::Min5,
            KLineArg::Min15 => KLineType::Min15,
            KLineArg::Min30 => KLineType::Min30,
            KLineArg::Min60 => KLineType::Min60,
        }
    }
}

/// CLI commands.
#[derive(Subcommand)]
enum Commands {
    /// Stock market commands
    Stock {
        /// Stock subcommand action
        #[command(subcommand)]
        action: StockAction,
    },

    /// ETF commands
    ETF {
        /// ETF subcommand action
        #[command(subcommand)]
        action: ETFAction,
    },

    /// Convertible bond commands
    Bond {
        /// Bond subcommand action
        #[command(subcommand)]
        action: BondAction,
    },

    /// News commands
    News {
        /// News subcommand action
        #[command(subcommand)]
        action: NewsAction,
    },

    /// Market data commands
    Market {
        /// Market subcommand action
        #[command(subcommand)]
        action: MarketAction,
    },

    /// Index commands
    Index {
        #[command(subcommand)]
        action: index::IndexAction,
    },

    /// Board commands
    Board {
        #[command(subcommand)]
        action: board::BoardAction,
    },

    /// Futures commands
    Futures {
        #[command(subcommand)]
        action: futures::FuturesAction,
    },

    /// Options commands
    Options {
        #[command(subcommand)]
        action: options::OptionsAction,
    },

    /// Financial statement commands
    Financial {
        #[command(subcommand)]
        action: financial::FinancialAction,
    },

    /// Macro economic data
    Macro {
        #[command(subcommand)]
        action: macro_data::MacroAction,
    },

    /// Hong Kong / US quotes
    Global {
        #[command(subcommand)]
        action: global::GlobalAction,
    },

    /// Limit pool commands
    Pool {
        #[command(subcommand)]
        action: pool::PoolAction,
    },
}

/// Stock subcommands.
#[derive(Subcommand)]
pub enum StockAction {
    /// Get real-time quotes
    Quote {
        /// Stock symbols to query
        #[arg(required = true)]
        symbols: Vec<String>,
    },
    /// Get K-line (candlestick) data
    Kline {
        /// Stock symbol
        symbol: String,

        /// K-line type
        #[arg(short = 'k', long, value_enum, default_value = "daily")]
        kline_type: KLineArg,

        /// Start date
        #[arg(long)]
        start: Option<String>,

        /// End date
        #[arg(long)]
        end: Option<String>,

        /// Maximum number of records
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Get intraday minute data
    Minute {
        /// Stock symbol
        symbol: String,
    },
    /// Get order book
    Orderbook {
        /// Stock symbol
        symbol: String,
    },
    /// Get recent tick-by-tick trades
    Ticks {
        /// Stock symbol
        symbol: String,

        /// Maximum number of records
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
    /// Get stock info
    Info {
        /// Stock symbol
        symbol: String,
    },
    /// Get stock valuation metrics
    Valuation {
        /// Stock symbol
        symbol: String,
    },
    /// Get top 10 shareholders
    Holders {
        /// Stock symbol
        symbol: String,
    },
    /// Get fund holdings
    Funds {
        /// Stock symbol
        symbol: String,

        /// Maximum number of records
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Get dividend history
    Dividend {
        /// Stock symbol
        symbol: String,
    },
    /// List all available stocks
    List {
        /// Exchange filter
        #[arg(short, long)]
        exchange: Option<String>,

        /// Maximum number of records
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Search symbols globally
    Search {
        query: String,
        #[arg(long)]
        filter: Option<String>,
        #[arg(long, default_value = "0")]
        offset: u32,
    },
    /// Get TradingView technical analysis consensus
    Ta {
        symbol: String,
    },
    /// Get TradingView analyst consensus and estimates
    Analyst {
        symbol: String,
    },
}

/// ETF subcommands.
#[derive(Subcommand)]
pub enum ETFAction {
    /// Get real-time quotes
    Quote {
        /// ETF symbols to query
        #[arg(required = true)]
        symbols: Vec<String>,
    },
    /// Get K-line (candlestick) data
    Kline {
        /// ETF symbol
        symbol: String,

        /// K-line type
        #[arg(short = 'k', long, value_enum, default_value = "daily")]
        kline_type: KLineArg,

        /// Start date
        #[arg(long)]
        start: Option<String>,

        /// End date
        #[arg(long)]
        end: Option<String>,

        /// Maximum number of records
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Get intraday minute data
    Minute {
        /// ETF symbol
        symbol: String,
    },
    /// List all available ETFs
    List {
        /// Exchange filter
        #[arg(short, long)]
        exchange: Option<String>,

        /// Maximum number of records
        #[arg(short, long)]
        limit: Option<usize>,
    },
}

/// Convertible bond subcommands.
#[derive(Subcommand)]
pub enum BondAction {
    /// Get real-time quotes with optional ranking filters
    Quote {
        /// Bond symbols to query
        symbols: Vec<String>,

        /// Top N gainers
        #[arg(long)]
        top_gainers: Option<usize>,

        /// Top N losers
        #[arg(long)]
        top_losers: Option<usize>,

        /// Top N by volume
        #[arg(long)]
        top_volume: Option<usize>,
    },
    /// List all available convertible bonds
    List {
        /// Maximum number of records
        #[arg(short, long)]
        limit: Option<usize>,
    },
}

/// News subcommands.
#[derive(Subcommand)]
pub enum NewsAction {
    /// Get latest news by category
    List {
        /// Category: finance, company, stock, us, global, domestic, industry
        #[arg(short, long, default_value = "finance")]
        category: String,

        /// Page number
        #[arg(short, long, default_value = "1")]
        page: u32,

        /// Number of articles
        #[arg(short, long, default_value = "20")]
        limit: u32,
    },
    /// Search news by keyword
    Search {
        /// Search keyword
        keyword: String,

        /// Page number
        #[arg(short, long, default_value = "1")]
        page: u32,

        /// Number of results
        #[arg(short, long, default_value = "10")]
        limit: u32,
    },
    /// Read full news content by ID
    Read {
        /// News ID
        id: String,
    },
}

/// Market data subcommands.
#[derive(Subcommand)]
pub enum MarketAction {
    /// Get real-time capital flow for stocks
    Flow {
        /// Stock symbols
        #[arg(required = true)]
        symbols: Vec<String>,
    },
    /// Get historical capital flow for a stock
    FlowHistory {
        /// Stock symbol
        symbol: String,

        /// Number of days
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Get Billboard list
    Billboard {
        /// Trade date
        #[arg(short, long)]
        date: Option<String>,
    },
    /// Get Billboard details for a stock
    BillboardDetail {
        /// Stock symbol
        symbol: String,

        /// Trade date
        #[arg(short, long)]
        date: String,
    },
    /// Get earnings forecast
    Forecast {
        /// Report period
        #[arg(short = 'r', long)]
        period: Option<String>,

        /// Page number
        #[arg(short, long, default_value = "1")]
        page: u32,

        /// Number of records
        #[arg(short, long, default_value = "50")]
        limit: u32,
    },
    /// Get Stock Connect data
    Connect {
        /// Number of days
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Get margin trading data
    Margin {
        /// Stock symbol
        symbol: String,

        /// Number of days
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Get IPO list
    Ipo {
        /// Number of records
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Get block trade list
    Block {
        /// Number of records
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Get institutional research list
    Research {
        /// Number of records
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Get research reports
    Report {
        /// Stock symbol
        #[arg(short = 'c', long)]
        symbol: Option<String>,

        /// Number of records
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Run market screener
    Screener {
        #[arg(long, default_value = "china")]
        market: String,
        #[arg(long, value_delimiter = ',')]
        columns: Vec<String>,
        #[arg(long, default_value = "change")]
        sort_by: String,
        #[arg(long, default_value = "desc")]
        sort_order: String,
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
    /// Get market hotlist
    Hotlist {
        #[arg(long, default_value = "america")]
        market: String,
        #[arg(long, default_value = "gainers")]
        kind: String,
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
    /// Search indicators and strategies
    IndicatorSearch {
        query: String,
    },
    /// Get indicator specification
    Indicator {
        id: String,
        #[arg(long, default_value = "last")]
        version: String,
    },
    /// Get indicator time series
    IndicatorSeries {
        symbol: String,
        id: String,
        #[arg(long, default_value = "last")]
        version: String,
        #[arg(long, default_value = "1D")]
        timeframe: String,
        #[arg(short, long, default_value = "100")]
        limit: usize,
    },
    /// Get strategy backtest report
    Strategy {
        symbol: String,
        id: String,
        #[arg(long, default_value = "last")]
        version: String,
        #[arg(long, default_value = "1D")]
        timeframe: String,
        #[arg(short, long, default_value = "300")]
        limit: usize,
    },
    /// Replay chart bars from timestamp
    Replay {
        symbol: String,
        from: i64,
        #[arg(long, default_value = "1")]
        steps: u32,
        #[arg(long, default_value = "1D")]
        timeframe: String,
    },
    /// Get saved chart drawings
    Drawings {
        layout: String,
        symbol: String,
        user_id: i64,
    },
}

fn build_client(sources: &[SourceKind], proxy: Option<&str>) -> DataClient {
    client::build_client(sources, proxy).expect("failed to build data client")
}

fn source_kinds(sources: &[Source]) -> Vec<SourceKind> {
    sources.iter().map(|source| source.as_kind()).collect()
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    i18n::init(&cli.lang);

    if cli.mcp {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env().add_directive(Level::INFO.into()),
            )
            .with_writer(std::io::stderr)
            .init();

        return mcp::run_server().await;
    }

    let kinds = source_kinds(&cli.source);
    let tui_sources = client::resolve_tui_sources(&kinds);
    let client = if cli.command.is_none() {
        build_client(&tui_sources, cli.proxy.as_deref())
    } else {
        build_client(&client::resolve_cli_sources(&kinds), cli.proxy.as_deref())
    };

    if cli.command.is_none() {
        let source = tui_sources[0];
        return tui::run(client, source).await;
    }

    let filter = if cli.verbose { "debug" } else { "warn" };
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()),
        )
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    let output_config = output::OutputConfig {
        format: cli.format,
        file: cli.output_file,
    };

    match cli.command.expect("command checked above") {
        Commands::Stock { action } => stock::handle(action, &client, &output_config).await?,
        Commands::ETF { action } => etf::handle(action, &client, &output_config).await?,
        Commands::Bond { action } => bond::handle(action, &client, &output_config).await?,
        Commands::News { action } => news::handle(action, &client, &output_config).await?,
        Commands::Market { action } => market::handle(action, &client, &output_config).await?,
        Commands::Index { action } => index::handle(action, &client, &output_config).await?,
        Commands::Board { action } => board::handle(action, &client, &output_config).await?,
        Commands::Futures { action } => futures::handle(action, &client, &output_config).await?,
        Commands::Options { action } => options::handle(action, &client, &output_config).await?,
        Commands::Financial { action } => financial::handle(action, &client, &output_config).await?,
        Commands::Macro { action } => macro_data::handle(action, &client, &output_config).await?,
        Commands::Global { action } => global::handle(action, &client, &output_config).await?,
        Commands::Pool { action } => pool::handle(action, &client, &output_config).await?,
    }

    Ok(())
}
