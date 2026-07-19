//! Sina Finance data source.

use async_trait::async_trait;
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HOST, HeaderMap, HeaderValue, REFERER, USER_AGENT};
use serde::Deserialize;
use tracing::{debug, warn};

use crate::data::{
    BondCurrentData, ConvertibleBondCode, CurrentMarketData, DerivativesExchange,
    DerivativesQuote, ETFCode, ETFCurrentData, ETFMarketData, ETFMinuteData, Exchange, FuturesContract,
    IndexCode, KLineType, MarketData, MinuteData, OrderBookData, StockCode, StockInfo, TickData,
};
use crate::error::{DataError, DataResult};
use crate::request::{RequestConfig, RequestManager};
use crate::util::{
    decode_gb18030, kline_scale, parse_kline_records, parse_minute_records,
    parse_order_book_from_parts, parse_ticks_from_trans_list, sina_index_hq_symbol,
    SinaKLineRecord, SinaMinuteResponse,
};
use crate::traits::{
    BondInfoSource, BondMarketSource, DataSource, FundInfoSource, FundMarketSource, FuturesSource,
    IndexMarketSource, StockInfoSource, StockMarketSource,
};

/// Sina Finance data source.
#[derive(Debug, Clone)]
pub struct SinaSource {
    /// HTTP request manager
    request: RequestManager,
    /// VIP API request manager
    vip_request: RequestManager,
}

impl SinaSource {
    /// Creates a new Sina source.
    pub fn new() -> DataResult<Self> {
        Self::try_new(None)
    }

    pub fn try_new(proxy: Option<&str>) -> DataResult<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_static("hq.sinajs.cn"));
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/110.0",
            ),
        );
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_static("zh-CN,zh;q=0.8,zh-TW;q=0.7,zh-HK;q=0.5,en-US;q=0.3,en;q=0.2"),
        );
        headers.insert(
            REFERER,
            HeaderValue::from_static("https://finance.sina.com.cn"),
        );

        let hq_config = RequestConfig::default()
            .with_headers(headers)
            .with_proxy_opt(proxy);
        let vip_config = RequestConfig::default().with_proxy_opt(proxy);

        Ok(Self {
            request: RequestManager::new(hq_config)?,
            vip_request: RequestManager::new(vip_config)?,
        })
    }

    pub fn with_request_manager(request: RequestManager) -> Self {
        Self {
            request: request.clone(),
            vip_request: request,
        }
    }

    async fn decode_hq_text(response: reqwest::Response) -> DataResult<String> {
        let bytes = response.bytes().await.map_err(DataError::Network)?;
        Ok(decode_gb18030(&bytes))
    }

    fn full_symbol(stock_code: &str) -> String {
        format!("{}{}", Self::market_prefix(stock_code), stock_code)
    }

    fn market_prefix(stock_code: &str) -> &'static str {
        match Exchange::from_stock_code(stock_code) {
            Exchange::SH => "sh",
            Exchange::SZ => "sz",
            Exchange::BJ => "bj",
            Exchange::Unknown => "sh",
        }
    }

    fn parse_sina_line(line: &str, min_fields: usize) -> Option<(String, Vec<&str>)> {
        let eq_pos = line.find('=')?;
        if eq_pos < 6 {
            return None;
        }
        let code = line[eq_pos - 6..eq_pos].to_string();
        let quote_start = line.find('"')? + 1;
        let quote_end = line.rfind('"')?;
        if quote_start >= quote_end {
            return None;
        }
        let parts: Vec<&str> = line[quote_start..quote_end].split(',').collect();
        if parts.len() < min_fields {
            return None;
        }
        Some((code, parts))
    }

    fn parse_quote_line(line: &str) -> Option<CurrentMarketData> {
        let (stock_code, parts) = Self::parse_sina_line(line, 6)?;
        let short_name = parts[0].to_string();
        let price: f64 = parts[1].parse().ok()?;
        let change: f64 = parts[2].parse().ok()?;
        let change_pct: f64 = parts[3].parse().ok()?;
        let volume: u64 = parts[4].parse().ok()?;
        let amount: f64 = parts[5].parse().ok()?;
        let (adj_volume, adj_amount) = if stock_code.starts_with(['0', '3', '6', '9']) {
            (volume * 100, amount * 10000.0)
        } else {
            (volume, amount)
        };
        Some(CurrentMarketData {
            stock_code,
            short_name,
            price,
            change,
            change_pct,
            volume: adj_volume,
            amount: adj_amount,
            open: None,
            high: None,
            low: None,
            pre_close: Some(price - change),
        })
    }

    fn parse_index_quote_line(line: &str) -> Option<CurrentMarketData> {
        let (index_code, parts) = Self::parse_sina_line(line, 6)?;
        let short_name = parts[0].to_string();
        let price: f64 = parts[1].parse().ok()?;
        let change: f64 = parts[2].parse().ok()?;
        let change_pct: f64 = parts[3].parse().ok()?;
        let volume: u64 = parts[4].parse().ok()?;
        let amount: f64 = parts[5].parse().ok()?;
        Some(CurrentMarketData {
            stock_code: index_code,
            short_name,
            price,
            change,
            change_pct,
            volume,
            amount,
            open: None,
            high: None,
            low: None,
            pre_close: Some(price - change),
        })
    }

    fn parse_etf_quote_line(line: &str) -> Option<ETFCurrentData> {
        let (fund_code, parts) = Self::parse_sina_line(line, 6)?;
        let short_name = parts[0].to_string();
        let price: f64 = parts[1].parse().ok()?;
        let change: f64 = parts[2].parse().ok()?;
        let change_pct: f64 = parts[3].parse().ok()?;
        let volume: u64 = parts[4].parse().ok()?;
        let amount: f64 = parts[5].parse().ok()?;
        let (adj_volume, adj_amount) = if fund_code.starts_with(['0', '1', '5']) {
            (volume * 100, amount * 10000.0)
        } else {
            (volume, amount)
        };
        Some(ETFCurrentData {
            fund_code,
            short_name,
            price,
            change: Some(change),
            change_pct: Some(change_pct),
            volume: adj_volume,
            amount: adj_amount,
            open: None,
            high: None,
            low: None,
        })
    }
}

