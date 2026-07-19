//! Stock data structures.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString, VariantNames};

/// Stock exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Exchange {
    /// Shanghai Stock Exchange
    SH,
    /// Shenzhen Stock Exchange
    SZ,
    /// Beijing Stock Exchange
    BJ,
    /// Unknown exchange
    Unknown,
}

impl Exchange {
    /// Determines exchange from stock code prefix.
    pub fn from_stock_code(code: &str) -> Self {
        let first_two: String = code.chars().take(2).collect();
        match first_two.as_str() {
            // Shanghai: 6xxxxx stocks, 11xxxx bonds, 5xxxxx ETFs
            s if s.starts_with('6') => Exchange::SH,
            "11" => Exchange::SH,
            s if s.starts_with('5') => Exchange::SH,
            // Shenzhen: 0xxxxx/3xxxxx stocks, 12xxxx bonds, 1xxxxx ETFs
            s if s.starts_with('0') || s.starts_with('3') => Exchange::SZ,
            "12" => Exchange::SZ,
            s if s.starts_with('1') && !s.starts_with("11") => Exchange::SZ,
            // Beijing: 4xxxxx/8xxxxx
            s if s.starts_with('4') || s.starts_with('8') => Exchange::BJ,
            _ => Exchange::Unknown,
        }
    }

    /// Returns the market prefix string.
    pub fn market_prefix(&self) -> &'static str {
        match self {
            Exchange::SH => "sh",
            Exchange::SZ => "sz",
            Exchange::BJ => "bj",
            Exchange::Unknown => "",
        }
    }

    /// Converts a stock code to EastMoney secid format.
    pub fn eastmoney_secid(stock_code: &str) -> String {
        let prefix = if stock_code.starts_with('6') || stock_code.starts_with('5') {
            "1"
        } else {
            "0"
        };
        format!("{prefix}.{stock_code}")
    }
}

impl std::fmt::Display for Exchange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Exchange::SH => write!(f, "SH"),
            Exchange::SZ => write!(f, "SZ"),
            Exchange::BJ => write!(f, "BJ"),
            Exchange::Unknown => write!(f, "Unknown"),
        }
    }
}

/// K-line (candlestick) time period.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    AsRefStr,
    EnumString,
    VariantNames,
)]
#[strum(serialize_all = "snake_case")]
pub enum KLineType {
    /// Daily K-line
    Daily = 1,
    /// Weekly K-line
    Weekly = 2,
    /// Monthly K-line
    Monthly = 3,
    /// Quarterly K-line
    Quarterly = 4,
    /// 5-minute K-line
    Min5 = 5,
    /// 15-minute K-line
    Min15 = 15,
    /// 30-minute K-line
    Min30 = 30,
    /// 60-minute K-line
    Min60 = 60,
}

impl KLineType {
    /// Converts to API value code.
    pub fn to_api_value(self) -> u32 {
        match self {
            KLineType::Daily => 101,
            KLineType::Weekly => 102,
            KLineType::Monthly => 103,
            KLineType::Quarterly => 104,
            KLineType::Min5 => 5,
            KLineType::Min15 => 15,
            KLineType::Min30 => 30,
            KLineType::Min60 => 60,
        }
    }

    /// Parses from CLI/MCP string (defaults to daily).
    pub fn from_name(name: &str) -> Self {
        name.parse().unwrap_or(Self::Daily)
    }
}

/// Price adjustment type for historical data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AdjustType {
    /// No adjustment
    None = 0,
    /// Forward adjusted
    #[default]
    Forward = 1,
    /// Backward adjusted
    Backward = 2,
}

/// Stock basic information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockCode {
    /// Stock code
    pub stock_code: String,
    /// Short name
    pub short_name: String,
    /// Exchange
    pub exchange: Exchange,
    /// Listing date
    pub list_date: Option<NaiveDate>,
}

impl StockCode {
    /// Creates a new stock code.
    pub fn new(stock_code: String, short_name: String, exchange: Exchange) -> Self {
        Self {
            stock_code,
            short_name,
            exchange,
            list_date: None,
        }
    }

    /// Returns the full symbol with exchange prefix.
    pub fn full_symbol(&self) -> String {
        format!("{}{}", self.exchange.market_prefix(), self.stock_code)
    }
}

/// Historical market data (K-line).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    /// Stock code
    pub stock_code: String,
    /// Trade time
    pub trade_time: DateTime<Utc>,
    /// Trade date
    pub trade_date: NaiveDate,
    /// Open price
    pub open: f64,
    /// Close price
    pub close: f64,
    /// High price
    pub high: f64,
    /// Low price
    pub low: f64,
    /// Volume
    pub volume: u64,
    /// Amount
    pub amount: f64,
    /// Price change
    pub change: f64,
    /// Change percentage
    pub change_pct: f64,
    /// Turnover ratio
    pub turnover_ratio: f64,
    /// Previous close
    pub pre_close: f64,
}

