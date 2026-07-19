use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FinancialReportKind {
    BalanceSheet,
    IncomeStatement,
    CashFlow,
    PerformanceSummary,
}

impl FinancialReportKind {
    pub fn eastmoney_report_name(self) -> &'static str {
        match self {
            Self::BalanceSheet => "RPT_DMSK_FN_BALANCE",
            Self::IncomeStatement => "RPT_DMSK_FN_INCOME",
            Self::CashFlow => "RPT_DMSK_FN_CASHFLOW",
            Self::PerformanceSummary => "RPT_LICO_FN_CPD",
        }
    }

    pub fn sort_column(self) -> &'static str {
        match self {
            Self::PerformanceSummary => "REPORTDATE",
            _ => "REPORT_DATE",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialRecord {
    pub stock_code: String,
    pub stock_name: String,
    pub report_date: NaiveDate,
    pub kind: FinancialReportKind,
    pub values: Vec<(String, f64)>,
}