impl Default for SinaSource {
    /// Creates default Sina source.
    fn default() -> Self {
        Self::new().expect("Failed to create SinaSource")
    }
}

/// Stock item from Sina API.
#[derive(Debug, Deserialize)]
struct SinaStockItem {
    /// Stock code
    code: String,
    /// Stock name
    name: String,
}

/// Bond code item from Sina list API.
#[derive(Debug, Deserialize)]
struct SinaBondCodeItem {
    code: String,
    name: String,
}

/// Bond item from Sina API.
#[derive(Debug, Deserialize)]
struct SinaBondItem {
    /// Bond code
    code: String,
    /// Bond name
    name: String,
    /// Current price
    #[serde(deserialize_with = "deserialize_string_or_number")]
    trade: String,
    /// Price change
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pricechange: String,
    /// Change percentage
    #[serde(deserialize_with = "deserialize_string_or_number")]
    changepercent: String,
    /// Previous close
    #[serde(deserialize_with = "deserialize_string_or_number")]
    settlement: String,
    /// Open price
    #[serde(deserialize_with = "deserialize_string_or_number")]
    open: String,
    /// High price
    #[serde(deserialize_with = "deserialize_string_or_number")]
    high: String,
    /// Low price
    #[serde(deserialize_with = "deserialize_string_or_number")]
    low: String,
    /// Volume
    #[serde(deserialize_with = "deserialize_string_or_number")]
    volume: String,
    /// Amount
    #[serde(deserialize_with = "deserialize_string_or_number")]
    amount: String,
}

/// Deserializes a value that can be either a string or a number.
fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct StringOrNumberVisitor;

    impl<'de> Visitor<'de> for StringOrNumberVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or number")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(v.to_string())
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(v)
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(v.to_string())
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(v.to_string())
        }

        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(v.to_string())
        }
    }

    deserializer.deserialize_any(StringOrNumberVisitor)
}

#[async_trait]
impl DataSource for SinaSource {
    /// Returns the source name.
    fn name(&self) -> &'static str {
        "sina"
    }

    /// Returns the source priority.
    fn priority(&self) -> u8 {
        2
    }

    /// Checks if the source is available.
    async fn is_available(&self) -> bool {
        self.request
            .get("https://hq.sinajs.cn/list=s_sh000001")
            .await
            .is_ok()
    }
}

#[async_trait]
impl StockMarketSource for SinaSource {
    /// Fetches historical K-line market data.
    async fn get_market(
        &self,
        stock_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        let scale = kline_scale(k_type)
            .ok_or_else(|| DataError::not_supported(format!("sina: kline type {k_type:?}")))?;
        let symbol = Self::full_symbol(stock_code);
        let url = "https://money.finance.sina.com.cn/quotes_service/api/json_v2.php/CN_MarketData.getKLineData";
        let params = [
            ("symbol", symbol.as_str()),
            ("scale", &scale.to_string()),
            ("ma", "no"),
            ("datalen", "1023"),
        ];

        debug!("Fetching K-line from Sina: {} scale={}", stock_code, scale);

        let records: Vec<SinaKLineRecord> = self.vip_request.get_json_with_params(url, &params).await?;
        let intraday = scale < 240;
        Ok(parse_kline_records(
            stock_code,
            &records,
            start_date,
            end_date,
            intraday,
        ))
    }

