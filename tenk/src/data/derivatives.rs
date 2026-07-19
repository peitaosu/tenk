use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DerivativesExchange {
    Shfe,
    Dce,
    Czce,
    Cffex,
    Ine,
    Gfex,
    SseOptions,
    SzseOptions,
    CffexOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OptionExchange {
    Sse,
    Szse,
    Cffex,
}

impl OptionExchange {
    pub fn eastmoney_fs(self) -> &'static str {
        match self {
            OptionExchange::Sse => "m:10",
            OptionExchange::Szse => "m:12",
            OptionExchange::Cffex => "m:11",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuturesContract {
    pub contract_code: String,
    pub contract_name: String,
    pub secid: String,
    pub exchange: DerivativesExchange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionContract {
    pub contract_code: String,
    pub contract_name: String,
    pub exchange: OptionExchange,
    pub price: f64,
    pub change_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivativesQuote {
    pub contract_code: String,
    pub contract_name: String,
    pub secid: String,
    pub price: f64,
    pub change: f64,
    pub change_pct: f64,
    pub volume: u64,
    pub amount: f64,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub pre_close: Option<f64>,
    pub open_interest: Option<u64>,
    pub trade_date: Option<NaiveDate>,
}

impl DerivativesExchange {
    pub fn from_market_id(id: i64) -> Self {
        match id {
            113 => Self::Shfe,
            114 => Self::Dce,
            115 => Self::Czce,
            8 => Self::Cffex,
            142 => Self::Ine,
            225 => Self::Gfex,
            10 => Self::SseOptions,
            12 => Self::SzseOptions,
            11 => Self::CffexOptions,
            _ => Self::Shfe,
        }
    }
}
