use anyhow::Result;
use clap::Subcommand;
use comfy_table::Cell;
use rust_i18n::t;
use tenk::{DataClient, MacroRecord};

use crate::output::{OutputConfig, TableRow, print_json_value, print_output, right_cell};

#[derive(Subcommand)]
pub enum MacroAction {
    Cpi {
        #[arg(short, long)]
        limit: Option<usize>,
    },
    Gdp {
        #[arg(short, long)]
        limit: Option<usize>,
    },
    Calendar {
        from: String,
        to: String,
        #[arg(long, default_value = "CN,US")]
        countries: String,
    },
}

pub async fn handle(
    action: MacroAction,
    client: &DataClient,
    config: &OutputConfig,
) -> Result<()> {
    match action {
        MacroAction::Cpi { limit } => {
            let data = client.get_macro_cpi(limit).await?;
            print_output(&data, config);
        }
        MacroAction::Gdp { limit } => {
            let data = client.get_macro_gdp(limit).await?;
            print_output(&data, config);
        }
        MacroAction::Calendar { from, to, countries } => {
            print_json_value(
                &client
                    .get_economic_calendar(&from, &to, &countries)
                    .await?,
                config,
            );
        }
    }
    Ok(())
}

impl TableRow for MacroRecord {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new(t!("headers.indicator")),
            Cell::new(t!("headers.period")),
            Cell::new(t!("headers.report_date")),
            Cell::new(t!("headers.fields")).set_alignment(comfy_table::CellAlignment::Right),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.indicator),
            Cell::new(&self.period),
            Cell::new(self.report_date.to_string()),
            right_cell(self.values.len()),
        ]
    }
}