    /// Fetches real-time market quotes.
    async fn get_market_current(&self, stock_codes: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        if stock_codes.is_empty() {
            return Ok(Vec::new());
        }

        let codes_str: String = stock_codes
            .iter()
            .map(|c| format!("s_{}{}", Self::market_prefix(c), c))
            .collect::<Vec<_>>()
            .join(",");

        let url = format!("https://hq.sinajs.cn/list={codes_str}");
        debug!("Fetching current market from Sina: {}", url);

        let response = self.request.get(&url).await?;
        let text = Self::decode_hq_text(response).await?;

        if text.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::new();

        for line in text.split(';') {
            let line = line.trim();
            if line.len() < 10 {
                continue;
            }

            if let Some(data) = Self::parse_quote_line(line) {
                result.push(data);
            }
        }

        Ok(result)
    }

    /// Fetches intraday minute-level data.
    async fn get_market_min(&self, stock_code: &str) -> DataResult<Vec<MinuteData>> {
        let symbol = Self::full_symbol(stock_code);
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let url = "https://cn.finance.sina.com.cn/minline/getMinlineData";
        let params = [
            ("symbol", symbol.as_str()),
            ("scale", "1"),
            ("date", date.as_str()),
        ];

        debug!("Fetching minute data from Sina: {}", stock_code);

        let response: SinaMinuteResponse = self.vip_request.get_json_with_params(url, &params).await?;
        let records = response
            .result
            .and_then(|r| r.data)
            .unwrap_or_default();
        let trade_date = chrono::Utc::now().date_naive();
        Ok(parse_minute_records(stock_code, trade_date, &records))
    }

    async fn get_order_book(&self, stock_code: &str) -> DataResult<OrderBookData> {
        let prefix = Self::market_prefix(stock_code);
        let url = format!("https://hq.sinajs.cn/list={prefix}{stock_code}");
        debug!("Fetching order book from Sina: {}", stock_code);

        let response = self.request.get(&url).await?;
        let text = Self::decode_hq_text(response).await?;
        let line = text.lines().next().ok_or_else(|| DataError::NoDataAvailable)?;
        let (code, parts) = Self::parse_sina_line(line, 30)
            .ok_or_else(|| DataError::custom("Invalid order book response"))?;
        parse_order_book_from_parts(&code, parts[0], &parts)
            .ok_or_else(|| DataError::custom("Failed to parse order book"))
    }

    async fn get_ticks(&self, stock_code: &str) -> DataResult<Vec<TickData>> {
        let symbol = Self::full_symbol(stock_code);
        let url = "https://vip.stock.finance.sina.com.cn/quotes_service/view/CN_TransListV2.php";
        let params = [("symbol", symbol.as_str()), ("num", "100")];

        debug!("Fetching ticks from Sina: {}", stock_code);

        let response = self.vip_request.get_with_params(url, &params).await?;
        let text = response.text().await.map_err(DataError::Network)?;
        Ok(parse_ticks_from_trans_list(&text, stock_code))
    }
}

#[async_trait]
impl StockInfoSource for SinaSource {
    /// Fetches all available stock codes.
    async fn get_all_codes(&self, limit: Option<usize>) -> DataResult<Vec<StockCode>> {
        let url = "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeData";
        let mut all_codes = Vec::new();
        let page_size = 80;
        let mut page = 1;

        loop {
            let params = [
                ("page", page.to_string()),
                ("num", page_size.to_string()),
                ("sort", "changepercent".to_string()),
                ("asc", "0".to_string()),
                ("node", "hs_a".to_string()),
                ("symbol", "".to_string()),
                ("_s_r_a", "page".to_string()),
            ];

            debug!("Fetching stock codes page {} from Sina", page);

            let response = match self.vip_request.get_with_params(url, &params).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Failed to fetch page {}: {}", page, e);
                    break;
                }
            };

            let text = match response.text().await {
                Ok(t) => t,
                Err(e) => {
                    warn!("Failed to read response: {}", e);
                    break;
                }
            };

            if text.is_empty() || text == "null" {
                break;
            }

            let items: Vec<SinaStockItem> = match serde_json::from_str(&text) {
                Ok(items) => items,
                Err(e) => {
                    warn!("Failed to parse response: {}", e);
                    break;
                }
            };

            if items.is_empty() {
                break;
            }

