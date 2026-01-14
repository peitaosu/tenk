//! Bond data structures.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertibleBondCode {
    pub bond_code: String,
    pub bond_name: String,
    pub stock_code: String,
    pub short_name: String,
    pub sub_date: Option<NaiveDate>,
    pub issue_amount: Option<f64>,
    pub listing_date: Option<NaiveDate>,
    pub expire_date: Option<NaiveDate>,
    pub convert_price: Option<f64>,
}

impl ConvertibleBondCode {
    pub fn new(bond_code: String, bond_name: String, stock_code: String) -> Self {
        Self {
            bond_code,
            bond_name,
            stock_code,
            short_name: String::new(),
            sub_date: None,
            issue_amount: None,
            listing_date: None,
            expire_date: None,
            convert_price: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondCurrentData {
    pub bond_code: String,
    pub bond_name: String,
    pub price: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub pre_close: f64,
    pub change: f64,
    pub change_pct: f64,
    pub volume: u64,
    pub amount: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bond_code_creation() {
        let bond = ConvertibleBondCode::new(
            "127046".to_string(),
            "百润转债".to_string(),
            "002568".to_string(),
        );
        assert_eq!(bond.bond_code, "127046");
        assert_eq!(bond.stock_code, "002568");
    }
}
