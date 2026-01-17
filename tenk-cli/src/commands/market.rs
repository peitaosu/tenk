//! Market data command handlers.

use anyhow::Result;
use comfy_table::{Cell, CellAlignment};
use tenk::sources::EastMoneySource;
use tenk::traits::{
    BillboardSource, BlockTradeSource, CapitalFlowSource, EarningsForecastSource,
    IPOSource, InstitutionalResearchSource, MarginTradingSource, ResearchReportSource,
    StockConnectSource,
};
use tenk::{
    BillboardDetail, BillboardItem, BlockTradeData, CapitalFlowData, CapitalFlowHistory,
    EarningsForecast, IPOData, InstitutionalResearchData, MarginTradingData, ResearchReportData,
    StockConnectData,
};

use crate::MarketAction;
use crate::output::{
    OutputConfig, TableRow, change_pct_cell, format_amount, print_output, right_cell,
};

/// Handles market commands.
pub async fn handle(action: MarketAction, config: &OutputConfig) -> Result<()> {
    let source = EastMoneySource::default();

    match action {
        MarketAction::Flow { symbols } => {
            let refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
            let data = source.get_capital_flow(&refs).await?;
            print_output(&data, config);
        }
        MarketAction::FlowHistory { symbol, limit } => {
            let data = source.get_capital_flow_history(&symbol, limit).await?;
            print_output(&data, config);
        }
        MarketAction::Billboard { date } => {
            let data = source.get_billboard_list(date.as_deref()).await?;
            print_output(&data, config);
        }
        MarketAction::BillboardDetail { symbol, date } => {
            let data = source.get_billboard_detail(&symbol, &date).await?;
            print_output(&data, config);
        }
        MarketAction::Forecast { period, page, limit } => {
            let data = source.get_earnings_forecast(period.as_deref(), page, limit).await?;
            print_output(&data, config);
        }
        MarketAction::Connect { limit } => {
            let data = source.get_stock_connect(limit).await?;
            print_output(&data, config);
        }
        MarketAction::Margin { symbol, limit } => {
            let data = source.get_margin_trading(&symbol, limit).await?;
            print_output(&data, config);
        }
        MarketAction::Ipo { limit } => {
            let data = source.get_ipo_list(limit).await?;
            print_output(&data, config);
        }
        MarketAction::Block { limit } => {
            let data = source.get_block_trades(limit).await?;
            print_output(&data, config);
        }
        MarketAction::Research { limit } => {
            let data = source.get_institutional_research(limit).await?;
            print_output(&data, config);
        }
        MarketAction::Report { symbol, limit } => {
            let data = source.get_research_reports(symbol.as_deref(), limit).await?;
            print_output(&data, config);
        }
    }
    Ok(())
}

impl TableRow for CapitalFlowData {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Code"),
            Cell::new("Name"),
            Cell::new("MainNet").set_alignment(CellAlignment::Right),
            Cell::new("MainIn").set_alignment(CellAlignment::Right),
            Cell::new("MainOut").set_alignment(CellAlignment::Right),
            Cell::new("Ratio%").set_alignment(CellAlignment::Right),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.stock_code),
            Cell::new(&self.stock_name),
            right_cell(format_amount(self.main_net_inflow)),
            right_cell(format_amount(self.main_inflow)),
            right_cell(format_amount(self.main_outflow)),
            change_pct_cell(self.main_net_ratio),
        ]
    }
}

impl TableRow for CapitalFlowHistory {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Date"),
            Cell::new("MainNet").set_alignment(CellAlignment::Right),
            Cell::new("SuperLarge").set_alignment(CellAlignment::Right),
            Cell::new("Large").set_alignment(CellAlignment::Right),
            Cell::new("Close").set_alignment(CellAlignment::Right),
            Cell::new("Change%").set_alignment(CellAlignment::Right),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(self.trade_date.to_string()),
            right_cell(format_amount(self.main_net_inflow)),
            right_cell(format_amount(self.super_large_net_inflow)),
            right_cell(format_amount(self.large_net_inflow)),
            right_cell(format!("{:.2}", self.close)),
            change_pct_cell(self.change_pct),
        ]
    }
}

impl TableRow for BillboardItem {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Code"),
            Cell::new("Name"),
            Cell::new("Date"),
            Cell::new("NetBuy").set_alignment(CellAlignment::Right),
            Cell::new("Buy").set_alignment(CellAlignment::Right),
            Cell::new("Sell").set_alignment(CellAlignment::Right),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.stock_code),
            Cell::new(&self.stock_name),
            Cell::new(self.trade_date.to_string()),
            right_cell(format_amount(self.net_buy_amount)),
            right_cell(format_amount(self.buy_amount)),
            right_cell(format_amount(self.sell_amount)),
        ]
    }
}

impl TableRow for BillboardDetail {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Code"),
            Cell::new("Date"),
            Cell::new("Trader"),
            Cell::new("Buy").set_alignment(CellAlignment::Right),
            Cell::new("Sell").set_alignment(CellAlignment::Right),
            Cell::new("Net").set_alignment(CellAlignment::Right),
            Cell::new("Dir"),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.stock_code),
            Cell::new(self.trade_date.to_string()),
            Cell::new(&self.trader_name),
            right_cell(format_amount(self.buy_amount)),
            right_cell(format_amount(self.sell_amount)),
            right_cell(format_amount(self.net_amount)),
            Cell::new(&self.direction),
        ]
    }
}