            let count = items.len();

            for item in items {
                let exchange = Exchange::from_stock_code(&item.code);
                all_codes.push(StockCode {
                    stock_code: item.code,
                    short_name: item.name,
                    exchange,
                    list_date: None,
                });

                if let Some(lim) = limit {
                    if all_codes.len() >= lim {
                        return Ok(all_codes);
                    }
                }
            }

            if count < page_size {
                break;
            }
            page += 1;
        }

        Ok(all_codes)
    }

    /// Fetches detailed stock information.
    async fn get_stock_info(&self, stock_code: &str) -> DataResult<StockInfo> {
        let prefix = Self::market_prefix(stock_code);
        let url = format!("https://hq.sinajs.cn/list={prefix}{stock_code}");
        debug!("Fetching stock info from Sina: {}", stock_code);

        let response = self.request.get(&url).await?;
        let text = Self::decode_hq_text(response).await?;

        if text.is_empty() || !text.contains('=') {
            return Err(DataError::custom("No stock info available"));
        }

        let quote_start = match text.find('"') {
            Some(pos) => pos + 1,
            None => return Err(DataError::custom("Invalid response format")),
        };
        let quote_end = match text.rfind('"') {
            Some(pos) => pos,
            None => return Err(DataError::custom("Invalid response format")),
        };

        if quote_start >= quote_end {
            return Err(DataError::custom("No stock info available"));
        }

        let data = &text[quote_start..quote_end];
        let parts: Vec<&str> = data.split(',').collect();

        if parts.is_empty() {
            return Err(DataError::custom("No stock info available"));
        }

        let short_name = parts.first().map(|s| s.to_string()).unwrap_or_default();
        let exchange = Exchange::from_stock_code(stock_code);

        Ok(StockInfo {
            stock_code: stock_code.to_string(),
            full_name: String::new(),
            short_name,
            exchange,
            industry: None,
            total_shares: None,
            circulating_shares: None,
            list_date: None,
        })
    }
}

#[async_trait]
impl FundInfoSource for SinaSource {
    /// Fetches all available ETF codes.
    async fn get_all_etf_codes(&self, limit: Option<usize>) -> DataResult<Vec<ETFCode>> {
        let url = "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeData";
        let mut all_codes = Vec::new();
        let page_size = 80;
        let mut page = 1;

        loop {
            let params = [
                ("page", page.to_string()),
                ("num", page_size.to_string()),
                ("sort", "changepercent".to_string()),
                ("asc", "0".to_string()),
                ("node", "etf_hq_fund".to_string()),
                ("symbol", "".to_string()),
                ("_s_r_a", "page".to_string()),
            ];

            debug!("Fetching ETF codes page {} from Sina", page);

            let response = match self.vip_request.get_with_params(url, &params).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Failed to fetch ETF page {}: {}", page, e);
                    break;
                }
            };

            let text = match response.text().await {
                Ok(t) => t,
                Err(e) => {
                    warn!("Failed to read ETF response: {}", e);
                    break;
                }
            };

            if text.is_empty() || text == "null" {
                break;
            }

            let items: Vec<SinaETFItem> = match serde_json::from_str(&text) {
                Ok(items) => items,
                Err(e) => {
                    warn!("Failed to parse ETF response: {}", e);
                    break;
                }
            };

            if items.is_empty() {
                break;
            }

            let count = items.len();

            for item in items {
                let exchange = Exchange::from_stock_code(&item.code);
                all_codes.push(ETFCode {
                    fund_code: item.code,
                    short_name: item.name,
                    exchange,
                    net_value: item.trade.parse().ok(),
                });

                if let Some(lim) = limit {
                    if all_codes.len() >= lim {
                        return Ok(all_codes);
                    }
                }
            }

            if count < page_size {
                break;
            }
            page += 1;
        }

        Ok(all_codes)
    }
}

/// ETF item from Sina API.
#[derive(Debug, Deserialize)]
struct SinaETFItem {
    /// ETF code
    code: String,
    /// ETF name
    name: String,
    /// Current price
    #[serde(default)]
    trade: String,
}

