use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use tenk::sources::{EastMoneySource, SinaSource, THSSource};
use tenk::{DataClient, KLineType};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod commands;
mod output;

use commands::{bond, etf, stock};
use output::OutputFormat;

#[derive(Parser)]
#[command(name = "tenk")]
#[command(author = "Tony Su <peitaosu@163.com>")]
#[command(version = "0.1.0")]
#[command(about = "CLI for fetching market data from multiple sources", long_about = None)]
pub struct Cli {
    #[arg(short, long, global = true)]
    verbose: bool,

    #[arg(short = 'f', long = "format", global = true, value_enum, default_value = "table")]
    format: OutputFormat,

    #[arg(short = 'o', long = "output", global = true)]
    output_file: Option<String>,

    #[arg(short, long, global = true, value_enum, default_values_t = vec![Source::Eastmoney, Source::Sina, Source::Ths])]
    source: Vec<Source>,

    #[arg(long, global = true)]
    proxy: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Copy, ValueEnum, Debug, PartialEq)]
pub enum Source {
    Eastmoney,
    Sina,
    Ths,
}

#[derive(Clone, Copy, ValueEnum, Debug)]
pub enum KLineArg {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Min5,
    Min15,
    Min30,
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

#[derive(Subcommand)]
enum Commands {
    Stock {
        #[command(subcommand)]
        action: StockAction,
    },

    ETF {
        #[command(subcommand)]
        action: ETFAction,
    },

    Bond {
        #[command(subcommand)]
        action: BondAction,
    },
}

#[derive(Subcommand)]
pub enum StockAction {
    Quote {

        #[arg(required = true)]
        symbols: Vec<String>,
    },

    Kline {
        symbol: String,

        #[arg(short = 'k', long, value_enum, default_value = "daily")]
        kline_type: KLineArg,

        #[arg(long)]
        start: Option<String>,

        #[arg(long)]
        end: Option<String>,

        #[arg(short, long)]
        limit: Option<usize>,
    },

    Minute {
        symbol: String,
    },

    Orderbook {
        symbol: String,
    },

    Ticks {
        symbol: String,

        #[arg(short, long, default_value = "50")]
        limit: usize,
    },

    Info {
        symbol: String,
    },

    List {
        #[arg(short, long)]
        exchange: Option<String>,

        #[arg(short, long)]
        limit: Option<usize>,
    },
}

#[derive(Subcommand)]
pub enum ETFAction {
    Quote {
        #[arg(required = true)]
        symbols: Vec<String>,
    },

    Kline {
        symbol: String,

        #[arg(short = 'k', long, value_enum, default_value = "daily")]
        kline_type: KLineArg,

        #[arg(long)]
        start: Option<String>,

        #[arg(long)]
        end: Option<String>,

        #[arg(short, long)]
        limit: Option<usize>,
    },

    Minute {
        symbol: String,
    },

    List {
        #[arg(short, long)]
        exchange: Option<String>,

        #[arg(short, long)]
        limit: Option<usize>,
    },
}

#[derive(Subcommand)]
pub enum BondAction {
    Quote {
        symbols: Vec<String>,

        #[arg(long)]
        top_gainers: Option<usize>,

        #[arg(long)]
        top_losers: Option<usize>,

        #[arg(long)]
        top_volume: Option<usize>,
    },
    List {
        #[arg(short, long)]
        limit: Option<usize>,
    },
}

fn build_client(sources: &[Source]) -> DataClient {
    let mut client = DataClient::new();

    for source in sources {
        match source {
            Source::Eastmoney => {
                client = client
                    .with_source(EastMoneySource::default())
                    .with_fund_source(EastMoneySource::default())
                    .with_bond_source(EastMoneySource::default());
            }
            Source::Sina => {
                client = client
                    .with_source(SinaSource::default())
                    .with_fund_source(SinaSource::default())
                    .with_bond_market_source(SinaSource::default());
            }
            Source::Ths => {
                client = client
                    .with_source(THSSource::default())
                    .with_fund_source(THSSource::default())
                    .with_bond_info_source(THSSource::default());
            }
        }
    }

    client
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose { "debug" } else { "warn" };
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| filter.into()),
        )
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    let client = build_client(&cli.source);

    let output_config = output::OutputConfig {
        format: cli.format,
        file: cli.output_file,
    };

    match cli.command {
        Commands::Stock { action } => stock::handle(action, &client, &output_config).await?,
        Commands::ETF { action } => etf::handle(action, &client, &output_config).await?,
        Commands::Bond { action } => bond::handle(action, &client, &output_config).await?,
    }

    Ok(())
}