impl MarketData {
    /// Returns true if data is valid.
    pub fn is_valid(&self) -> bool {
        self.volume > 0 && self.amount > 0.0
    }
}

/// Real-time stock market data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentMarketData {
    /// Stock code
    pub stock_code: String,
    /// Short name
    pub short_name: String,
    /// Current price
    pub price: f64,
    /// Price change
    pub change: f64,
    /// Change percentage
    pub change_pct: f64,
    /// Volume
    pub volume: u64,
    /// Amount
    pub amount: f64,
    /// Open price
    pub open: Option<f64>,
    /// High price
    pub high: Option<f64>,
    /// Low price
    pub low: Option<f64>,
    /// Previous close
    pub pre_close: Option<f64>,
}

/// Intraday minute-level data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinuteData {
    /// Stock code
    pub stock_code: String,
    /// Trade time
    pub trade_time: DateTime<Utc>,
    /// Current price
    pub price: f64,
    /// Price change
    pub change: f64,
    /// Change percentage
    pub change_pct: f64,
    /// Volume
    pub volume: u64,
    /// Average price
    pub avg_price: f64,
    /// Amount
    pub amount: f64,
}

/// Level 1 order book data (5 levels).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookData {
    /// Stock code
    pub stock_code: String,
    /// Short name
    pub short_name: String,
    /// Sell prices (5 levels)
    pub sell_prices: [f64; 5],
    /// Sell volumes (5 levels)
    pub sell_volumes: [u64; 5],
    /// Buy prices (5 levels)
    pub buy_prices: [f64; 5],
    /// Buy volumes (5 levels)
    pub buy_volumes: [u64; 5],
}

/// Tick-by-tick transaction data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickData {
    /// Stock code
    pub stock_code: String,
    /// Trade time
    pub trade_time: DateTime<Utc>,
    /// Trade price
    pub price: f64,
    /// Trade volume
    pub volume: u64,
    /// Trade direction
    pub direction: char,
}

/// Detailed stock information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockInfo {
    /// Stock code
    pub stock_code: String,
    /// Full company name
    pub full_name: String,
    /// Short name
    pub short_name: String,
    /// Exchange
    pub exchange: Exchange,
    /// Industry sector
    pub industry: Option<String>,
    /// Total shares
    pub total_shares: Option<u64>,
    /// Circulating shares
    pub circulating_shares: Option<u64>,
    /// Listing date
    pub list_date: Option<NaiveDate>,
}

/// Capital flow data for a stock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapitalFlowData {
    /// Stock code
    pub stock_code: String,
    /// Stock name
    pub stock_name: String,
    /// Main capital net inflow
    pub main_net_inflow: f64,
    /// Main capital inflow
    pub main_inflow: f64,
    /// Main capital outflow
    pub main_outflow: f64,
    /// Super large order net inflow
    pub super_large_net_inflow: f64,
    /// Large order net inflow
    pub large_net_inflow: f64,
    /// Medium order net inflow
    pub medium_net_inflow: f64,
    /// Small order net inflow
    pub small_net_inflow: f64,
    /// Main capital net inflow ratio
    pub main_net_ratio: f64,
}

/// Historical capital flow data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapitalFlowHistory {
    /// Stock code
    pub stock_code: String,
    /// Trade date
    pub trade_date: NaiveDate,
    /// Main capital net inflow
    pub main_net_inflow: f64,
    /// Small order net inflow
    pub small_net_inflow: f64,
    /// Medium order net inflow
    pub medium_net_inflow: f64,
    /// Large order net inflow
    pub large_net_inflow: f64,
    /// Super large order net inflow
    pub super_large_net_inflow: f64,
    /// Close price
    pub close: f64,
    /// Change percentage
    pub change_pct: f64,
}

/// Billboard item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillboardItem {
    /// Stock code
    pub stock_code: String,
    /// Stock name
    pub stock_name: String,
    /// Trade date
    pub trade_date: NaiveDate,
    /// Close price
    pub close: f64,
    /// Change percentage
    pub change_pct: f64,
    /// Turnover ratio
    pub turnover_ratio: f64,
    /// Net buy amount
    pub net_buy_amount: f64,
    /// Buy amount
    pub buy_amount: f64,
    /// Sell amount
    pub sell_amount: f64,
    /// Reason for listing
    pub reason: String,
}

/// Billboard institution detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillboardDetail {
    /// Stock code
    pub stock_code: String,
    /// Trade date
    pub trade_date: NaiveDate,
    /// Institution/broker name
    pub trader_name: String,
    /// Buy amount
    pub buy_amount: f64,
    /// Sell amount
    pub sell_amount: f64,
    /// Net amount
    pub net_amount: f64,
    /// Trade direction
    pub direction: String,
}

