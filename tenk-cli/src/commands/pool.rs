use anyhow::Result;
use clap::Subcommand;
use comfy_table::Cell;
use rust_i18n::t;
use tenk::{DataClient, LimitPoolItem};

use crate::args::PoolKindArg;
use crate::i18n::format_amount_i18n;
use crate::output::{OutputConfig, TableRow, change_pct_cell, print_output, right_cell};

#[derive(Subcommand)]
pub enum PoolAction {
    Limit {
        #[arg(short = 't', long, value_enum, default_value = "limit-up")]
        kind: PoolKindArg,
        #[arg(short, long)]
        date: Option<String>,
        #[arg(short, long)]
        limit: Option<usize>,
    },
}

pub async fn handle(
    action: PoolAction,
    client: &DataClient,
    config: &OutputConfig,
) -> Result<()> {
    match action {
        PoolAction::Limit { kind, date, limit } => {
            let data = client
                .get_limit_pool(kind.into(), date.as_deref(), limit)
                .await?;
            print_output(&data, config);
        }
    }
    Ok(())
}

impl TableRow for LimitPoolItem {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new(t!("headers.code")),
            Cell::new(t!("headers.name")),
            Cell::new(t!("headers.price")).set_alignment(comfy_table::CellAlignment::Right),
            Cell::new(t!("headers.change_pct")).set_alignment(comfy_table::CellAlignment::Right),
            Cell::new(t!("headers.amount")).set_alignment(comfy_table::CellAlignment::Right),
            Cell::new(t!("headers.industry")),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.stock_code),
            Cell::new(&self.stock_name),
            right_cell(format!("{:.2}", self.price)),
            change_pct_cell(self.change_pct),
            right_cell(format_amount_i18n(self.amount)),
            Cell::new(&self.industry),
        ]
    }
}
