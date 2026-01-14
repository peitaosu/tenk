//! East Money data source.

use async_trait::async_trait;
use chrono::{NaiveDate, TimeZone, Utc};
use serde::Deserialize;
use tracing::{debug, warn};

use crate::data::{
    BondCurrentData, ConvertibleBondCode, CurrentMarketData, ETFCode, ETFCurrentData,
    ETFMarketData, ETFMinuteData, Exchange, KLineType, MarketData, MinuteData, StockCode, StockInfo,
};
use crate::error::DataResult;
use crate::request::RequestManager;
use crate::traits::{
    BondInfoSource, BondMarketSource, DataSource, FundInfoSource, FundMarketSource,
    StockInfoSource, StockMarketSource,
};

#[derive(Debug, Clone)]
pub struct EastMoneySource {
    request: RequestManager,
}

impl EastMoneySource {
    pub fn new() -> DataResult<Self> {
        Ok(Self {
            request: RequestManager::default_manager()?,
        })
    }

    pub fn with_request_manager(request: RequestManager) -> Self {
        Self { request }
    }

    fn get_secid(stock_code: &str) -> String {
        let prefix = if stock_code.starts_with('6') || stock_code.starts_with('5') {
            "1"
        } else {
            "0"
        };
        format!("{prefix}.{stock_code}")
    }

