//! Bond command handlers.

use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, CellAlignment};
use rust_i18n::t;
use tenk::{BondCurrentData, ConvertibleBondCode, DataClient};

use crate::BondAction;
use crate::i18n::{format_amount_i18n, format_volume_i18n};
use crate::output::{
    OutputConfig, TableRow, change_pct_cell, price_cell, print_output, right_cell,
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
                eprintln!("{}", t!("messages.top_gainers", count = n).red().bold());
            } else if let Some(n) = top_losers {
                data.retain(|b| b.change_pct < 0.0 && b.price > 0.0);
                data.sort_by(|a, b| {
                    a.change_pct
                        .partial_cmp(&b.change_pct)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                data.truncate(n);
                eprintln!("{}", t!("messages.top_losers", count = n).green().bold());
            } else if let Some(n) = top_volume {
                data.retain(|b| b.volume > 0 && b.price > 0.0);
                data.sort_by(|a, b| b.volume.cmp(&a.volume));
                data.truncate(n);
                eprintln!("{}", t!("messages.top_volume", count = n).cyan().bold());
            }

            print_output(&data, config);
        }
        BondAction::List { limit } => {
            let data = client.get_all_bond_codes(limit).await?;
            print_output(&data, config);
        }
    }
    Ok(())
}

impl TableRow for BondCurrentData {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new(t!("headers.code")),
            Cell::new(t!("headers.name")),
            Cell::new(t!("headers.price")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.change_pct")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.volume")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.amount")).set_alignment(CellAlignment::Right),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.bond_code),
            Cell::new(&self.bond_name),
            price_cell(self.price),
            change_pct_cell(self.change_pct),
            right_cell(format_volume_i18n(self.volume)),
            right_cell(format_amount_i18n(self.amount)),
        ]
    }
}

impl TableRow for ConvertibleBondCode {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new(t!("headers.bond_code")),
            Cell::new(t!("headers.bond_name")),
            Cell::new(t!("headers.stock")),
            Cell::new(t!("headers.convert_price")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.list_date")),
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
