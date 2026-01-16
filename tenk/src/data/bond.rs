//! Bond data structures.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Convertible bond information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertibleBondCode {
    /// Bond code
    pub bond_code: String,
    /// Bond name
    pub bond_name: String,
    /// Underlying stock code
    pub stock_code: String,
    /// Short name
    pub short_name: String,
    /// Subscription date
    pub sub_date: Option<NaiveDate>,
    /// Issue amount
    pub issue_amount: Option<f64>,
    /// Listing date
    pub listing_date: Option<NaiveDate>,
    /// Expiration date
    pub expire_date: Option<NaiveDate>,
    /// Conversion price
    pub convert_price: Option<f64>,
}

impl ConvertibleBondCode {
    /// Creates a new convertible bond code.
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

/// Real-time bond market data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondCurrentData {
    /// Bond code
    pub bond_code: String,
    /// Bond name
    pub bond_name: String,
    /// Current price
    pub price: f64,
    /// Open price
    pub open: f64,
    /// High price
    pub high: f64,
    /// Low price
    pub low: f64,
    /// Previous close
    pub pre_close: f64,
    /// Price change
    pub change: f64,
    /// Change percentage
    pub change_pct: f64,
    /// Volume
    pub volume: u64,
    /// Amount
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