    fn kline_to_klt(k_type: KLineType) -> u32 {
        match k_type {
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
}

impl Default for EastMoneySource {
    fn default() -> Self {
        Self::new().expect("Failed to create EastMoneySource")
    }
}

#[derive(Debug, Deserialize)]
struct KLineResponse {
    data: Option<KLineData>,
}

#[derive(Debug, Deserialize)]
struct KLineData {
    klines: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct MinuteResponse {
    data: Option<MinuteResponseData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MinuteResponseData {
    pre_close: f64,
    trends: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct StockListResponse {
    data: Option<StockListData>,
}

#[derive(Debug, Deserialize)]
struct StockListData {
    diff: Option<Vec<StockItem>>,
}

#[derive(Debug, Deserialize)]
struct StockItem {
    #[serde(rename = "f12")]
    code: String,
    #[serde(rename = "f14")]
    name: String,
    #[serde(rename = "f2", default)]
    price: Option<f64>,
    #[serde(rename = "f3", default)]
    change_pct: Option<f64>,
    #[serde(rename = "f4", default)]
    change: Option<f64>,
    #[serde(rename = "f5", default)]
    volume: Option<u64>,
    #[serde(rename = "f6", default)]
    amount: Option<f64>,
}

#[async_trait]
impl DataSource for EastMoneySource {
    fn name(&self) -> &'static str {
        "eastmoney"
    }

    fn priority(&self) -> u8 {
        1
    }

    async fn is_available(&self) -> bool {
        self.request
            .get("https://push2.eastmoney.com/api/qt/stock/trends2/get")
            .await
            .is_ok()
    }
}

#[async_trait]
impl StockMarketSource for EastMoneySource {
    async fn get_market(
        &self,
        stock_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        let secid = Self::get_secid(stock_code);
        let klt = Self::kline_to_klt(k_type);

        let start = start_date
            .map(|s| s.replace('-', ""))
            .unwrap_or_else(|| "19900101".to_string());
        let end = end_date
            .map(|s| s.replace('-', ""))
            .unwrap_or_else(|| chrono::Utc::now().format("%Y%m%d").to_string());

        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f116"),
            ("ut", "7eea3edcaed734bea9cbfc24409ed989"),
            ("klt", &klt.to_string()),
            ("fqt", "1"),
            ("secid", &secid),
            ("beg", &start),
            ("end", &end),
        ];

        let url = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
        debug!("Fetching market data from East Money: {}", stock_code);

        let response: KLineResponse = self.request.get_json_with_params(url, &params).await?;
        let klines = response.data.and_then(|d| d.klines).unwrap_or_default();

        if klines.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::with_capacity(klines.len());

        for line in klines {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 11 {
                continue;
            }

            let trade_date = NaiveDate::parse_from_str(parts[0], "%Y-%m-%d")
                .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

            let open: f64 = parts[1].parse().unwrap_or(0.0);
            let close: f64 = parts[2].parse().unwrap_or(0.0);
            let high: f64 = parts[3].parse().unwrap_or(0.0);
            let low: f64 = parts[4].parse().unwrap_or(0.0);
            let volume: u64 = parts[5].parse::<f64>().unwrap_or(0.0) as u64 * 100;
            let amount: f64 = parts[6].parse().unwrap_or(0.0);
            let change_pct: f64 = parts[8].parse().unwrap_or(0.0);
            let change: f64 = parts[9].parse().unwrap_or(0.0);
            let turnover_ratio: f64 = parts[10].parse().unwrap_or(0.0);
            let pre_close = close - change;

            result.push(MarketData {
                stock_code: stock_code.to_string(),
                trade_time: Utc.from_utc_datetime(&trade_date.and_hms_opt(15, 0, 0).unwrap()),
                trade_date,
                open,
                close,
                high,
                low,
                volume,
                amount,
                change,
                change_pct,
                turnover_ratio,
                pre_close: (pre_close * 100.0).round() / 100.0,
            });
        }

        Ok(result)
    }

    async fn get_market_current(&self, stock_codes: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        if stock_codes.is_empty() {
            return Ok(Vec::new());
        }

        let codes_str: String = stock_codes
            .iter()
            .map(|c| Self::get_secid(c))
            .collect::<Vec<_>>()
            .join(",");

        let params = [
            ("pn", "1"),
            ("pz", &stock_codes.len().to_string()),
            ("po", "1"),
            ("np", "1"),
            ("ut", "bd1d9ddb04089700cf9c27f6f7426281"),
            ("fltt", "2"),
            ("invt", "2"),
            ("fid", "f3"),
            ("fs", &format!("b:{codes_str}")),
            ("fields", "f2,f3,f4,f5,f6,f12,f14"),
        ];

        let url = "https://push2.eastmoney.com/api/qt/clist/get";
        let response: StockListResponse = self.request.get_json_with_params(url, &params).await?;

        let items = response.data.and_then(|d| d.diff).unwrap_or_default();

        let result: Vec<CurrentMarketData> = items
            .into_iter()
            .map(|item| CurrentMarketData {
                stock_code: item.code,
                short_name: item.name,
                price: item.price.unwrap_or(0.0),
                change: item.change.unwrap_or(0.0),
                change_pct: item.change_pct.unwrap_or(0.0),
                volume: item.volume.unwrap_or(0) * 100,
                amount: item.amount.unwrap_or(0.0),
                open: None,
                high: None,
                low: None,
                pre_close: None,
            })
            .collect();

        Ok(result)
    }

    async fn get_market_min(&self, stock_code: &str) -> DataResult<Vec<MinuteData>> {
        let secid = Self::get_secid(stock_code);

        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58"),
            ("ut", "fa5fd1943c7b386f172d6893dbfba10b"),
            ("ndays", "1"),
            ("iscr", "1"),
            ("iscca", "0"),
            ("secid", &secid),
        ];

        let url = "https://push2.eastmoney.com/api/qt/stock/trends2/get";
        debug!("Fetching minute data from East Money: {}", stock_code);

        let response: MinuteResponse = self.request.get_json_with_params(url, &params).await?;

        let data = match response.data {
            Some(d) => d,
            None => return Ok(Vec::new()),
        };

        let pre_close = data.pre_close;
        let trends = data.trends.unwrap_or_default();

        if trends.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::with_capacity(trends.len());
        let today = Utc::now().date_naive();

        for line in trends {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 8 {
                continue;
            }

            let time_str = parts[0];
            let trade_time = if time_str.contains(' ') {
                chrono::NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M")
                    .map(|dt| Utc.from_utc_datetime(&dt))
                    .unwrap_or_else(|_| Utc::now())
            } else {
                chrono::NaiveTime::parse_from_str(time_str, "%H:%M")
                    .map(|t| Utc.from_utc_datetime(&today.and_time(t)))
                    .unwrap_or_else(|_| Utc::now())
            };

            let price: f64 = parts[2].parse().unwrap_or(0.0);
            let volume: u64 = (parts[5].parse::<f64>().unwrap_or(0.0) * 100.0) as u64;
            let amount: f64 = parts[6].parse().unwrap_or(0.0);
            let avg_price: f64 = parts[7].parse().unwrap_or(price);

            let change = price - pre_close;
            let change_pct = if pre_close > 0.0 {
                (change / pre_close * 100.0 * 100.0).round() / 100.0
            } else {
                0.0
            };

            result.push(MinuteData {
                stock_code: stock_code.to_string(),
                trade_time,
                price,
                change,
                change_pct,
                volume,
                avg_price,
                amount,
            });
        }

        Ok(result)
    }
}

#[async_trait]
impl StockInfoSource for EastMoneySource {
    async fn get_all_codes(&self) -> DataResult<Vec<StockCode>> {
        let url = "https://82.push2.eastmoney.com/api/qt/clist/get";
        let mut all_codes = Vec::new();
        let page_size = 100;

        for page in 1..=100 {
            let params = [
                ("pn", page.to_string()),
                ("pz", page_size.to_string()),
                ("po", "1".to_string()),
                ("np", "1".to_string()),
                ("ut", "bd1d9ddb04089700cf9c27f6f7426281".to_string()),
                ("fltt", "2".to_string()),
                ("invt", "2".to_string()),
                ("fid", "f3".to_string()),
                ("fs", "m:0 t:6,m:0 t:80,m:1 t:2,m:1 t:23,m:0 t:81 s:2048".to_string()),
                ("fields", "f12,f14".to_string()),
            ];

            debug!("Fetching stock codes page {} from East Money", page);

            let response: StockListResponse = match self.request.get_json_with_params(url, &params).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Failed to fetch page {}: {}", page, e);
                    break;
                }
            };

            let items = match response.data.and_then(|d| d.diff) {
                Some(items) if !items.is_empty() => items,
                _ => break,
            };

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
        let exchange = Exchange::from_stock_code(stock_code);
        let secucode = format!("{}.{}", stock_code, exchange);
        let secid = Self::get_secid(stock_code);

        let quote_url = format!(
            "https://push2.eastmoney.com/api/qt/stock/get?secid={}&fields=f58",
            secid
        );

        #[derive(Deserialize)]
        struct QuoteData {
            #[serde(default)]
            f58: Option<String>,
        }

        #[derive(Deserialize)]
        struct QuoteResponse {
            data: Option<QuoteData>,
        }

        let short_name = match self.request.get_json::<QuoteResponse>(&quote_url).await {
            Ok(response) => response
                .data
                .and_then(|d| d.f58)
                .unwrap_or_default(),
            Err(_) => String::new(),
        };

        let url = "https://datacenter.eastmoney.com/securities/api/data/v1/get";
        let params = [
            ("reportName", "RPT_F10_EH_EQUITY"),
            ("columns", "SECUCODE,SECURITY_CODE,END_DATE,TOTAL_SHARES,LIMITED_SHARES,LISTED_A_SHARES"),
            ("quoteColumns", ""),
            ("filter", &format!("(SECUCODE=\"{}\")", secucode)),
            ("pageNumber", "1"),
            ("pageSize", "1"),
            ("sortTypes", "-1"),
            ("sortColumns", "END_DATE"),
            ("source", "HSF10"),
            ("client", "PC"),
        ];

        let response: StockInfoResponse = match self.request.get_json_with_params(url, &params).await {
            Ok(r) => r,
            Err(_) => {
                return Ok(StockInfo {
                    stock_code: stock_code.to_string(),
                    full_name: String::new(),
                    short_name,
                    exchange,
                    industry: None,
                    total_shares: None,
                    circulating_shares: None,
                    list_date: None,
                });
            }
        };

        let (total_shares, circulating_shares) = response
            .result
            .and_then(|r| r.data)
            .and_then(|d| d.first().cloned())
            .map(|item| {
                (
                    item.total_shares.map(|v| v as u64),
                    item.listed_a_shares.map(|v| v as u64),
                )
            })
            .unwrap_or((None, None));

        Ok(StockInfo {
            stock_code: stock_code.to_string(),
            full_name: String::new(),
            short_name,
            exchange,
            industry: None,
            total_shares,
            circulating_shares,
            list_date: None,
        })
    }
}

#[async_trait]
impl FundInfoSource for EastMoneySource {
    async fn get_all_etf_codes(&self) -> DataResult<Vec<ETFCode>> {
        let url = "https://82.push2.eastmoney.com/api/qt/clist/get";
        let mut all_codes = Vec::new();
        let page_size = 50;

        for page in 1..=50 {
            let params = [
                ("pn", page.to_string()),
                ("pz", page_size.to_string()),
                ("po", "1".to_string()),
                ("np", "1".to_string()),
                ("ut", "bd1d9ddb04089700cf9c27f6f7426281".to_string()),
                ("fltt", "2".to_string()),
                ("invt", "2".to_string()),
                ("wbp2u", "|0|0|0|web".to_string()),
                ("fid", "f3".to_string()),
                ("fs", "b:MK0021,b:MK0022,b:MK0023,b:MK0024".to_string()),
                ("fields", "f12,f14,f2".to_string()),
            ];

            debug!("Fetching ETF codes page {} from East Money", page);

            let response: StockListResponse = match self.request.get_json_with_params(url, &params).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Failed to fetch ETF page {}: {}", page, e);
                    break;
                }
            };

