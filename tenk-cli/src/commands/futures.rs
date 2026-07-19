use anyhow::Result;
use clap::Subcommand;
use comfy_table::Cell;
use rust_i18n::t;
use tenk::{DataClient, DerivativesQuote, FuturesContract};

use crate::KLineArg;
use crate::args::limit_kline;
use crate::output::{OutputConfig, TableRow, change_pct_cell, print_output, right_cell};

#[derive(Subcommand)]
pub enum FuturesAction {
    List {
        #[arg(short, long)]
        limit: Option<usize>,
    },
    Quote {
        secids: Vec<String>,
    },
    Kline {
        secid: String,
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
    action: FuturesAction,
    client: &DataClient,
    config: &OutputConfig,
) -> Result<()> {
    match action {
        FuturesAction::List { limit } => {
            let data = client.get_futures_list(limit).await?;
            print_output(&data, config);
        }
        FuturesAction::Quote { secids } => {
            let refs: Vec<&str> = secids.iter().map(|s| s.as_str()).collect();
            let data = client.get_futures_current(&refs).await?;
            print_output(&data, config);
        }
        FuturesAction::Kline {
            secid,
            kline_type,
            start,
            end,
            limit,
        } => {
            let data = limit_kline(
                client
                    .get_futures_market(&secid, start.as_deref(), end.as_deref(), kline_type.into())
                    .await?,
                limit,
            );
            print_output(&data, config);
        }
    }
    Ok(())
}

impl TableRow for FuturesContract {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new(t!("headers.code")),
            Cell::new(t!("headers.name")),
            Cell::new(t!("headers.secid")),
            Cell::new(t!("headers.exchange")),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.contract_code),
            Cell::new(&self.contract_name),
            Cell::new(&self.secid),
            Cell::new(format!("{:?}", self.exchange)),
        ]
    }
}

impl TableRow for DerivativesQuote {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new(t!("headers.code")),
            Cell::new(t!("headers.name")),
            Cell::new(t!("headers.price")).set_alignment(comfy_table::CellAlignment::Right),
            Cell::new(t!("headers.change_pct")).set_alignment(comfy_table::CellAlignment::Right),
            Cell::new(t!("headers.volume")).set_alignment(comfy_table::CellAlignment::Right),
            Cell::new(t!("headers.open_interest")).set_alignment(comfy_table::CellAlignment::Right),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.contract_code),
            Cell::new(&self.contract_name),
            right_cell(format!("{:.2}", self.price)),
            change_pct_cell(self.change_pct),
            right_cell(self.volume),
            right_cell(self.open_interest.unwrap_or(0)),
        ]
    }
}