#[async_trait]
impl FundMarketSource for SinaSource {
    /// Fetches historical ETF K-line market data.
    async fn get_etf_market(
        &self,
        fund_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<ETFMarketData>> {
        let scale = kline_scale(k_type)
            .ok_or_else(|| DataError::not_supported(format!("sina: etf kline type {k_type:?}")))?;
        let symbol = Self::full_symbol(fund_code);
        let url = "https://money.finance.sina.com.cn/quotes_service/api/json_v2.php/CN_MarketData.getKLineData";
        let params = [
            ("symbol", symbol.as_str()),
            ("scale", &scale.to_string()),
            ("ma", "no"),
            ("datalen", "1023"),
        ];

        let records: Vec<SinaKLineRecord> = self.vip_request.get_json_with_params(url, &params).await?;
        let intraday = scale < 240;
        let market = parse_kline_records(fund_code, &records, start_date, end_date, intraday);
        Ok(market
            .into_iter()
            .map(|row| ETFMarketData {
                fund_code: row.stock_code,
                trade_time: row.trade_time,
                trade_date: row.trade_date,
                open: row.open,
                close: row.close,
                high: row.high,
                low: row.low,
                volume: row.volume,
                amount: row.amount,
                change: Some(row.change),
                change_pct: Some(row.change_pct),
            })
            .collect())
    }

    /// Fetches real-time ETF quotes.
    async fn get_etf_current(&self, fund_codes: &[&str]) -> DataResult<Vec<ETFCurrentData>> {
        if fund_codes.is_empty() {
            return Ok(Vec::new());
        }

        let codes_str: String = fund_codes
            .iter()
            .map(|c| format!("s_{}{}", Self::market_prefix(c), c))
            .collect::<Vec<_>>()
            .join(",");

        let url = format!("https://hq.sinajs.cn/list={codes_str}");
        debug!("Fetching ETF current from Sina: {}", url);

        let response = self.request.get(&url).await?;
        let text = Self::decode_hq_text(response).await?;

        if text.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::new();

        for line in text.split(';') {
            let line = line.trim();
            if line.len() < 10 {
                continue;
            }

            if let Some(data) = Self::parse_etf_quote_line(line) {
                result.push(data);
            }
        }

        Ok(result)
    }

    /// Fetches intraday ETF minute-level data.
    async fn get_etf_min(&self, fund_code: &str) -> DataResult<Vec<ETFMinuteData>> {
        let minutes = self.get_market_min(fund_code).await?;
        Ok(minutes
            .into_iter()
            .map(|row| ETFMinuteData {
                fund_code: row.stock_code,
                trade_time: row.trade_time,
                price: row.price,
                change: row.change,
                change_pct: row.change_pct,
                volume: row.volume,
                avg_price: row.avg_price,
                amount: row.amount,
            })
            .collect())
    }
}

#[async_trait]
impl BondInfoSource for SinaSource {
    /// Fetches all available convertible bond codes.
    async fn get_all_bond_codes(
        &self,
        limit: Option<usize>,
    ) -> DataResult<Vec<ConvertibleBondCode>> {
        let url = "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeDataSimple";
        let mut all_bonds = Vec::new();
        let page_size = 80;
        let mut page = 1;

        loop {
            let params = [
                ("page", page.to_string()),
                ("num", page_size.to_string()),
                ("sort", "symbol".to_string()),
                ("asc", "1".to_string()),
                ("node", "hskzz_z".to_string()),
                ("_s_r_a", "page".to_string()),
            ];

            let response = match self.vip_request.get_with_params(url, &params).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Failed to fetch bond codes page {}: {}", page, e);
                    break;
                }
            };

            let text = match response.text().await {
                Ok(t) => t,
                Err(e) => {
                    warn!("Failed to read bond codes response: {}", e);
                    break;
                }
            };

            if text.starts_with('<') || text.is_empty() || text == "null" || text == "[]" {
                break;
            }

            let items: Vec<SinaBondCodeItem> = match serde_json::from_str(&text) {
                Ok(items) => items,
                Err(e) => {
                    warn!("Failed to parse bond codes: {}", e);
                    break;
                }
            };

            if items.is_empty() {
                break;
            }

            let count = items.len();
            for item in items {
                all_bonds.push(ConvertibleBondCode {
                    bond_code: item.code,
                    bond_name: item.name,
                    stock_code: String::new(),
                    short_name: String::new(),
                    sub_date: None,
                    issue_amount: None,
                    listing_date: None,
                    expire_date: None,
                    convert_price: None,
                });
                if let Some(lim) = limit {
                    if all_bonds.len() >= lim {
                        return Ok(all_bonds);
                    }
                }
            }

            if count < page_size {
                break;
            }
            page += 1;
        }

        Ok(all_bonds)
    }
}

#[async_trait]
impl BondMarketSource for SinaSource {
    /// Fetches real-time bond quotes.
    async fn get_bond_current(
        &self,
        bond_codes: Option<&[&str]>,
    ) -> DataResult<Vec<BondCurrentData>> {
        if let Some(codes) = bond_codes {
            if !codes.is_empty() {
                return self.get_bond_current_by_codes(codes).await;
            }
        }

        self.get_all_bond_current().await
    }
}

