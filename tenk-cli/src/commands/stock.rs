//! Stock command handlers.

use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, CellAlignment, Color};
use tenk::sources::EastMoneySource;
use tenk::traits::{DividendSource, HoldingsSource, ValuationSource};
use tenk::{
    CurrentMarketData, DataClient, DividendData, FundHolding, MarketData, MinuteData,
    OrderBookData, StockCode, StockInfo, StockValuation, TickData, TopHolder,
};

use crate::StockAction;
use crate::output::{
    OutputConfig, SingleDisplay, TableRow, change_pct_cell, format_amount, format_volume,
    price_cell, print_output, print_single, right_cell,
};

/// Handles stock commands.
pub async fn handle(action: StockAction, client: &DataClient, config: &OutputConfig) -> Result<()> {
    match action {
        StockAction::Quote { symbols } => {
            let refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
            let data = client.get_market_current(&refs).await?;
            print_output(&data, config);
        }
        StockAction::Kline {
            symbol,
            kline_type,
            start,
            end,
            limit,
        } => {
            let mut data = client
                .get_market(&symbol, start.as_deref(), end.as_deref(), kline_type.into())
                .await?;

            if let Some(n) = limit {
                let len = data.len();
                if n < len {
                    data = data.split_off(len - n);
                }
            }

            print_output(&data, config);
        }
        StockAction::Minute { symbol } => {
            let data = client.get_market_min(&symbol).await?;
            print_output(&data, config);
        }
        StockAction::Orderbook { symbol } => {
            let data = client.get_order_book(&symbol).await?;
            print_single(&data, config);
        }
        StockAction::Ticks { symbol, limit } => {
            let mut data = client.get_ticks(&symbol).await?;
            data.truncate(limit);
            print_output(&data, config);
        }
        StockAction::Info { symbol } => {
            let data = client.get_stock_info(&symbol).await?;
            print_single(&data, config);
        }
        StockAction::Valuation { symbol } => {
            let source = EastMoneySource::default();
            let data = source.get_valuation(&symbol).await?;
            print_single(&data, config);
        }
        StockAction::Holders { symbol } => {
            let source = EastMoneySource::default();
            let data = source.get_top_holders(&symbol).await?;
            print_output(&data, config);
        }
        StockAction::Funds { symbol, limit } => {
            let source = EastMoneySource::default();
            let data = source.get_fund_holdings(&symbol, limit).await?;
            print_output(&data, config);
        }
        StockAction::Dividend { symbol } => {
            let source = EastMoneySource::default();
            let data = source.get_dividends(&symbol).await?;
            print_output(&data, config);
        }
        StockAction::List { exchange, limit } => {
            let fetch_limit = if exchange.is_some() { None } else { limit };
            let mut data = client.get_all_codes(fetch_limit).await?;

            if let Some(ex) = exchange {
                let ex_upper = ex.to_uppercase();
                data.retain(|c| c.exchange.to_string() == ex_upper);
                if let Some(n) = limit {
                    data.truncate(n);
                }
            }

            print_output(&data, config);
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
            Cell::new(&self.short_name),
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
        vec![Cell::new("Code"), Cell::new("Name"), Cell::new("Exchange")]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.stock_code),
            Cell::new(&self.short_name),
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
            println!(
                "  {:<15} {}",
                "Total Shares:".dimmed(),
                format_volume(total)
            );
        }
        if let Some(circ) = self.circulating_shares {
            println!("  {:<15} {}", "Circulating:".dimmed(), format_volume(circ));
        }
        if let Some(date) = self.list_date {
            println!("  {:<15} {}", "List Date:".dimmed(), date);
        }
    }
}