            let items = match response.data.and_then(|d| d.diff) {
                Some(items) if !items.is_empty() => items,
                _ => break,
            };

            let count = items.len();

            for item in items {
                let exchange = Exchange::from_stock_code(&item.code);
                all_codes.push(ETFCode {
                    fund_code: item.code,
                    short_name: item.name,
                    exchange,
                    net_value: item.price,
                });
            }

            if count < page_size {
                break;
            }
        }

        Ok(all_codes)
    }
}

#[async_trait]
impl FundMarketSource for EastMoneySource {
    async fn get_etf_market(
        &self,
        fund_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<ETFMarketData>> {
        let secid = Self::get_secid(fund_code);
        let klt = Self::kline_to_klt(k_type);

        let start = start_date
            .map(|s| s.replace('-', ""))
            .unwrap_or_else(|| "19900101".to_string());
        let end = end_date
            .map(|s| s.replace('-', ""))
            .unwrap_or_else(|| chrono::Utc::now().format("%Y%m%d").to_string());

        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f116"),
            ("ut", "7eea3edcaed734bea9cbfc24409ed989"),
            ("klt", &klt.to_string()),
            ("fqt", "1"),
            ("secid", &secid),
            ("beg", &start),
            ("end", &end),
        ];

        let url = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
        debug!("Fetching ETF market data from East Money: {}", fund_code);

        let response: KLineResponse = self.request.get_json_with_params(url, &params).await?;
        let klines = response.data.and_then(|d| d.klines).unwrap_or_default();

        if klines.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::with_capacity(klines.len());

        for line in klines {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 11 {
                continue;
            }

            let trade_date = NaiveDate::parse_from_str(parts[0], "%Y-%m-%d")
                .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

            let open: f64 = parts[1].parse().unwrap_or(0.0);
            let close: f64 = parts[2].parse().unwrap_or(0.0);
            let high: f64 = parts[3].parse().unwrap_or(0.0);
            let low: f64 = parts[4].parse().unwrap_or(0.0);
            let volume: u64 = parts[5].parse::<f64>().unwrap_or(0.0) as u64;
            let amount: f64 = parts[6].parse().unwrap_or(0.0);
            let change_pct: f64 = parts[8].parse().unwrap_or(0.0);
            let change: f64 = parts[9].parse().unwrap_or(0.0);

            result.push(ETFMarketData {
                fund_code: fund_code.to_string(),
                trade_time: Utc.from_utc_datetime(&trade_date.and_hms_opt(15, 0, 0).unwrap()),
                trade_date,
                open,
                close,
                high,
                low,
                volume,
                amount,
                change: Some(change),
                change_pct: Some(change_pct),
            });
        }

        Ok(result)
    }