/// Earnings forecast data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarningsForecast {
    /// Stock code
    pub stock_code: String,
    /// Stock name
    pub stock_name: String,
    /// Forecast type
    pub forecast_type: String,
    /// Forecast net profit lower bound
    pub profit_min: Option<f64>,
    /// Forecast net profit upper bound
    pub profit_max: Option<f64>,
    /// YoY change lower bound
    pub change_min: Option<f64>,
    /// YoY change upper bound
    pub change_max: Option<f64>,
    /// Report period
    pub report_period: String,
    /// Announcement date
    pub announce_date: NaiveDate,
    /// Forecast summary
    pub summary: Option<String>,
}

/// Stock Connect flow data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockConnectData {
    /// Trade date
    pub trade_date: NaiveDate,
    /// Northbound net buy
    pub north_net_buy: f64,
    /// Shanghai Connect net buy
    pub sh_net_buy: f64,
    /// Shenzhen Connect net buy
    pub sz_net_buy: f64,
    /// Northbound buy amount
    pub north_buy: f64,
    /// Northbound sell amount
    pub north_sell: f64,
}

/// Margin trading data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginTradingData {
    /// Stock code
    pub stock_code: String,
    /// Stock name
    pub stock_name: String,
    /// Trade date
    pub trade_date: NaiveDate,
    /// Margin balance
    pub margin_balance: f64,
    /// Margin buy amount
    pub margin_buy: f64,
    /// Margin repay amount
    pub margin_repay: f64,
    /// Short selling balance
    pub short_balance: f64,
    /// Short selling volume
    pub short_volume: u64,
    /// Total balance
    pub total_balance: f64,
}

/// IPO information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPOData {
    /// Stock code
    pub stock_code: String,
    /// Stock name
    pub stock_name: String,
    /// Issue price
    pub issue_price: f64,
    /// Subscription date
    pub sub_date: NaiveDate,
    /// Listing date
    pub list_date: Option<NaiveDate>,
    /// Winning rate
    pub winning_rate: Option<f64>,
    /// Issue quantity
    pub issue_quantity: Option<u64>,
    /// Online issue quantity
    pub online_quantity: Option<u64>,
    /// PE ratio
    pub pe_ratio: Option<f64>,
}

/// Block trade data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockTradeData {
    /// Stock code
    pub stock_code: String,
    /// Stock name
    pub stock_name: String,
    /// Trade date
    pub trade_date: NaiveDate,
    /// Trade price
    pub price: f64,
    /// Close price
    pub close_price: f64,
    /// Premium rate
    pub premium_rate: f64,
    /// Trade volume
    pub volume: u64,
    /// Trade amount
    pub amount: f64,
    /// Buyer broker
    pub buyer: String,
    /// Seller broker
    pub seller: String,
}

/// Institutional research data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstitutionalResearchData {
    /// Stock code
    pub stock_code: String,
    /// Stock name
    pub stock_name: String,
    /// Research date
    pub research_date: NaiveDate,
    /// Institution count
    pub institution_count: u32,
    /// Institution names
    pub institutions: String,
    /// Research type
    pub research_type: String,
    /// Researchers
    pub researchers: Option<String>,
}

/// Research report data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchReportData {
    /// Report ID
    pub report_id: String,
    /// Stock code
    pub stock_code: String,
    /// Stock name
    pub stock_name: String,
    /// Report title
    pub title: String,
    /// Institution name
    pub institution: String,
    /// Analyst names
    pub analysts: String,
    /// Rating
    pub rating: Option<String>,
    /// Publish date
    pub publish_date: NaiveDate,
}

/// Stock valuation metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockValuation {
    /// Stock code
    pub stock_code: String,
    /// Stock name
    pub stock_name: String,
    /// Current price
    pub price: f64,
    /// Market capitalization
    pub market_cap: f64,
    /// Circulating market cap
    pub float_cap: f64,
    /// PE ratio (TTM)
    pub pe_ttm: Option<f64>,
    /// PE ratio (static)
    pub pe_static: Option<f64>,
    /// PB ratio
    pub pb: Option<f64>,
    /// PS ratio
    pub ps: Option<f64>,
    /// EPS
    pub eps: Option<f64>,
    /// BPS
    pub bps: Option<f64>,
    /// ROE
    pub roe: Option<f64>,
    /// Gross margin
    pub gross_margin: Option<f64>,
    /// Net margin
    pub net_margin: Option<f64>,
    /// Revenue
    pub revenue: Option<f64>,
    /// Net profit
    pub net_profit: Option<f64>,
    /// Revenue YoY growth
    pub revenue_yoy: Option<f64>,
    /// Net profit YoY growth
    pub profit_yoy: Option<f64>,
}