impl SinaSource {
    /// Parses bond quote line from hq.sinajs.cn response.
    fn parse_bond_quote_line(line: &str) -> Option<BondCurrentData> {
        let (bond_code, parts) = Self::parse_sina_line(line, 10)?;
        let bond_name = parts[0].to_string();
        let open: f64 = parts[1].parse().unwrap_or(0.0);
        let pre_close: f64 = parts[2].parse().unwrap_or(0.0);
        let price: f64 = parts[3].parse().unwrap_or(0.0);
        let high: f64 = parts[4].parse().unwrap_or(0.0);
        let low: f64 = parts[5].parse().unwrap_or(0.0);
        let volume: u64 = parts[8].parse().unwrap_or(0);
        let amount: f64 = parts[9].parse().unwrap_or(0.0);
        let change = price - pre_close;
        let change_pct = if pre_close > 0.0 {
            (change / pre_close) * 100.0
        } else {
            0.0
        };
        Some(BondCurrentData {
            bond_code,
            bond_name,
            price,
            open,
            high,
            low,
            pre_close,
            change,
            change_pct,
            volume,
            amount,
        })
    }

    /// Fetches bond data for specific codes directly.
    async fn get_bond_current_by_codes(&self, codes: &[&str]) -> DataResult<Vec<BondCurrentData>> {
        let symbols: Vec<String> = codes
            .iter()
            .map(|c| format!("{}{}", Self::market_prefix(c), c))
            .collect();
        let symbols_str = symbols.join(",");

        let url = format!("https://hq.sinajs.cn/list={}", symbols_str);
        debug!("Fetching bond quotes for {} codes from Sina", codes.len());

        let response = self.request.get(&url).await?;
        let text = Self::decode_hq_text(response).await?;

        let mut results = Vec::with_capacity(codes.len());

        for line in text.lines() {
            if let Some(data) = Self::parse_bond_quote_line(line) {
                results.push(data);
            }
        }

        Ok(results)
    }

    /// Fetches all bond data with pagination.
    async fn get_all_bond_current(&self) -> DataResult<Vec<BondCurrentData>> {
        let url = "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeDataSimple";
        let mut all_bonds = Vec::new();
        let page_size = 80;
        let mut page = 1;

        loop {
            let params = [
                ("page", page.to_string()),
                ("num", page_size.to_string()),
                ("sort", "symbol".to_string()),
                ("asc", "1".to_string()),
                ("node", "hskzz_z".to_string()),
                ("_s_r_a", "page".to_string()),
            ];

            debug!("Fetching bond data page {} from Sina", page);

            let response = match self.vip_request.get_with_params(url, &params).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Failed to fetch bond page {}: {}", page, e);
                    break;
                }
            };

            let text = match response.text().await {
                Ok(t) => t,
                Err(e) => {
                    warn!("Failed to read bond response: {}", e);
                    break;
                }
            };

            if text.starts_with('<') || text.is_empty() || text == "null" || text == "[]" {
                debug!("Bond response is HTML or empty, trying next page or stopping");
                if page == 1 {
                    break;
                }
                page += 1;
                continue;
            }

            let items: Vec<SinaBondItem> = match serde_json::from_str(&text) {
                Ok(items) => items,
                Err(e) => {
                    warn!(
                        "Failed to parse bond response: {} (text starts with: {})",
                        e,
                        text.chars().take(100).collect::<String>()
                    );
                    break;
                }
            };

            if items.is_empty() {
                break;
            }

            let count = items.len();

            for item in items {
                let price: f64 = item.trade.parse().unwrap_or(0.0);
                let change: f64 = item.pricechange.parse().unwrap_or(0.0);
                let change_pct: f64 = item.changepercent.parse().unwrap_or(0.0);
                let pre_close: f64 = item.settlement.parse().unwrap_or(0.0);
                let open: f64 = item.open.parse().unwrap_or(0.0);
                let high: f64 = item.high.parse().unwrap_or(0.0);
                let low: f64 = item.low.parse().unwrap_or(0.0);
                let volume: u64 = item.volume.parse::<f64>().unwrap_or(0.0) as u64;
                let amount: f64 = item.amount.parse().unwrap_or(0.0);

                all_bonds.push(BondCurrentData {
                    bond_code: item.code,
                    bond_name: item.name,
                    price,
                    open,
                    high,
                    low,
                    pre_close,
                    change,
                    change_pct,
                    volume,
                    amount,
                });
            }

            if count < page_size {
                break;
            }
            page += 1;
        }

        let mut seen = std::collections::HashSet::new();
        all_bonds.retain(|b| seen.insert(b.bond_code.clone()));

        Ok(all_bonds)
    }
}