impl SingleDisplay for StockValuation {
    fn print_single(&self) {
        println!(
            "{} {} ({})",
            "Valuation:".cyan().bold(),
            self.stock_code.white().bold(),
            self.stock_name
        );
        println!("{}", "═".repeat(55).cyan());
        println!(
            "  {:<18} {:.2}",
            "Price:".dimmed(),
            self.price
        );
        println!(
            "  {:<18} {}",
            "Market Cap:".dimmed(),
            format_amount(self.market_cap)
        );
        println!(
            "  {:<18} {}",
            "Float Cap:".dimmed(),
            format_amount(self.float_cap)
        );
        println!("{}", "─".repeat(55));
        if let Some(pe) = self.pe_ttm {
            println!("  {:<18} {:.2}", "PE (TTM):".dimmed(), pe);
        }
        if let Some(pe) = self.pe_static {
            println!("  {:<18} {:.2}", "PE (Static):".dimmed(), pe);
        }
        if let Some(pb) = self.pb {
            println!("  {:<18} {:.2}", "PB:".dimmed(), pb);
        }
        if let Some(ps) = self.ps {
            println!("  {:<18} {:.2}", "PS:".dimmed(), ps);
        }
        println!("{}", "─".repeat(55));
        if let Some(eps) = self.eps {
            println!("  {:<18} {:.4}", "EPS:".dimmed(), eps);
        }
        if let Some(bps) = self.bps {
            println!("  {:<18} {:.2}", "BPS:".dimmed(), bps);
        }
        if let Some(roe) = self.roe {
            println!("  {:<18} {:.2}%", "ROE:".dimmed(), roe);
        }
        if let Some(gm) = self.gross_margin {
            println!("  {:<18} {:.2}%", "Gross Margin:".dimmed(), gm);
        }
        if let Some(nm) = self.net_margin {
            println!("  {:<18} {:.2}%", "Net Margin:".dimmed(), nm);
        }
        println!("{}", "─".repeat(55));
        if let Some(rev) = self.revenue {
            println!("  {:<18} {}", "Revenue:".dimmed(), format_amount(rev));
        }
        if let Some(profit) = self.net_profit {
            println!("  {:<18} {}", "Net Profit:".dimmed(), format_amount(profit));
        }
        if let Some(yoy) = self.revenue_yoy {
            println!("  {:<18} {:.2}%", "Revenue YoY:".dimmed(), yoy);
        }
        if let Some(yoy) = self.profit_yoy {
            println!("  {:<18} {:.2}%", "Profit YoY:".dimmed(), yoy);
        }
    }
}

impl TableRow for TopHolder {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Rank").set_alignment(CellAlignment::Right),
            Cell::new("Holder"),
            Cell::new("Quantity").set_alignment(CellAlignment::Right),
            Cell::new("Ratio%").set_alignment(CellAlignment::Right),
            Cell::new("Change").set_alignment(CellAlignment::Right),
            Cell::new("Type"),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            right_cell(self.rank),
            Cell::new(&self.holder_name),
            right_cell(format_volume(self.hold_quantity)),
            right_cell(format!("{:.2}%", self.hold_ratio)),
            right_cell(
                self.change_quantity
                    .map(|c| {
                        if c > 0 {
                            format!("+{}", format_volume(c as u64))
                        } else if c < 0 {
                            format!("-{}", format_volume((-c) as u64))
                        } else {
                            "0".to_string()
                        }
                    })
                    .unwrap_or_else(|| "-".to_string()),
            ),
            Cell::new(&self.holder_type),
        ]
    }
}

impl TableRow for FundHolding {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Code"),
            Cell::new("Stock"),
            Cell::new("Date"),
            Cell::new("FundName"),
            Cell::new("Shares").set_alignment(CellAlignment::Right),
            Cell::new("Ratio%").set_alignment(CellAlignment::Right),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.stock_code),
            Cell::new(&self.stock_name),
            Cell::new(self.report_date.to_string()),
            Cell::new(&self.fund_name),
            right_cell(format_volume(self.hold_shares)),
            right_cell(format!("{:.2}%", self.hold_ratio)),
        ]
    }
}

impl TableRow for DividendData {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("ReportDate"),
            Cell::new("ExDate"),
            Cell::new("Dividend").set_alignment(CellAlignment::Right),
            Cell::new("Bonus").set_alignment(CellAlignment::Right),
            Cell::new("Transfer").set_alignment(CellAlignment::Right),
            Cell::new("Yield%").set_alignment(CellAlignment::Right),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(self.report_date.to_string()),
            Cell::new(
                self.ex_date
                    .map(|d| d.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            ),
            right_cell(format!("{:.4}", self.dividend_per_share)),
            right_cell(format!("{:.2}", self.bonus_shares)),
            right_cell(format!("{:.2}", self.transfer_shares)),
            right_cell(
                self.dividend_yield
                    .map(|y| format!("{:.2}%", y))
                    .unwrap_or_else(|| "-".to_string()),
            ),
        ]
    }
}
