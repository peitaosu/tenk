use anyhow::Result;
use clap::Subcommand;
use comfy_table::Cell;
use rust_i18n::t;
use tenk::{DataClient, FinancialRecord};

use crate::args::FinancialKindArg;
use crate::output::{OutputConfig, TableRow, print_output, right_cell};

#[derive(Subcommand)]
pub enum FinancialAction {
    Statement {
        symbol: String,
        #[arg(short = 'k', long, value_enum, default_value = "income")]
        kind: FinancialKindArg,
        #[arg(short, long)]
        limit: Option<usize>,
    },
}

pub async fn handle(
    action: FinancialAction,
    client: &DataClient,
    config: &OutputConfig,
) -> Result<()> {
    match action {
        FinancialAction::Statement { symbol, kind, limit } => {
            let data = client
                .get_financial_statement(&symbol, kind.into(), limit)
                .await?;
            print_output(&data, config);
        }
    }
    Ok(())
}

impl TableRow for FinancialRecord {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new(t!("headers.code")),
            Cell::new(t!("headers.name")),
            Cell::new(t!("headers.report_date")),
            Cell::new(t!("headers.type")),
            Cell::new(t!("headers.fields")).set_alignment(comfy_table::CellAlignment::Right),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.stock_code),
            Cell::new(&self.stock_name),
            Cell::new(self.report_date.to_string()),
            Cell::new(format!("{:?}", self.kind)),
            right_cell(self.values.len()),
        ]
    }
}