#[async_trait]
impl IndexMarketSource for SinaSource {
    async fn get_index_list(&self, limit: Option<usize>) -> DataResult<Vec<IndexCode>> {
        let _ = limit;
        Err(DataError::not_supported("sina: get_index_list"))
    }

    async fn get_index_current(&self, index_codes: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        if index_codes.is_empty() {
            return Ok(Vec::new());
        }

        let codes_str: String = index_codes
            .iter()
            .map(|code| {
                let exchange = if code.starts_with("399") {
                    Exchange::SZ
                } else {
                    Exchange::SH
                };
                sina_index_hq_symbol(code, exchange)
            })
            .collect::<Vec<_>>()
            .join(",");

        let url = format!("https://hq.sinajs.cn/list={codes_str}");
        debug!("Fetching index quotes from Sina: {}", url);

        let response = self.request.get(&url).await?;
        let text = Self::decode_hq_text(response).await?;

        let mut result = Vec::new();
        for line in text.split(';') {
            let line = line.trim();
            if line.len() < 10 {
                continue;
            }
            if let Some(data) = Self::parse_index_quote_line(line) {
                result.push(data);
            }
        }
        Ok(result)
    }
}

const SINA_FUTURES_CONTINUOUS: &[(&str, &str, DerivativesExchange)] = &[
    ("AU0", "沪金连续", DerivativesExchange::Shfe),
    ("AG0", "沪银连续", DerivativesExchange::Shfe),
    ("CU0", "沪铜连续", DerivativesExchange::Shfe),
    ("AL0", "沪铝连续", DerivativesExchange::Shfe),
    ("ZN0", "沪锌连续", DerivativesExchange::Shfe),
    ("RB0", "螺纹钢连续", DerivativesExchange::Shfe),
    ("RU0", "橡胶连续", DerivativesExchange::Shfe),
    ("I0", "铁矿石连续", DerivativesExchange::Dce),
    ("J0", "焦炭连续", DerivativesExchange::Dce),
    ("JM0", "焦煤连续", DerivativesExchange::Dce),
    ("M0", "豆粕连续", DerivativesExchange::Dce),
    ("Y0", "豆油连续", DerivativesExchange::Dce),
    ("P0", "棕榈油连续", DerivativesExchange::Dce),
    ("C0", "玉米连续", DerivativesExchange::Dce),
    ("SR0", "白糖连续", DerivativesExchange::Czce),
    ("CF0", "棉花连续", DerivativesExchange::Czce),
    ("TA0", "PTA连续", DerivativesExchange::Czce),
    ("MA0", "甲醇连续", DerivativesExchange::Czce),
    ("IF0", "沪深300连续", DerivativesExchange::Cffex),
    ("IC0", "中证500连续", DerivativesExchange::Cffex),
    ("IH0", "上证50连续", DerivativesExchange::Cffex),
];

impl SinaSource {
    fn parse_futures_quote_line(line: &str) -> Option<DerivativesQuote> {
        let start = line.find('"')? + 1;
        let end = line.rfind('"')?;
        if end <= start {
            return None;
        }
        let parts: Vec<&str> = line[start..end].split(',').collect();
        if parts.len() < 15 {
            return None;
        }
        let contract_name = parts[0].to_string();
        let open = parts[2].parse().ok();
        let high = parts[3].parse().ok();
        let low = parts[4].parse().ok();
        let price: f64 = parts[8].parse().ok()?;
        if price <= 0.0 {
            return None;
        }
        let pre_close: f64 = parts[10].parse().unwrap_or(0.0);
        let open_interest = parts[13].parse::<f64>().ok().map(|v| v as u64);
        let volume = parts[14].parse().unwrap_or(0);
        let change = if pre_close > 0.0 {
            price - pre_close
        } else {
            0.0
        };
        let change_pct = if pre_close > 0.0 {
            change / pre_close * 100.0
        } else {
            0.0
        };
        let prefix = line
            .split('=')
            .next()?
            .trim()
            .trim_start_matches("var hq_str_nf_");
        let contract_code = prefix.to_string();
        Some(DerivativesQuote {
            contract_code: contract_code.clone(),
            contract_name,
            secid: format!("nf.{contract_code}"),
            price,
            change,
            change_pct,
            volume,
            amount: 0.0,
            open,
            high,
            low,
            pre_close: if pre_close > 0.0 { Some(pre_close) } else { None },
            open_interest,
            trade_date: None,
        })
    }

