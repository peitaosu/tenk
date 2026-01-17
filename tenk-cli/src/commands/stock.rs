//! Stock command handlers.

use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, CellAlignment, Color};
use rust_i18n::t;
use tenk::sources::EastMoneySource;
use tenk::traits::{DividendSource, HoldingsSource, ValuationSource};
use tenk::{
    CurrentMarketData, DataClient, DividendData, FundHolding, MarketData, MinuteData,
    OrderBookData, StockCode, StockInfo, StockValuation, TickData, TopHolder,
};

use crate::StockAction;
use crate::i18n::pad_display_width;
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
            Cell::new(t!("headers.date")),
            Cell::new(t!("headers.open")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.high")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.low")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.close")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.volume")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.change_pct")).set_alignment(CellAlignment::Right),
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
            Cell::new(t!("headers.time")),
            Cell::new(t!("headers.price")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.avg_price")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.change_pct")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.volume")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.amount")).set_alignment(CellAlignment::Right),
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
            Cell::new(t!("headers.time")),
            Cell::new(t!("headers.price")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.volume")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.direction")),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        let (dir_text, dir_color) = match self.direction {
            'B' | 'b' => (t!("trade.buy"), Color::Red),
            'S' | 's' => (t!("trade.sell"), Color::Green),
            _ => (t!("trade.na"), Color::White),
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
            Cell::new(t!("headers.code")),
            Cell::new(t!("headers.name")),
            Cell::new(t!("headers.exchange")),
        ]
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
            t!("labels.order_book").cyan().bold(),
            self.stock_code.white().bold(),
            self.short_name
        );
        println!("{}", "═".repeat(40).cyan());

        println!("\n{}", t!("labels.sell_ask").red().bold());
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

        println!("{}", t!("labels.buy_bid").green().bold());
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
        const LABEL_WIDTH: usize = 15;
        println!(
            "{} {} ({})",
            t!("labels.stock_info").cyan().bold(),
            self.stock_code.white().bold(),
            self.short_name
        );
        println!("{}", "═".repeat(50).cyan());
        println!(
            "  {} {}",
            pad_display_width(&t!("labels.full_name"), LABEL_WIDTH).dimmed(),
            self.full_name
        );
        println!(
            "  {} {}",
            pad_display_width(&t!("labels.exchange"), LABEL_WIDTH).dimmed(),
            self.exchange
        );
        if let Some(industry) = &self.industry {
            println!(
                "  {} {}",
                pad_display_width(&t!("labels.industry"), LABEL_WIDTH).dimmed(),
                industry
            );
        }
        if let Some(total) = self.total_shares {
            println!(
                "  {} {}",
                pad_display_width(&t!("labels.total_shares"), LABEL_WIDTH).dimmed(),
                format_volume(total)
            );
        }
        if let Some(circ) = self.circulating_shares {
            println!(
                "  {} {}",
                pad_display_width(&t!("labels.circulating"), LABEL_WIDTH).dimmed(),
                format_volume(circ)
            );
        }
        if let Some(date) = self.list_date {
            println!(
                "  {} {}",
                pad_display_width(&t!("labels.list_date"), LABEL_WIDTH).dimmed(),
                date
            );
        }
    }
}

impl SingleDisplay for StockValuation {
    fn print_single(&self) {
        const LABEL_WIDTH: usize = 18;
        println!(
            "{} {} ({})",
            t!("labels.valuation").cyan().bold(),
            self.stock_code.white().bold(),
            self.stock_name
        );
        println!("{}", "═".repeat(55).cyan());
        println!(
            "  {} {:.2}",
            pad_display_width(&t!("labels.price"), LABEL_WIDTH).dimmed(),
            self.price
        );
        println!(
            "  {} {}",
            pad_display_width(&t!("labels.market_cap"), LABEL_WIDTH).dimmed(),
            format_amount(self.market_cap)
        );
        println!(
            "  {} {}",
            pad_display_width(&t!("labels.float_cap"), LABEL_WIDTH).dimmed(),
            format_amount(self.float_cap)
        );
        println!("{}", "─".repeat(55));
        if let Some(pe) = self.pe_ttm {
            println!(
                "  {} {:.2}",
                pad_display_width(&t!("labels.pe_ttm"), LABEL_WIDTH).dimmed(),
                pe
            );
        }
        if let Some(pe) = self.pe_static {
            println!(
                "  {} {:.2}",
                pad_display_width(&t!("labels.pe_static"), LABEL_WIDTH).dimmed(),
                pe
            );
        }
        if let Some(pb) = self.pb {
            println!(
                "  {} {:.2}",
                pad_display_width(&t!("labels.pb"), LABEL_WIDTH).dimmed(),
                pb
            );
        }
        if let Some(ps) = self.ps {
            println!(
                "  {} {:.2}",
                pad_display_width(&t!("labels.ps"), LABEL_WIDTH).dimmed(),
                ps
            );
        }
        println!("{}", "─".repeat(55));
        if let Some(eps) = self.eps {
            println!(
                "  {} {:.4}",
                pad_display_width(&t!("labels.eps"), LABEL_WIDTH).dimmed(),
                eps
            );
        }
        if let Some(bps) = self.bps {
            println!(
                "  {} {:.2}",
                pad_display_width(&t!("labels.bps"), LABEL_WIDTH).dimmed(),
                bps
            );
        }
        if let Some(roe) = self.roe {
            println!(
                "  {} {:.2}%",
                pad_display_width(&t!("labels.roe"), LABEL_WIDTH).dimmed(),
                roe
            );
        }
        if let Some(gm) = self.gross_margin {
            println!(
                "  {} {:.2}%",
                pad_display_width(&t!("labels.gross_margin"), LABEL_WIDTH).dimmed(),
                gm
            );
        }
        if let Some(nm) = self.net_margin {
            println!(
                "  {} {:.2}%",
                pad_display_width(&t!("labels.net_margin"), LABEL_WIDTH).dimmed(),
                nm
            );
        }
        println!("{}", "─".repeat(55));
        if let Some(rev) = self.revenue {
            println!(
                "  {} {}",
                pad_display_width(&t!("labels.revenue"), LABEL_WIDTH).dimmed(),
                format_amount(rev)
            );
        }
        if let Some(profit) = self.net_profit {
            println!(
                "  {} {}",
                pad_display_width(&t!("labels.net_profit"), LABEL_WIDTH).dimmed(),
                format_amount(profit)
            );
        }
        if let Some(yoy) = self.revenue_yoy {
            println!(
                "  {} {:.2}%",
                pad_display_width(&t!("labels.revenue_yoy"), LABEL_WIDTH).dimmed(),
                yoy
            );
        }
        if let Some(yoy) = self.profit_yoy {
            println!(
                "  {} {:.2}%",
                pad_display_width(&t!("labels.profit_yoy"), LABEL_WIDTH).dimmed(),
                yoy
            );
        }
    }
}

impl TableRow for TopHolder {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new(t!("headers.rank")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.holder")),
            Cell::new(t!("headers.quantity")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.ratio")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.change")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.type")),
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
            Cell::new(t!("headers.code")),
            Cell::new(t!("headers.stock")),
            Cell::new(t!("headers.date")),
            Cell::new(t!("headers.fund_name")),
            Cell::new(t!("headers.shares")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.ratio")).set_alignment(CellAlignment::Right),
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
            Cell::new(t!("headers.report_date")),
            Cell::new(t!("headers.ex_date")),
            Cell::new(t!("headers.dividend")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.bonus")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.transfer")).set_alignment(CellAlignment::Right),
            Cell::new(t!("headers.yield_pct")).set_alignment(CellAlignment::Right),
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