    async fn get_etf_current(&self, fund_codes: &[&str]) -> DataResult<Vec<ETFCurrentData>> {
        if fund_codes.is_empty() {
            return Ok(Vec::new());
        }

        let codes_str: String = fund_codes
            .iter()
            .map(|c| Self::get_secid(c))
            .collect::<Vec<_>>()
            .join(",");

        let params = [
            ("pn", "1"),
            ("pz", &fund_codes.len().to_string()),
            ("po", "1"),
            ("np", "1"),
            ("ut", "bd1d9ddb04089700cf9c27f6f7426281"),
            ("fltt", "2"),
            ("invt", "2"),
            ("fid", "f3"),
            ("fs", &format!("b:{codes_str}")),
            ("fields", "f2,f3,f4,f5,f6,f12,f14"),
        ];

        let url = "https://push2.eastmoney.com/api/qt/clist/get";
        let response: StockListResponse = self.request.get_json_with_params(url, &params).await?;

        let items = response.data.and_then(|d| d.diff).unwrap_or_default();

        let result: Vec<ETFCurrentData> = items
            .into_iter()
            .map(|item| ETFCurrentData {
                fund_code: item.code,
                short_name: item.name,
                price: item.price.unwrap_or(0.0),
                change: item.change,
                change_pct: item.change_pct,
                volume: item.volume.unwrap_or(0),
                amount: item.amount.unwrap_or(0.0),
                open: None,
                high: None,
                low: None,
            })
            .collect();

        Ok(result)
    }