    fn normalize_futures_symbol(symbol: &str) -> String {
        let upper = symbol.trim().to_uppercase();
        if upper.starts_with("NF_") {
            upper.trim_start_matches("NF_").to_string()
        } else if upper.contains('.') {
            upper.split('.').nth(1).unwrap_or(&upper).to_string()
        } else {
            upper
        }
    }
}

#[async_trait]
impl FuturesSource for SinaSource {
    async fn get_futures_list(&self, limit: Option<usize>) -> DataResult<Vec<FuturesContract>> {
        let mut items: Vec<FuturesContract> = SINA_FUTURES_CONTINUOUS
            .iter()
            .map(|(code, name, exchange)| FuturesContract {
                contract_code: (*code).to_string(),
                contract_name: (*name).to_string(),
                secid: format!("nf.{code}"),
                exchange: *exchange,
            })
            .collect();
        if let Some(lim) = limit {
            items.truncate(lim);
        }
        Ok(items)
    }

    async fn get_futures_current(&self, secids: &[&str]) -> DataResult<Vec<DerivativesQuote>> {
        if secids.is_empty() {
            return Ok(Vec::new());
        }
        let symbols: Vec<String> = secids
            .iter()
            .map(|s| Self::normalize_futures_symbol(s))
            .collect();
        let codes_str = symbols
            .iter()
            .map(|symbol| format!("nf_{symbol}"))
            .collect::<Vec<_>>()
            .join(",");
        let url = format!("https://hq.sinajs.cn/list={codes_str}");
        debug!("Fetching futures quotes from Sina: {}", url);
        let response = self.request.get(&url).await?;
        let text = Self::decode_hq_text(response).await?;
        Ok(text
            .split(';')
            .filter_map(Self::parse_futures_quote_line)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_futures_quote_line() {
        let line = r#"var hq_str_nf_ZN0="沪锌连续,010000,24370.000,24430.000,24235.000,0.000,24400.000,24415.000,24400.000,0.000,24610.000,12,21,96296.000,62729,沪,沪锌,2026-07-18,1";"#;
        let quote = SinaSource::parse_futures_quote_line(line).unwrap();
        assert_eq!(quote.contract_code, "ZN0");
        assert_eq!(quote.price, 24400.0);
        assert!(quote.volume > 0);
    }

    #[test]
    fn test_market_prefix() {
        assert_eq!(SinaSource::market_prefix("600000"), "sh");
        assert_eq!(SinaSource::market_prefix("000001"), "sz");
        assert_eq!(SinaSource::market_prefix("300001"), "sz");
    }

    #[test]
    fn test_parse_quote_line() {
        let line = r#"var hq_str_s_sh600000="浦发银行,10.500,0.100,0.96,1234567,12345678.00";"#;
        let result = SinaSource::parse_quote_line(line);
        assert!(result.is_some());

        let data = result.unwrap();
        assert_eq!(data.stock_code, "600000");
        assert_eq!(data.short_name, "浦发银行");
        assert_eq!(data.price, 10.5);
    }

    #[test]
    fn test_parse_invalid_line() {
        let line = "invalid";
        assert!(SinaSource::parse_quote_line(line).is_none());
    }

    #[test]
    fn test_market_prefix_beijing() {
        assert_eq!(SinaSource::market_prefix("830001"), "bj");
    }

    #[test]
    fn test_parse_quote_line_volume_adjustment() {
        let line = r#"var hq_str_sz000001="平安银行,10.500,0.100,0.96,1234567,12345678.00";"#;
        let data = SinaSource::parse_quote_line(line).unwrap();
        assert_eq!(data.stock_code, "000001");
        assert_eq!(data.volume, 123456700);
        assert_eq!(data.amount, 123456780000.0);
    }

    #[test]
    fn test_parse_etf_quote_line() {
        let line = r#"var hq_str_sh510300="沪深300ETF,4.500,0.010,0.22,987654,9876543.00";"#;
        let data = SinaSource::parse_etf_quote_line(line).unwrap();
        assert_eq!(data.fund_code, "510300");
        assert_eq!(data.short_name, "沪深300ETF");
        assert_eq!(data.price, 4.5);
        assert_eq!(data.change_pct, Some(0.22));
    }

    #[test]
    fn test_parse_index_quote_line() {
        let line = r#"var hq_str_s_sh000001="上证指数,3764.1547,-118.2579,-3.05,6504509,124644545";"#;
        let data = SinaSource::parse_index_quote_line(line).unwrap();
        assert_eq!(data.stock_code, "000001");
        assert_eq!(data.short_name, "上证指数");
        assert_eq!(data.price, 3764.1547);
        assert_eq!(data.change_pct, -3.05);
        assert_eq!(data.volume, 6504509);
    }
}
