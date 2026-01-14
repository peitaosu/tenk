//! Sina Finance data source.

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, HOST, REFERER, USER_AGENT};
use serde::Deserialize;
use tracing::{debug, warn};

use crate::data::{
    BondCurrentData, ConvertibleBondCode, CurrentMarketData, ETFCode, ETFCurrentData,
    ETFMarketData, ETFMinuteData, Exchange, KLineType, MarketData, MinuteData, StockCode, StockInfo,
};
use crate::error::{DataError, DataResult};
use crate::request::{RequestConfig, RequestManager};
use crate::traits::{
    BondInfoSource, BondMarketSource, DataSource, FundInfoSource, FundMarketSource,
    StockInfoSource, StockMarketSource,
};

#[derive(Debug, Clone)]
pub struct SinaSource {
    request: RequestManager,
    vip_request: RequestManager,
}

impl SinaSource {
    pub fn new() -> DataResult<Self> {
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
            HeaderValue::from_static("http://vip.stock.finance.sina.com.cn/"),
        );

        let hq_config = RequestConfig::default().with_headers(headers);
        let vip_config = RequestConfig::default();

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

    fn get_prefix(stock_code: &str) -> &'static str {
        match Exchange::from_stock_code(stock_code) {
            Exchange::SH => "sh",
            Exchange::SZ => "sz",
            Exchange::BJ => "bj",
            Exchange::Unknown => "sh",
        }
    }

    fn parse_quote_line(line: &str) -> Option<CurrentMarketData> {
        let eq_pos = line.find('=')?;

        if eq_pos < 6 {
            return None;
        }
        let code_start = eq_pos - 6;
        let stock_code = &line[code_start..eq_pos];

        let quote_start = line.find('"')? + 1;
        let quote_end = line.rfind('"')?;
        if quote_start >= quote_end {
            return None;
        }

        let data = &line[quote_start..quote_end];
        let parts: Vec<&str> = data.split(',').collect();

        if parts.len() < 6 {
            return None;
        }

        let short_name = parts[0].to_string();
        let price: f64 = parts[1].parse().ok()?;
        let change: f64 = parts[2].parse().ok()?;
        let change_pct: f64 = parts[3].parse().ok()?;
        let volume: u64 = parts[4].parse().ok()?;
        let amount: f64 = parts[5].parse().ok()?;

        let (adj_volume, adj_amount) =
            if stock_code.starts_with(['0', '3', '6', '9']) {
                (volume * 100, amount * 10000.0)
            } else {
                (volume, amount)
            };

        Some(CurrentMarketData {
            stock_code: stock_code.to_string(),
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

    fn parse_etf_quote_line(line: &str) -> Option<ETFCurrentData> {
        let eq_pos = line.find('=')?;

        if eq_pos < 6 {
            return None;
        }
        let code_start = eq_pos - 6;
        let fund_code = &line[code_start..eq_pos];

        let quote_start = line.find('"')? + 1;
        let quote_end = line.rfind('"')?;
        if quote_start >= quote_end {
            return None;
        }

        let data = &line[quote_start..quote_end];
        let parts: Vec<&str> = data.split(',').collect();

        if parts.len() < 6 {
            return None;
        }

        let short_name = parts[0].to_string();
        let price: f64 = parts[1].parse().ok()?;
        let change: f64 = parts[2].parse().ok()?;
        let change_pct: f64 = parts[3].parse().ok()?;
        let volume: u64 = parts[4].parse().ok()?;
        let amount: f64 = parts[5].parse().ok()?;

        let (adj_volume, adj_amount) =
            if fund_code.starts_with(['0', '1', '5']) {
                (volume * 100, amount * 10000.0)
            } else {
                (volume, amount)
            };

        Some(ETFCurrentData {
            fund_code: fund_code.to_string(),
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
    fn default() -> Self {
        Self::new().expect("Failed to create SinaSource")
    }
}

#[derive(Debug, Deserialize)]
struct SinaStockItem {
    code: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct SinaBondItem {
    code: String,
    name: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    trade: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pricechange: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    changepercent: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    settlement: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    open: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    high: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    low: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    volume: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    amount: String,
}

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
    fn name(&self) -> &'static str {
        "sina"
    }

    fn priority(&self) -> u8 {
        2
    }

    async fn is_available(&self) -> bool {
        self.request
            .get("https://hq.sinajs.cn/list=s_sh000001")
            .await
            .is_ok()
    }
}

#[async_trait]
impl StockMarketSource for SinaSource {
    async fn get_market(
        &self,
        _stock_code: &str,
        _start_date: Option<&str>,
        _end_date: Option<&str>,
        _k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        Err(DataError::not_supported("sina: get_market (K-line)"))
    }

    async fn get_market_current(&self, stock_codes: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        if stock_codes.is_empty() {
            return Ok(Vec::new());
        }

        let codes_str: String = stock_codes
            .iter()
            .map(|c| format!("s_{}{}", Self::get_prefix(c), c))
            .collect::<Vec<_>>()
            .join(",");

        let url = format!("https://hq.sinajs.cn/list={codes_str}");
        debug!("Fetching current market from Sina: {}", url);

        let response = self.request.get(&url).await?;
        let text = response.text().await.map_err(DataError::Network)?;

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

    async fn get_market_min(&self, _stock_code: &str) -> DataResult<Vec<MinuteData>> {
        Err(DataError::not_supported("sina: get_market_min"))
    }
}

#[async_trait]
impl StockInfoSource for SinaSource {
    async fn get_all_codes(&self) -> DataResult<Vec<StockCode>> {
        let url = "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeData";
        let mut all_codes = Vec::new();
        let page_size = 80;

        for page in 1..=200 {
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
            }

            if count < page_size {
                break;
            }
        }

        Ok(all_codes)
    }

    async fn get_stock_info(&self, stock_code: &str) -> DataResult<StockInfo> {
        let prefix = Self::get_prefix(stock_code);
        let url = format!("https://hq.sinajs.cn/list={prefix}{stock_code}");
        debug!("Fetching stock info from Sina: {}", stock_code);

        let response = self.request.get(&url).await?;
        let text = response.text().await.map_err(DataError::Network)?;

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
    async fn get_all_etf_codes(&self) -> DataResult<Vec<ETFCode>> {
        let url = "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeData";
        let mut all_codes = Vec::new();
        let page_size = 80;

        for page in 1..=100 {
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
            }

            if count < page_size {
                break;
            }
        }

        Ok(all_codes)
    }
}

#[derive(Debug, Deserialize)]
struct SinaETFItem {
    code: String,
    name: String,
    #[serde(default)]
    trade: String,
}

#[async_trait]
impl FundMarketSource for SinaSource {
    async fn get_etf_market(
        &self,
        _fund_code: &str,
        _start_date: Option<&str>,
        _end_date: Option<&str>,
        _k_type: KLineType,
    ) -> DataResult<Vec<ETFMarketData>> {
        Err(DataError::not_supported("sina: get_etf_market (K-line)"))
    }

    async fn get_etf_current(&self, fund_codes: &[&str]) -> DataResult<Vec<ETFCurrentData>> {
        if fund_codes.is_empty() {
            return Ok(Vec::new());
        }

        let codes_str: String = fund_codes
            .iter()
            .map(|c| format!("s_{}{}", Self::get_prefix(c), c))
            .collect::<Vec<_>>()
            .join(",");

        let url = format!("https://hq.sinajs.cn/list={codes_str}");
        debug!("Fetching ETF current from Sina: {}", url);

        let response = self.request.get(&url).await?;
        let text = response.text().await.map_err(DataError::Network)?;

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

    async fn get_etf_min(&self, _fund_code: &str) -> DataResult<Vec<ETFMinuteData>> {
        Err(DataError::not_supported("sina: get_etf_min"))
    }
}

#[async_trait]
impl BondInfoSource for SinaSource {
    async fn get_all_bond_codes(&self) -> DataResult<Vec<ConvertibleBondCode>> {
        Err(DataError::not_supported("sina: get_all_bond_codes"))
    }
}

#[async_trait]
impl BondMarketSource for SinaSource {
    async fn get_bond_current(
        &self,
        bond_codes: Option<&[&str]>,
    ) -> DataResult<Vec<BondCurrentData>> {
        let url = "http://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeDataSimple";
        let mut all_bonds = Vec::new();
        let page_size = 80;

        for page in 1..=100 {
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
                if let Some(codes) = bond_codes {
                    if !codes.contains(&item.code.as_str()) {
                        continue;
                    }
                }

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

            if let Some(codes) = bond_codes {
                if all_bonds.len() >= codes.len() {
                    break;
                }
            }
        }

        let mut seen = std::collections::HashSet::new();
        all_bonds.retain(|b| seen.insert(b.bond_code.clone()));

        Ok(all_bonds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_prefix() {
        assert_eq!(SinaSource::get_prefix("600000"), "sh");
        assert_eq!(SinaSource::get_prefix("000001"), "sz");
        assert_eq!(SinaSource::get_prefix("300001"), "sz");
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
}