/// Top shareholder data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopHolder {
    /// Stock code
    pub stock_code: String,
    /// Report date
    pub report_date: NaiveDate,
    /// Holder rank
    pub rank: u32,
    /// Holder name
    pub holder_name: String,
    /// Hold quantity
    pub hold_quantity: u64,
    /// Hold ratio
    pub hold_ratio: f64,
    /// Change quantity
    pub change_quantity: Option<i64>,
    /// Holder type
    pub holder_type: String,
}

/// Fund holding data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundHolding {
    /// Stock code
    pub stock_code: String,
    /// Stock name
    pub stock_name: String,
    /// Report date
    pub report_date: NaiveDate,
    /// Fund name
    pub fund_name: String,
    /// Holding shares
    pub hold_shares: u64,
    /// Holding ratio
    pub hold_ratio: f64,
}

/// Dividend history data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DividendData {
    /// Stock code
    pub stock_code: String,
    /// Stock name
    pub stock_name: String,
    /// Report date
    pub report_date: NaiveDate,
    /// Ex-dividend date
    pub ex_date: Option<NaiveDate>,
    /// Record date
    pub record_date: Option<NaiveDate>,
    /// Cash dividend per share
    pub dividend_per_share: f64,
    /// Bonus shares per 10 shares
    pub bonus_shares: f64,
    /// Transfer shares per 10 shares
    pub transfer_shares: f64,
    /// Dividend yield
    pub dividend_yield: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exchange_from_code() {
        assert_eq!(Exchange::from_stock_code("600000"), Exchange::SH);
        assert_eq!(Exchange::from_stock_code("000001"), Exchange::SZ);
        assert_eq!(Exchange::from_stock_code("300001"), Exchange::SZ);
        assert_eq!(Exchange::from_stock_code("830001"), Exchange::BJ);
    }

    #[test]
    fn test_stock_code_full_symbol() {
        let stock = StockCode::new("600519".to_string(), "贵州茅台".to_string(), Exchange::SH);
        assert_eq!(stock.full_symbol(), "sh600519");
    }

    #[test]
    fn test_kline_type_api_value() {
        assert_eq!(KLineType::Daily.to_api_value(), 101);
        assert_eq!(KLineType::Weekly.to_api_value(), 102);
        assert_eq!(KLineType::Monthly.to_api_value(), 103);
        assert_eq!(KLineType::Quarterly.to_api_value(), 104);
        assert_eq!(KLineType::Min5.to_api_value(), 5);
        assert_eq!(KLineType::Min15.to_api_value(), 15);
        assert_eq!(KLineType::Min30.to_api_value(), 30);
        assert_eq!(KLineType::Min60.to_api_value(), 60);
    }

    #[test]
    fn test_kline_type_from_name() {
        assert_eq!(KLineType::from_name("daily"), KLineType::Daily);
        assert_eq!(KLineType::from_name("weekly"), KLineType::Weekly);
        assert_eq!(KLineType::from_name("min5"), KLineType::Min5);
        assert_eq!(KLineType::from_name("unknown"), KLineType::Daily);
    }

    #[test]
    fn test_exchange_market_prefix() {
        assert_eq!(Exchange::SH.market_prefix(), "sh");
        assert_eq!(Exchange::SZ.market_prefix(), "sz");
        assert_eq!(Exchange::BJ.market_prefix(), "bj");
        assert_eq!(Exchange::Unknown.market_prefix(), "");
    }

    #[test]
    fn test_exchange_display() {
        assert_eq!(Exchange::SH.to_string(), "SH");
        assert_eq!(Exchange::SZ.to_string(), "SZ");
    }

    #[test]
    fn test_eastmoney_secid() {
        assert_eq!(Exchange::eastmoney_secid("600519"), "1.600519");
        assert_eq!(Exchange::eastmoney_secid("300059"), "0.300059");
        assert_eq!(Exchange::eastmoney_secid("510300"), "1.510300");
    }

    #[test]
    fn test_stock_code_sz_full_symbol() {
        let stock = StockCode::new("300059".to_string(), "东方财富".to_string(), Exchange::SZ);
        assert_eq!(stock.full_symbol(), "sz300059");
    }

    #[test]
    fn test_market_data_is_valid() {
        let valid = MarketData {
            stock_code: "600519".to_string(),
            trade_time: Utc::now(),
            trade_date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
            open: 100.0,
            close: 101.0,
            high: 102.0,
            low: 99.0,
            volume: 1000,
            amount: 100000.0,
            change: 1.0,
            change_pct: 1.0,
            turnover_ratio: 0.5,
            pre_close: 100.0,
        };
        assert!(valid.is_valid());

        let invalid = MarketData {
            volume: 0,
            amount: 0.0,
            ..valid
        };
        assert!(!invalid.is_valid());
    }
}
