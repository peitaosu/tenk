//! Stock command handlers.

use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, CellAlignment, Color};
use tenk::{
    CurrentMarketData, DataClient, MarketData, MinuteData, OrderBookData, StockCode, StockInfo,
    TickData,
};

use crate::output::{
    change_pct_cell, format_amount, format_volume, price_cell, print_output, print_single,
    right_cell, truncate_str, OutputFormat, SingleDisplay, TableRow,
};
use crate::StockAction;

pub async fn handle(action: StockAction, client: &DataClient, format: OutputFormat) -> Result<()> {
    match action {
        StockAction::Quote { symbols } => {
            let refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
            let data = client.get_market_current(&refs).await?;
            print_output(&data, format);
        }
        StockAction::Kline {
            symbol,
            kline_type,
            start,
            end,
            limit,
        } => {
            let mut data = client
                .get_market(
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

            print_output(&data, format);
        }
        StockAction::Minute { symbol } => {
            let data = client.get_market_min(&symbol).await?;
            print_output(&data, format);
        }
        StockAction::Orderbook { symbol } => {
            let data = client.get_order_book(&symbol).await?;
            print_single(&data, format);
        }
        StockAction::Ticks { symbol, limit } => {
            let mut data = client.get_ticks(&symbol).await?;
            data.truncate(limit);
            print_output(&data, format);
        }
        StockAction::Info { symbol } => {
            let data = client.get_stock_info(&symbol).await?;
            print_single(&data, format);
        }
        StockAction::List { exchange, limit } => {
            let mut data = client.get_all_codes().await?;

            if let Some(ex) = exchange {
                let ex_upper = ex.to_uppercase();
                data.retain(|c| c.exchange.to_string() == ex_upper);
            }

            if let Some(n) = limit {
                data.truncate(n);
            }

            print_output(&data, format);
        }
    }
    Ok(())
}

impl TableRow for CurrentMarketData {
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
            Cell::new(&self.stock_code),
            Cell::new(truncate_str(&self.short_name, 12)),
            price_cell(self.price),
            change_pct_cell(self.change_pct),
            right_cell(format_volume(self.volume)),
            right_cell(format_amount(self.amount)),
        ]
    }
}

impl TableRow for MarketData {
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
        vec![
            Cell::new(self.trade_date.to_string()),
            price_cell(self.open),
            price_cell(self.high),
            price_cell(self.low),
            price_cell(self.close),
            right_cell(format_volume(self.volume)),
            change_pct_cell(self.change_pct),
        ]
    }
}

impl TableRow for MinuteData {
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
            price_cell(self.price),
            price_cell(self.avg_price),
            change_pct_cell(self.change_pct),
            right_cell(format_volume(self.volume)),
            right_cell(format_amount(self.amount)),
        ]
    }
}

impl TableRow for TickData {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Time"),
            Cell::new("Price").set_alignment(CellAlignment::Right),
            Cell::new("Volume").set_alignment(CellAlignment::Right),
            Cell::new("Direction"),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        let (dir_text, dir_color) = match self.direction {
            'B' | 'b' => ("BUY", Color::Red),
            'S' | 's' => ("SELL", Color::Green),
            _ => ("N/A", Color::White),
        };
        vec![
            Cell::new(self.trade_time.format("%H:%M:%S").to_string()),
            price_cell(self.price),
            right_cell(self.volume),
            Cell::new(dir_text).fg(dir_color),
        ]
    }
}

impl TableRow for StockCode {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Code"),
            Cell::new("Name"),
            Cell::new("Exchange"),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.stock_code),
            Cell::new(truncate_str(&self.short_name, 15)),
            Cell::new(self.exchange.to_string()),
        ]
    }
}

impl SingleDisplay for OrderBookData {
    fn print_single(&self) {
        println!(
            "{} {} ({})",
            "Order Book:".cyan().bold(),
            self.stock_code.white().bold(),
            self.short_name
        );
        println!("{}", "═".repeat(40).cyan());

        println!("\n{}", "Sell (Ask)".red().bold());
        for i in (0..5).rev() {
            if self.sell_prices[i] > 0.0 {
                println!(
                    "  {} {:>10.2}  {:>12}",
                    format!("S{}", i + 1).red(),
                    self.sell_prices[i],
                    format_volume(self.sell_volumes[i])
                );
            }
        }

        println!("{}", "─".repeat(35));

        println!("{}", "Buy (Bid)".green().bold());
        for i in 0..5 {
            if self.buy_prices[i] > 0.0 {
                println!(
                    "  {} {:>10.2}  {:>12}",
                    format!("B{}", i + 1).green(),
                    self.buy_prices[i],
                    format_volume(self.buy_volumes[i])
                );
            }
        }
    }
}

impl SingleDisplay for StockInfo {
    fn print_single(&self) {
        println!(
            "{} {} ({})",
            "Stock Info:".cyan().bold(),
            self.stock_code.white().bold(),
            self.short_name
        );
        println!("{}", "═".repeat(50).cyan());
        println!("  {:<15} {}", "Full Name:".dimmed(), self.full_name);
        println!("  {:<15} {}", "Exchange:".dimmed(), self.exchange);
        if let Some(industry) = &self.industry {
            println!("  {:<15} {}", "Industry:".dimmed(), industry);
        }
        if let Some(total) = self.total_shares {
            println!("  {:<15} {}", "Total Shares:".dimmed(), format_volume(total));
        }
        if let Some(circ) = self.circulating_shares {
            println!("  {:<15} {}", "Circulating:".dimmed(), format_volume(circ));
        }
        if let Some(date) = self.list_date {
            println!("  {:<15} {}", "List Date:".dimmed(), date);
        }
    }
}