    async fn get_etf_min(&self, fund_code: &str) -> DataResult<Vec<ETFMinuteData>> {
        let secid = Self::get_secid(fund_code);

        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58"),
            ("ut", "fa5fd1943c7b386f172d6893dbfba10b"),
            ("ndays", "1"),
            ("iscr", "1"),
            ("iscca", "0"),
            ("secid", &secid),
        ];

        let url = "https://push2.eastmoney.com/api/qt/stock/trends2/get";
        debug!("Fetching ETF minute data from East Money: {}", fund_code);

        let response: MinuteResponse = self.request.get_json_with_params(url, &params).await?;

        let data = match response.data {
            Some(d) => d,
            None => return Ok(Vec::new()),
        };

        let pre_close = data.pre_close;
        let trends = data.trends.unwrap_or_default();

        if trends.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::with_capacity(trends.len());
        let today = Utc::now().date_naive();

        for line in trends {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 8 {
                continue;
            }

            let time_str = parts[0];
            let trade_time = if time_str.contains(' ') {
                chrono::NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M")
                    .map(|dt| Utc.from_utc_datetime(&dt))
                    .unwrap_or_else(|_| Utc::now())
            } else {
                chrono::NaiveTime::parse_from_str(time_str, "%H:%M")
                    .map(|t| Utc.from_utc_datetime(&today.and_time(t)))
                    .unwrap_or_else(|_| Utc::now())
            };

            let price: f64 = parts[2].parse().unwrap_or(0.0);
            let volume: u64 = parts[5].parse::<f64>().unwrap_or(0.0) as u64;
            let amount: f64 = parts[6].parse().unwrap_or(0.0);
            let avg_price: f64 = parts[7].parse().unwrap_or(price);

            let change = price - pre_close;
            let change_pct = if pre_close > 0.0 {
                (change / pre_close * 100.0 * 100.0).round() / 100.0
            } else {
                0.0
            };

            result.push(ETFMinuteData {
                fund_code: fund_code.to_string(),
                trade_time,
                price,
                change,
                change_pct,
                volume,
                avg_price,
                amount,
            });
        }

