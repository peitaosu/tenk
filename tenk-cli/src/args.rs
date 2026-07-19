use clap::ValueEnum;
use tenk::{BoardCrosswalkKind, FinancialReportKind, LimitPoolKind, MarketData, OptionExchange};

#[derive(Clone, Copy, ValueEnum, Debug)]
pub enum BoardKindArg {
    Industry,
    Concept,
}

impl From<BoardKindArg> for BoardCrosswalkKind {
    fn from(value: BoardKindArg) -> Self {
        match value {
            BoardKindArg::Industry => BoardCrosswalkKind::Industry,
            BoardKindArg::Concept => BoardCrosswalkKind::Concept,
        }
    }
}

impl BoardKindArg {
    pub fn parse(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "concept" => Self::Concept,
            _ => Self::Industry,
        }
    }
}

#[derive(Clone, Copy, ValueEnum, Debug)]
pub enum PoolKindArg {
    LimitUp,
    LimitDown,
    YesterdayLimitUp,
    Strong,
    SubNew,
    BrokenBoard,
}

impl From<PoolKindArg> for LimitPoolKind {
    fn from(value: PoolKindArg) -> Self {
        match value {
            PoolKindArg::LimitUp => LimitPoolKind::LimitUp,
            PoolKindArg::LimitDown => LimitPoolKind::LimitDown,
            PoolKindArg::YesterdayLimitUp => LimitPoolKind::YesterdayLimitUp,
            PoolKindArg::Strong => LimitPoolKind::Strong,
            PoolKindArg::SubNew => LimitPoolKind::SubNew,
            PoolKindArg::BrokenBoard => LimitPoolKind::BrokenBoard,
        }
    }
}

impl PoolKindArg {
    pub fn parse(name: &str) -> Self {
        match name.to_lowercase().replace('_', "-").as_str() {
            "limit-down" | "limitdown" => Self::LimitDown,
            "yesterday-limit-up" | "yesterday" => Self::YesterdayLimitUp,
            "strong" => Self::Strong,
            "sub-new" | "subnew" => Self::SubNew,
            "broken-board" | "broken" => Self::BrokenBoard,
            _ => Self::LimitUp,
        }
    }
}

#[derive(Clone, Copy, ValueEnum, Debug)]
pub enum FinancialKindArg {
    Balance,
    Income,
    Cashflow,
    Performance,
}

impl From<FinancialKindArg> for FinancialReportKind {
    fn from(value: FinancialKindArg) -> Self {
        match value {
            FinancialKindArg::Balance => FinancialReportKind::BalanceSheet,
            FinancialKindArg::Income => FinancialReportKind::IncomeStatement,
            FinancialKindArg::Cashflow => FinancialReportKind::CashFlow,
            FinancialKindArg::Performance => FinancialReportKind::PerformanceSummary,
        }
    }
}

impl FinancialKindArg {
    pub fn parse(name: &str) -> Self {
        match name.to_lowercase().replace('_', "-").as_str() {
            "income" | "income-statement" => Self::Income,
            "cashflow" | "cash-flow" => Self::Cashflow,
            "performance" | "summary" => Self::Performance,
            _ => Self::Balance,
        }
    }
}

#[derive(Clone, Copy, ValueEnum, Debug)]
pub enum OptionExchangeArg {
    Sse,
    Szse,
    Cffex,
}

impl From<OptionExchangeArg> for OptionExchange {
    fn from(value: OptionExchangeArg) -> Self {
        match value {
            OptionExchangeArg::Sse => OptionExchange::Sse,
            OptionExchangeArg::Szse => OptionExchange::Szse,
            OptionExchangeArg::Cffex => OptionExchange::Cffex,
        }
    }
}

impl OptionExchangeArg {
    pub fn parse(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "szse" | "sz" => Self::Szse,
            "cffex" => Self::Cffex,
            _ => Self::Sse,
        }
    }
}

pub fn limit_kline(mut data: Vec<MarketData>, limit: Option<usize>) -> Vec<MarketData> {
    if let Some(n) = limit {
        let len = data.len();
        if n < len {
            data = data.split_off(len - n);
        }
    }
    data
}