impl TableRow for EarningsForecast {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Code"),
            Cell::new("Name"),
            Cell::new("Type"),
            Cell::new("ChangeMin%").set_alignment(CellAlignment::Right),
            Cell::new("ChangeMax%").set_alignment(CellAlignment::Right),
            Cell::new("Period"),
            Cell::new("Announce"),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.stock_code),
            Cell::new(&self.stock_name),
            Cell::new(&self.forecast_type),
            right_cell(self.change_min.map(|v| format!("{:.1}%", v)).unwrap_or_default()),
            right_cell(self.change_max.map(|v| format!("{:.1}%", v)).unwrap_or_default()),
            Cell::new(&self.report_period),
            Cell::new(self.announce_date.to_string()),
        ]
    }
}

impl TableRow for StockConnectData {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Date"),
            Cell::new("NorthNet").set_alignment(CellAlignment::Right),
            Cell::new("SHNet").set_alignment(CellAlignment::Right),
            Cell::new("SZNet").set_alignment(CellAlignment::Right),
            Cell::new("NorthBuy").set_alignment(CellAlignment::Right),
            Cell::new("NorthSell").set_alignment(CellAlignment::Right),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(self.trade_date.to_string()),
            right_cell(format_amount(self.north_net_buy)),
            right_cell(format_amount(self.sh_net_buy)),
            right_cell(format_amount(self.sz_net_buy)),
            right_cell(format_amount(self.north_buy)),
            right_cell(format_amount(self.north_sell)),
        ]
    }
}

impl TableRow for MarginTradingData {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Code"),
            Cell::new("Name"),
            Cell::new("Date"),
            Cell::new("MarginBal").set_alignment(CellAlignment::Right),
            Cell::new("ShortBal").set_alignment(CellAlignment::Right),
            Cell::new("Total").set_alignment(CellAlignment::Right),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.stock_code),
            Cell::new(&self.stock_name),
            Cell::new(self.trade_date.to_string()),
            right_cell(format_amount(self.margin_balance)),
            right_cell(format_amount(self.short_balance)),
            right_cell(format_amount(self.total_balance)),
        ]
    }
}

impl TableRow for IPOData {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Code"),
            Cell::new("Name"),
            Cell::new("Price").set_alignment(CellAlignment::Right),
            Cell::new("SubDate"),
            Cell::new("ListDate"),
            Cell::new("IssueQty").set_alignment(CellAlignment::Right),
            Cell::new("WinRate%").set_alignment(CellAlignment::Right),
            Cell::new("PE").set_alignment(CellAlignment::Right),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.stock_code),
            Cell::new(&self.stock_name),
            right_cell(format!("{:.2}", self.issue_price)),
            Cell::new(self.sub_date.to_string()),
            Cell::new(self.list_date.map(|d| d.to_string()).unwrap_or_default()),
            right_cell(self.issue_quantity.map(|v| format_amount(v as f64)).unwrap_or_default()),
            right_cell(self.winning_rate.map(|v| format!("{:.4}%", v)).unwrap_or_default()),
            right_cell(self.pe_ratio.map(|v| format!("{:.2}", v)).unwrap_or_default()),
        ]
    }
}

impl TableRow for BlockTradeData {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Code"),
            Cell::new("Name"),
            Cell::new("Date"),
            Cell::new("Price").set_alignment(CellAlignment::Right),
            Cell::new("Premium%").set_alignment(CellAlignment::Right),
            Cell::new("Amount").set_alignment(CellAlignment::Right),
            Cell::new("Buyer"),
            Cell::new("Seller"),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.stock_code),
            Cell::new(&self.stock_name),
            Cell::new(self.trade_date.to_string()),
            right_cell(format!("{:.2}", self.price)),
            change_pct_cell(self.premium_rate),
            right_cell(format_amount(self.amount)),
            Cell::new(&self.buyer),
            Cell::new(&self.seller),
        ]
    }
}

impl TableRow for InstitutionalResearchData {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Code"),
            Cell::new("Name"),
            Cell::new("Date"),
            Cell::new("Count").set_alignment(CellAlignment::Right),
            Cell::new("Institutions"),
            Cell::new("Type"),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.stock_code),
            Cell::new(&self.stock_name),
            Cell::new(self.research_date.to_string()),
            right_cell(self.institution_count),
            Cell::new(&self.institutions),
            Cell::new(&self.research_type),
        ]
    }
}

impl TableRow for ResearchReportData {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Code"),
            Cell::new("Name"),
            Cell::new("Title"),
            Cell::new("Institution"),
            Cell::new("Rating"),
            Cell::new("Date"),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        vec![
            Cell::new(&self.stock_code),
            Cell::new(&self.stock_name),
            Cell::new(&self.title),
            Cell::new(&self.institution),
            Cell::new(self.rating.as_deref().unwrap_or("-")),
            Cell::new(self.publish_date.to_string()),
        ]
    }
}