        Ok(result)
    }
}

#[derive(Debug, Deserialize)]
struct StockInfoResponse {
    result: Option<StockInfoResult>,
}

#[derive(Debug, Deserialize)]
struct StockInfoResult {
    data: Option<Vec<StockInfoItem>>,
}

#[derive(Debug, Deserialize, Clone)]
struct StockInfoItem {
    #[serde(rename = "TOTAL_SHARES", default)]
    total_shares: Option<f64>,
    #[serde(rename = "LISTED_A_SHARES", default)]
    listed_a_shares: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct BondListResponse {
    result: Option<BondListResult>,
}

#[derive(Debug, Deserialize)]
struct BondListResult {
    data: Option<Vec<BondItem>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BondItem {
    #[serde(rename = "SECURITY_CODE")]
    bond_code: String,
    #[serde(rename = "SECURITY_NAME_ABBR")]
    bond_name: String,
    #[serde(rename = "CONVERT_STOCK_CODE", default)]
    stock_code: Option<String>,
    #[serde(rename = "SECURITY_SHORT_NAME", default)]
    short_name: Option<String>,
    #[serde(rename = "PUBLIC_START_DATE", default)]
    sub_date: Option<String>,
    #[serde(rename = "ACTUAL_ISSUE_SCALE", default)]
    issue_amount: Option<f64>,
    #[serde(rename = "LISTING_DATE", default)]
    listing_date: Option<String>,
    #[serde(rename = "EXPIRE_DATE", default)]
    expire_date: Option<String>,
    #[serde(rename = "TRANSFER_PRICE", default)]
    convert_price: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct BondQuoteResponse {
    data: Option<BondQuoteData>,
}

#[derive(Debug, Deserialize)]
struct BondQuoteData {
    diff: Option<Vec<BondQuoteItem>>,
}

#[derive(Debug, Deserialize)]
struct BondQuoteItem {
    #[serde(rename = "f12")]
    bond_code: String,
    #[serde(rename = "f14")]
    bond_name: String,
    #[serde(rename = "f2", default)]
    price: Option<f64>,
    #[serde(rename = "f3", default)]
    change_pct: Option<f64>,
    #[serde(rename = "f4", default)]
    change: Option<f64>,
    #[serde(rename = "f5", default)]
    volume: Option<u64>,
    #[serde(rename = "f6", default)]
    amount: Option<f64>,
    #[serde(rename = "f15", default)]
    high: Option<f64>,
    #[serde(rename = "f16", default)]
    low: Option<f64>,
    #[serde(rename = "f17", default)]
    open: Option<f64>,
    #[serde(rename = "f18", default)]
    pre_close: Option<f64>,
}

#[async_trait]
impl BondInfoSource for EastMoneySource {
    async fn get_all_bond_codes(&self) -> DataResult<Vec<ConvertibleBondCode>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";
        let mut all_bonds = Vec::new();
        let page_size = 50;

        for page in 1..=100 {
            let params = [
                ("sortColumns", "PUBLIC_START_DATE"),
                ("sortTypes", "-1"),
                ("pageSize", &page_size.to_string()),
                ("pageNumber", &page.to_string()),
                ("reportName", "RPT_BOND_CB_LIST"),
                ("columns", "SECURITY_CODE,SECURITY_NAME_ABBR,CONVERT_STOCK_CODE,SECURITY_SHORT_NAME,PUBLIC_START_DATE,ACTUAL_ISSUE_SCALE,LISTING_DATE,EXPIRE_DATE,TRANSFER_PRICE"),
                ("quoteColumns", ""),
                ("source", "WEB"),
                ("client", "WEB"),
            ];

            debug!("Fetching bond codes page {} from East Money", page);

            let response: BondListResponse = match self.request.get_json_with_params(url, &params).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Failed to fetch bond page {}: {}", page, e);
                    break;
                }
            };

            let items = match response.result.and_then(|r| r.data) {
                Some(items) if !items.is_empty() => items,
                _ => break,
            };

            let count = items.len();

            for item in items {
                let sub_date = item.sub_date.as_ref().and_then(|s| {
                    NaiveDate::parse_from_str(&s[..10], "%Y-%m-%d").ok()
                });
                let listing_date = item.listing_date.as_ref().and_then(|s| {
                    NaiveDate::parse_from_str(&s[..10], "%Y-%m-%d").ok()
                });
                let expire_date = item.expire_date.as_ref().and_then(|s| {
                    NaiveDate::parse_from_str(&s[..10], "%Y-%m-%d").ok()
                });

                all_bonds.push(ConvertibleBondCode {
                    bond_code: item.bond_code,
                    bond_name: item.bond_name,
                    stock_code: item.stock_code.unwrap_or_default(),
                    short_name: item.short_name.unwrap_or_default(),
                    sub_date,
                    issue_amount: item.issue_amount,
                    listing_date,
                    expire_date,
                    convert_price: item.convert_price,
                });
            }

            if count < page_size {
                break;
            }
        }

