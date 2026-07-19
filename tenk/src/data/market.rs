use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::Exchange;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LimitPoolKind {
    LimitUp,
    LimitDown,
    YesterdayLimitUp,
    Strong,
    SubNew,
    BrokenBoard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BoardCrosswalkKind {
    Industry,
    Concept,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardItem {
    pub board_code: String,
    pub board_name: String,
    pub price: f64,
    pub change_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardCrosswalkItem {
    pub board_name: String,
    pub eastmoney_code: Option<String>,
    pub ths_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitPoolItem {
    pub stock_code: String,
    pub stock_name: String,
    pub price: f64,
    pub change_pct: f64,
    pub limit_price: f64,
    pub amount: f64,
    pub turnover_ratio: f64,
    pub float_market_cap: f64,
    pub total_market_cap: f64,
    pub continuous_boards: u32,
    pub first_board_time: String,
    pub last_board_time: String,
    pub board_amount: f64,
    pub industry: String,
    pub trade_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroRecord {
    pub indicator: String,
    pub period: String,
    pub report_date: NaiveDate,
    pub values: Vec<(String, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexCode {
    pub index_code: String,
    pub index_name: String,
    pub exchange: Exchange,
}

impl IndexCode {
    pub fn eastmoney_secid(&self) -> String {
        crate::util::eastmoney_secid_for_index(&self.index_code, self.exchange)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_code_secid() {
        let idx = IndexCode {
            index_code: "000001".to_string(),
            index_name: "上证指数".to_string(),
            exchange: Exchange::SH,
        };
        assert_eq!(idx.eastmoney_secid(), "1.000001");
    }
}
