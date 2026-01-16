//! Bond command handlers.

use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, CellAlignment};
use tenk::{BondCurrentData, ConvertibleBondCode, DataClient};

use crate::BondAction;
use crate::output::{
    OutputConfig, TableRow, change_pct_cell, format_amount, format_volume, price_cell,
    print_output, right_cell,
};

/// Handles bond commands.
pub async fn handle(action: BondAction, client: &DataClient, config: &OutputConfig) -> Result<()> {
    match action {
        BondAction::Quote {
            symbols,
            top_gainers,
            top_losers,
            top_volume,
        } => {
            let codes: Option<Vec<&str>> = if symbols.is_empty() {
                None
            } else {
                Some(symbols.iter().map(|s| s.as_str()).collect())
            };

            let mut data = client.get_bond_current(codes.as_deref()).await?;

            if let Some(n) = top_gainers {
                data.retain(|b| b.change_pct > 0.0 && b.price > 0.0);
                data.sort_by(|a, b| {
                    b.change_pct
                        .partial_cmp(&a.change_pct)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                data.truncate(n);
                eprintln!("{}", format!("Top {} Gainers:", n).red().bold());
            } else if let Some(n) = top_losers {
                data.retain(|b| b.change_pct < 0.0 && b.price > 0.0);
                data.sort_by(|a, b| {
                    a.change_pct
                        .partial_cmp(&b.change_pct)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                data.truncate(n);
                eprintln!("{}", format!("Top {} Losers:", n).green().bold());
            } else if let Some(n) = top_volume {
                data.retain(|b| b.volume > 0 && b.price > 0.0);
                data.sort_by(|a, b| b.volume.cmp(&a.volume));
                data.truncate(n);
                eprintln!("{}", format!("Top {} by Volume:", n).cyan().bold());
            }

            print_output(&data, config);
        }
        BondAction::List { limit } => {
            let mut data = client.get_all_bond_codes().await?;

            if let Some(n) = limit {
                data.truncate(n);
            }

            print_output(&data, config);
        }
    }
    Ok(())
}

impl TableRow for BondCurrentData {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Code"),
            Cell::new("Name"),
            Cell::new("Price").set_alignment(CellAlignment::Right),
            Cell::new("Change%").set_alignment(CellAlignment::Right),
            Cell::new("Volume").set_alignment(CellAlignment::Right),
            Cell::new("Amount").set_alignment(CellAlignment::Right),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.bond_code),
            Cell::new(&self.bond_name),
            price_cell(self.price),
            change_pct_cell(self.change_pct),
            right_cell(format_volume(self.volume)),
            right_cell(format_amount(self.amount)),
        ]
    }
}

impl TableRow for ConvertibleBondCode {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Bond Code"),
            Cell::new("Bond Name"),
            Cell::new("Stock"),
            Cell::new("Convert Price").set_alignment(CellAlignment::Right),
            Cell::new("List Date"),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.bond_code),
            Cell::new(&self.bond_name),
            Cell::new(&self.stock_code),
            right_cell(
                self.convert_price
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            ),
            Cell::new(
                self.listing_date
                    .map(|d| d.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            ),
        ]
    }
}
