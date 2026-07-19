use anyhow::Result;
use clap::Subcommand;
use comfy_table::Cell;
use rust_i18n::t;
use tenk::{DataClient, IndexCode};

use crate::KLineArg;
use crate::args::limit_kline;
use crate::output::{OutputConfig, TableRow, print_output};

#[derive(Subcommand)]
pub enum IndexAction {
    List {
        #[arg(short, long)]
        limit: Option<usize>,
    },
    Quote {
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
}

pub async fn handle(
    action: IndexAction,
    client: &DataClient,
    config: &OutputConfig,
) -> Result<()> {
    match action {
        IndexAction::List { limit } => {
            let data = client.get_index_list(limit).await?;
            print_output(&data, config);
        }
        IndexAction::Quote { symbols } => {
            let refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
            let data = client.get_index_current(&refs).await?;
            print_output(&data, config);
        }
        IndexAction::Kline {
            symbol,
            kline_type,
            start,
            end,
            limit,
        } => {
            let data = limit_kline(
                client
                    .get_index_market(&symbol, start.as_deref(), end.as_deref(), kline_type.into())
                    .await?,
                limit,
            );
            print_output(&data, config);
        }
    }
    Ok(())
}

impl TableRow for IndexCode {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new(t!("headers.code")),
            Cell::new(t!("headers.name")),
            Cell::new(t!("headers.exchange")),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.index_code),
            Cell::new(&self.index_name),
            Cell::new(format!("{:?}", self.exchange)),
        ]
    }
}