        Ok(all_bonds)
    }
}

#[async_trait]
impl BondMarketSource for EastMoneySource {
    async fn get_bond_current(&self, bond_codes: Option<&[&str]>) -> DataResult<Vec<BondCurrentData>> {
        let url = "https://push2.eastmoney.com/api/qt/clist/get";
        let mut all_bonds = Vec::new();
        let page_size = 100;

        for page in 1..=50 {
            let params = [
                ("pn", page.to_string()),
                ("pz", page_size.to_string()),
                ("po", "1".to_string()),
                ("np", "1".to_string()),
                ("ut", "bd1d9ddb04089700cf9c27f6f7426281".to_string()),
                ("fltt", "2".to_string()),
                ("invt", "2".to_string()),
                ("fid", "f3".to_string()),
                ("fs", "b:MK0354".to_string()),
                ("fields", "f2,f3,f4,f5,f6,f12,f14,f15,f16,f17,f18".to_string()),
            ];

            debug!("Fetching bond market page {} from East Money", page);

            let response: BondQuoteResponse = match self.request.get_json_with_params(url, &params).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Failed to fetch bond market page {}: {}", page, e);
                    break;
                }
            };

            let items = match response.data.and_then(|d| d.diff) {
                Some(items) if !items.is_empty() => items,
                _ => break,
            };

            let count = items.len();

            for item in items {
                if let Some(codes) = bond_codes {
                    if !codes.contains(&item.bond_code.as_str()) {
                        continue;
                    }
                }

                all_bonds.push(BondCurrentData {
                    bond_code: item.bond_code,
                    bond_name: item.bond_name,
                    price: item.price.unwrap_or(0.0),
                    open: item.open.unwrap_or(0.0),
                    high: item.high.unwrap_or(0.0),
                    low: item.low.unwrap_or(0.0),
                    pre_close: item.pre_close.unwrap_or(0.0),
                    change: item.change.unwrap_or(0.0),
                    change_pct: item.change_pct.unwrap_or(0.0),
                    volume: item.volume.unwrap_or(0),
                    amount: item.amount.unwrap_or(0.0),
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

        Ok(all_bonds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secid() {
        assert_eq!(EastMoneySource::get_secid("600000"), "1.600000");
        assert_eq!(EastMoneySource::get_secid("000001"), "0.000001");
        assert_eq!(EastMoneySource::get_secid("300001"), "0.300001");
    }

    #[test]
    fn test_kline_to_klt() {
        assert_eq!(EastMoneySource::kline_to_klt(KLineType::Daily), 101);
        assert_eq!(EastMoneySource::kline_to_klt(KLineType::Min5), 5);
    }
}
