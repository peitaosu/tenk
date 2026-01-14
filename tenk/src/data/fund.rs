//! ETF data structures.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::Exchange;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ETFCode {
    pub fund_code: String,
    pub short_name: String,
    pub exchange: Exchange,
    pub net_value: Option<f64>,
}

impl ETFCode {
    pub fn new(fund_code: String, short_name: String, exchange: Exchange) -> Self {
        Self {
            fund_code,
            short_name,
            exchange,
            net_value: None,
        }
    }

    pub fn full_symbol(&self) -> String {
        format!("{}{}", self.exchange.market_prefix(), self.fund_code)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ETFMarketData {
    pub fund_code: String,
    pub trade_time: DateTime<Utc>,
    pub trade_date: NaiveDate,
    pub open: f64,
    pub close: f64,
    pub high: f64,
    pub low: f64,
    pub volume: u64,
    pub amount: f64,
    pub change: Option<f64>,
    pub change_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ETFCurrentData {
    pub fund_code: String,
    pub short_name: String,
    pub price: f64,
    pub change: Option<f64>,
    pub change_pct: Option<f64>,
    pub volume: u64,
    pub amount: f64,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ETFMinuteData {
    pub fund_code: String,
    pub trade_time: DateTime<Utc>,
    pub price: f64,
    pub change: f64,
    pub change_pct: f64,
    pub volume: u64,
    pub avg_price: f64,
    pub amount: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_etf_code_full_symbol() {
        let etf = ETFCode::new("510300".to_string(), "沪深300ETF".to_string(), Exchange::SH);
        assert_eq!(etf.full_symbol(), "sh510300");
    }
}
