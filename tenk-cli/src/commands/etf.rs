//! ETF command handlers.

use anyhow::Result;
use comfy_table::{Cell, CellAlignment};
use tenk::{DataClient, ETFCode, ETFCurrentData, ETFMarketData, ETFMinuteData};

use crate::output::{
    change_pct_cell, format_amount, format_volume, price_cell_3, print_output, right_cell,
    OutputConfig, TableRow,
};
use crate::ETFAction;

/// Handles ETF commands.
pub async fn handle(action: ETFAction, client: &DataClient, config: &OutputConfig) -> Result<()> {
    match action {
        ETFAction::Quote { symbols } => {
            let refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
            let data = client.get_etf_current(&refs).await?;
            print_output(&data, config);
        }
        ETFAction::Kline {
            symbol,
            kline_type,
            start,
            end,
            limit,
        } => {
            let mut data = client
                .get_etf_market(
                    &symbol,
                    start.as_deref(),
                    end.as_deref(),
                    kline_type.into(),
                )
                .await?;

            if let Some(n) = limit {
                let len = data.len();
                if n < len {
                    data = data.split_off(len - n);
                }
            }

            print_output(&data, config);
        }
        ETFAction::Minute { symbol } => {
            let data = client.get_etf_min(&symbol).await?;
            print_output(&data, config);
        }
        ETFAction::List { exchange, limit } => {
            let mut data = client.get_all_etf_codes().await?;

            if let Some(ex) = exchange {
                let ex_upper = ex.to_uppercase();
                data.retain(|c| c.exchange.to_string() == ex_upper);
            }

            if let Some(n) = limit {
                data.truncate(n);
            }

            print_output(&data, config);
        }
    }
    Ok(())
}

impl TableRow for ETFCurrentData {
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
        let change_pct = self.change_pct.unwrap_or(0.0);
        vec![
            Cell::new(&self.fund_code),
            Cell::new(&self.short_name),
            price_cell_3(self.price),
            change_pct_cell(change_pct),
            right_cell(format_volume(self.volume)),
            right_cell(format_amount(self.amount)),
        ]
    }
}

impl TableRow for ETFMarketData {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Date"),
            Cell::new("Open").set_alignment(CellAlignment::Right),
            Cell::new("High").set_alignment(CellAlignment::Right),
            Cell::new("Low").set_alignment(CellAlignment::Right),
            Cell::new("Close").set_alignment(CellAlignment::Right),
            Cell::new("Volume").set_alignment(CellAlignment::Right),
            Cell::new("Change%").set_alignment(CellAlignment::Right),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        let change_pct = self.change_pct.unwrap_or(0.0);
        vec![
            Cell::new(self.trade_date.to_string()),
            price_cell_3(self.open),
            price_cell_3(self.high),
            price_cell_3(self.low),
            price_cell_3(self.close),
            right_cell(format_volume(self.volume)),
            change_pct_cell(change_pct),
        ]
    }
}

impl TableRow for ETFMinuteData {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Time"),
            Cell::new("Price").set_alignment(CellAlignment::Right),
            Cell::new("AvgPrice").set_alignment(CellAlignment::Right),
            Cell::new("Change%").set_alignment(CellAlignment::Right),
            Cell::new("Volume").set_alignment(CellAlignment::Right),
            Cell::new("Amount").set_alignment(CellAlignment::Right),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(self.trade_time.format("%H:%M").to_string()),
            price_cell_3(self.price),
            price_cell_3(self.avg_price),
            change_pct_cell(self.change_pct),
            right_cell(format_volume(self.volume)),
            right_cell(format_amount(self.amount)),
        ]
    }
}

impl TableRow for ETFCode {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Code"),
            Cell::new("Name"),
            Cell::new("Exchange"),
            Cell::new("NAV").set_alignment(CellAlignment::Right),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.fund_code),
            Cell::new(&self.short_name),
            Cell::new(self.exchange.to_string()),
            right_cell(
                self.net_value
                    .map(|v| format!("{:.3}", v))
                    .unwrap_or_else(|| "-".to_string()),
            ),
        ]
    }
}
