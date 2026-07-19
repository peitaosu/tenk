use anyhow::Result;
use clap::Subcommand;
use comfy_table::Cell;
use rust_i18n::t;
use tenk::{BoardCrosswalkItem, BoardItem, DataClient};

use crate::KLineArg;
use crate::args::{BoardKindArg, limit_kline};
use crate::output::{OutputConfig, TableRow, change_pct_cell, print_output, print_single, right_cell};

#[derive(Subcommand)]
pub enum BoardAction {
    List {
        #[arg(short = 't', long, value_enum, default_value = "industry")]
        kind: BoardKindArg,
        #[arg(short, long)]
        limit: Option<usize>,
    },
    Kline {
        code: String,
        #[arg(short = 'k', long, value_enum, default_value = "daily")]
        kline_type: KLineArg,
        #[arg(long)]
        start: Option<String>,
        #[arg(long)]
        end: Option<String>,
        #[arg(short, long)]
        limit: Option<usize>,
    },
    Members {
        code: String,
        #[arg(short, long)]
        limit: Option<usize>,
    },
    Crosswalk {
        #[arg(short = 't', long, value_enum, default_value = "industry")]
        kind: BoardKindArg,
        #[arg(short, long)]
        limit: Option<usize>,
    },
    Resolve {
        eastmoney_code: String,
        #[arg(short, long)]
        limit: Option<usize>,
    },
}

pub async fn handle(
    action: BoardAction,
    client: &DataClient,
    config: &OutputConfig,
) -> Result<()> {
    match action {
        BoardAction::List { kind, limit } => {
            let data = match kind {
                BoardKindArg::Industry => client.get_industry_boards(limit).await?,
                BoardKindArg::Concept => client.get_concept_boards(limit).await?,
            };
            print_output(&data, config);
        }
        BoardAction::Kline {
            code,
            kline_type,
            start,
            end,
            limit,
        } => {
            let data = limit_kline(
                client
                    .get_board_market(&code, start.as_deref(), end.as_deref(), kline_type.into())
                    .await?,
                limit,
            );
            print_output(&data, config);
        }
        BoardAction::Members { code, limit } => {
            let data = client.get_board_constituents(&code, limit).await?;
            print_output(&data, config);
        }
        BoardAction::Crosswalk { kind, limit } => {
            let data = client.resolve_board_crosswalk(kind.into(), limit).await?;
            print_output(&data, config);
        }
        BoardAction::Resolve {
            eastmoney_code,
            limit,
        } => {
            let data = client
                .resolve_ths_board_for_eastmoney(&eastmoney_code, limit)
                .await?;
            match data {
                Some(item) => print_single(&item, config),
                None => println!("{}", t!("messages.no_data")),
            }
        }
    }
    Ok(())
}

impl TableRow for BoardItem {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new(t!("headers.code")),
            Cell::new(t!("headers.name")),
            Cell::new(t!("headers.price")).set_alignment(comfy_table::CellAlignment::Right),
            Cell::new(t!("headers.change_pct")).set_alignment(comfy_table::CellAlignment::Right),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.board_code),
            Cell::new(&self.board_name),
            right_cell(format!("{:.2}", self.price)),
            change_pct_cell(self.change_pct),
        ]
    }
}

impl TableRow for BoardCrosswalkItem {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new(t!("headers.name")),
            Cell::new(t!("headers.em_code")),
            Cell::new(t!("headers.ths_code")),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.board_name),
            Cell::new(self.eastmoney_code.as_deref().unwrap_or("-")),
            Cell::new(self.ths_code.as_deref().unwrap_or("-")),
        ]
    }
}

impl crate::output::SingleDisplay for BoardCrosswalkItem {
    fn print_single(&self) {
        println!(
            "{}: {}  {}: {}  {}: {}",
            t!("headers.name"),
            self.board_name,
            t!("headers.em_code"),
            self.eastmoney_code.as_deref().unwrap_or("-"),
            t!("headers.ths_code"),
            self.ths_code.as_deref().unwrap_or("-"),
        );
    }
}
