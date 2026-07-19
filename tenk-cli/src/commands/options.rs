use anyhow::Result;
use clap::Subcommand;
use comfy_table::Cell;
use rust_i18n::t;
use serde::Serialize;
use tenk::{DataClient, OptionContract};

use crate::args::OptionExchangeArg;
use crate::output::{OutputConfig, TableRow, change_pct_cell, print_output, right_cell};

#[derive(Serialize)]
struct OptionQuoteRow {
    contract_code: String,
    contract_name: String,
    price: f64,
    change_pct: f64,
    volume: u64,
}

#[derive(Subcommand)]
pub enum OptionsAction {
    List {
        #[arg(short = 'e', long, value_enum, default_value = "sse")]
        exchange: OptionExchangeArg,
        #[arg(short, long)]
        limit: Option<usize>,
    },
    Quote {
        codes: Vec<String>,
    },
}

pub async fn handle(
    action: OptionsAction,
    client: &DataClient,
    config: &OutputConfig,
) -> Result<()> {
    match action {
        OptionsAction::List { exchange, limit } => {
            let data = client.get_options_list(exchange.into(), limit).await?;
            print_output(&data, config);
        }
        OptionsAction::Quote { codes } => {
            let refs: Vec<&str> = codes.iter().map(|s| s.as_str()).collect();
            let data = client.get_options_current(&refs).await?;
            let rows: Vec<OptionQuoteRow> = data
                .iter()
                .map(|quote| OptionQuoteRow {
                    contract_code: quote.contract_code.clone(),
                    contract_name: quote.contract_name.clone(),
                    price: quote.price,
                    change_pct: quote.change_pct,
                    volume: quote.volume,
                })
                .collect();
            print_output(&rows, config);
        }
    }
    Ok(())
}

impl TableRow for OptionContract {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new(t!("headers.code")),
            Cell::new(t!("headers.name")),
            Cell::new(t!("headers.price")).set_alignment(comfy_table::CellAlignment::Right),
            Cell::new(t!("headers.change_pct")).set_alignment(comfy_table::CellAlignment::Right),
            Cell::new(t!("headers.exchange")),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.contract_code),
            Cell::new(&self.contract_name),
            right_cell(format!("{:.2}", self.price)),
            change_pct_cell(self.change_pct),
            Cell::new(format!("{:?}", self.exchange)),
        ]
    }
}

impl TableRow for OptionQuoteRow {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new(t!("headers.code")),
            Cell::new(t!("headers.name")),
            Cell::new(t!("headers.price")).set_alignment(comfy_table::CellAlignment::Right),
            Cell::new(t!("headers.change_pct")).set_alignment(comfy_table::CellAlignment::Right),
            Cell::new(t!("headers.volume")).set_alignment(comfy_table::CellAlignment::Right),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.contract_code),
            Cell::new(&self.contract_name),
            right_cell(format!("{:.2}", self.price)),
            change_pct_cell(self.change_pct),
            right_cell(self.volume),
        ]
    }
}
