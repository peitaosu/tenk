//! East Money data source.

use async_trait::async_trait;
use chrono::{FixedOffset, NaiveDate, TimeZone, Utc};
use serde::Deserialize;
use tracing::{debug, warn};

use crate::data::{
    BillboardDetail, BillboardItem, BlockTradeData, BondCurrentData, CapitalFlowData,
    CapitalFlowHistory, ConvertibleBondCode, CurrentMarketData, DividendData, ETFCode,
    ETFCurrentData, ETFMarketData, ETFMinuteData, EarningsForecast, Exchange, FundHolding, IPOData,
    InstitutionalResearchData, KLineType, MarginTradingData, MarketData, MinuteData, NewsArticle,
    NewsCategory, NewsContent, ResearchReportData, StockCode, StockConnectData, StockInfo,
    StockValuation, TopHolder,
};
use crate::error::DataResult;
use crate::request::RequestManager;
use crate::traits::{
    BillboardSource, BlockTradeSource, BondInfoSource, BondMarketSource, CapitalFlowSource,
    DataSource, DividendSource, EarningsForecastSource, FundInfoSource, FundMarketSource,
    HoldingsSource, IPOSource, InstitutionalResearchSource, MarginTradingSource, NewsSource,
    ResearchReportSource, StockConnectSource, StockInfoSource, StockMarketSource, ValuationSource,
};

/// East Money data source.
#[derive(Debug, Clone)]
pub struct EastMoneySource {
    /// HTTP request manager
    request: RequestManager,
}

impl EastMoneySource {
    /// Creates a new EastMoney source.
    pub fn new() -> DataResult<Self> {
        Ok(Self {
            request: RequestManager::default_manager()?,
        })
    }

    /// Creates with a custom request manager.
    pub fn with_request_manager(request: RequestManager) -> Self {
        Self { request }
    }

    /// Converts stock code to EastMoney secid format.
    fn get_secid(stock_code: &str) -> String {
        let prefix = if stock_code.starts_with('6') || stock_code.starts_with('5') {
            "1"
        } else {
            "0"
        };
        format!("{prefix}.{stock_code}")
    }

    /// Converts KLineType to EastMoney API klt value.
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
    /// Creates default EastMoney source.
    fn default() -> Self {
        Self::new().expect("Failed to create EastMoneySource")
    }
}

/// Returns Beijing timezone (UTC+8).
fn beijing_tz() -> FixedOffset {
    FixedOffset::east_opt(8 * 3600).unwrap()
}

/// K-line API response.
#[derive(Debug, Deserialize)]
struct KLineResponse {
    /// K-line data
    data: Option<KLineData>,
}

/// K-line data wrapper.
#[derive(Debug, Deserialize)]
struct KLineData {
    /// K-line strings
    klines: Option<Vec<String>>,
}

/// Minute data API response.
#[derive(Debug, Deserialize)]
struct MinuteResponse {
    /// Minute data
    data: Option<MinuteResponseData>,
}

/// Minute data wrapper.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MinuteResponseData {
    /// Previous close price
    pre_close: f64,
    /// Trend data strings
    trends: Option<Vec<String>>,
}

/// Stock list API response.
#[derive(Debug, Deserialize)]
struct StockListResponse {
    /// Stock list data
    data: Option<StockListData>,
}

/// Stock list data wrapper.
#[derive(Debug, Deserialize)]
struct StockListData {
    /// Stock items
    diff: Option<Vec<StockItem>>,
}

/// Stock item from API.
#[derive(Debug, Deserialize)]
struct StockItem {
    /// Stock code
    #[serde(rename = "f12")]
    code: String,
    /// Stock name
    #[serde(rename = "f14")]
    name: String,
    /// Current price
    #[serde(rename = "f2", default)]
    price: Option<f64>,
}

#[async_trait]
impl DataSource for EastMoneySource {
    /// Returns the source name.
    fn name(&self) -> &'static str {
        "eastmoney"
    }

    /// Returns the source priority.
    fn priority(&self) -> u8 {
        1
    }

    /// Checks if the source is available.
    async fn is_available(&self) -> bool {
        self.request
            .get("https://push2.eastmoney.com/api/qt/stock/trends2/get")
            .await
            .is_ok()
    }
}

#[async_trait]
impl StockMarketSource for EastMoneySource {
    /// Fetches historical K-line market data.
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
            (
                "fields2",
                "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f116",
            ),
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

    /// Fetches real-time market quotes.
    async fn get_market_current(&self, stock_codes: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        if stock_codes.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::with_capacity(stock_codes.len());

        for code in stock_codes {
            let secid = Self::get_secid(code);
            let params = [
                ("secid", secid.as_str()),
                ("fields", "f43,f44,f45,f46,f47,f48,f57,f58,f60,f169,f170"),
            ];

            let url = "https://push2.eastmoney.com/api/qt/stock/get";
            debug!("Fetching stock quote for {}", code);

            #[derive(Deserialize)]
            struct StockGetResponse {
                data: Option<StockGetData>,
            }

            #[derive(Deserialize)]
            struct StockGetData {
                #[serde(rename = "f57")]
                code: String,
                #[serde(rename = "f58")]
                name: String,
                #[serde(rename = "f43", default)]
                price: Option<i64>,
                #[serde(rename = "f44", default)]
                high: Option<i64>,
                #[serde(rename = "f45", default)]
                low: Option<i64>,
                #[serde(rename = "f46", default)]
                open: Option<i64>,
                #[serde(rename = "f47", default)]
                volume: Option<u64>,
                #[serde(rename = "f48", default)]
                amount: Option<f64>,
                #[serde(rename = "f60", default)]
                pre_close: Option<i64>,
                #[serde(rename = "f169", default)]
                change: Option<i64>,
                #[serde(rename = "f170", default)]
                change_pct: Option<i64>,
            }

            match self.request.get_json_with_params(url, &params).await {
                Ok(response) => {
                    let response: StockGetResponse = response;
                    if let Some(data) = response.data {
                        results.push(CurrentMarketData {
                            stock_code: data.code,
                            short_name: data.name,
                            price: data.price.map(|v| v as f64 / 100.0).unwrap_or(0.0),
                            open: data.open.map(|v| v as f64 / 100.0),
                            high: data.high.map(|v| v as f64 / 100.0),
                            low: data.low.map(|v| v as f64 / 100.0),
                            pre_close: data.pre_close.map(|v| v as f64 / 100.0),
                            change: data.change.map(|v| v as f64 / 100.0).unwrap_or(0.0),
                            change_pct: data.change_pct.map(|v| v as f64 / 100.0).unwrap_or(0.0),
                            volume: data.volume.unwrap_or(0) * 100,
                            amount: data.amount.unwrap_or(0.0),
                        });
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch stock {}: {}", code, e);
                }
            }
        }

        Ok(results)
    }

    /// Fetches intraday minute-level data.
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
    /// Fetches all available stock codes.
    async fn get_all_codes(&self, limit: Option<usize>) -> DataResult<Vec<StockCode>> {
        let url = "https://82.push2.eastmoney.com/api/qt/clist/get";
        let mut all_codes = Vec::new();
        let page_size = 100;
        let mut page = 1;

        loop {
            let params = [
                ("pn", page.to_string()),
                ("pz", page_size.to_string()),
                ("po", "1".to_string()),
                ("np", "1".to_string()),
                ("ut", "bd1d9ddb04089700cf9c27f6f7426281".to_string()),
                ("fltt", "2".to_string()),
                ("invt", "2".to_string()),
                ("fid", "f3".to_string()),
                (
                    "fs",
                    "m:0 t:6,m:0 t:80,m:1 t:2,m:1 t:23,m:0 t:81 s:2048".to_string(),
                ),
                ("fields", "f12,f14".to_string()),
            ];

            debug!("Fetching stock codes page {} from East Money", page);

            let response: StockListResponse =
                match self.request.get_json_with_params(url, &params).await {
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

                if let Some(lim) = limit {
                    if all_codes.len() >= lim {
                        return Ok(all_codes);
                    }
                }
            }

            if count < page_size as usize {
                break;
            }
            page += 1;
        }

        Ok(all_codes)
    }

    /// Fetches detailed stock information.
    async fn get_stock_info(&self, stock_code: &str) -> DataResult<StockInfo> {
        let exchange = Exchange::from_stock_code(stock_code);
        let secucode = format!("{}.{}", stock_code, exchange);

        #[derive(Deserialize)]
        struct OrgInfoItem {
            #[serde(rename = "ORG_NAME", default)]
            org_name: Option<String>,
            #[serde(rename = "SECURITY_NAME_ABBR", default)]
            short_name: Option<String>,
            #[serde(rename = "EM2016", default)]
            industry: Option<String>,
            #[serde(rename = "LISTING_DATE", default)]
            listing_date: Option<String>,
        }

        #[derive(Deserialize)]
        struct OrgInfoResult {
            data: Option<Vec<OrgInfoItem>>,
        }

        #[derive(Deserialize)]
        struct OrgInfoResponse {
            result: Option<OrgInfoResult>,
        }

        let url = "https://datacenter.eastmoney.com/securities/api/data/v1/get";
        let org_params = [
            ("reportName", "RPT_F10_ORG_BASICINFO"),
            ("columns", "ORG_NAME,SECURITY_NAME_ABBR,EM2016,LISTING_DATE"),
            ("filter", &format!("(SECUCODE=\"{}\")", secucode)),
            ("pageNumber", "1"),
            ("pageSize", "1"),
            ("source", "HSF10"),
            ("client", "PC"),
        ];

        let org_response: Result<OrgInfoResponse, _> =
            self.request.get_json_with_params(url, &org_params).await;

        let (full_name, short_name, industry, list_date) = match org_response {
            Ok(response) => response
                .result
                .and_then(|r| r.data)
                .and_then(|d| d.into_iter().next())
                .map(|item| {
                    let list_date = item.listing_date.and_then(|s| {
                        s.split(' ').next().and_then(|date_str| {
                            chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
                        })
                    });
                    (
                        item.org_name.unwrap_or_default(),
                        item.short_name.unwrap_or_default(),
                        item.industry,
                        list_date,
                    )
                })
                .unwrap_or_default(),
            Err(_) => (String::new(), String::new(), None, None),
        };

        let shares_params = [
            ("reportName", "RPT_F10_EH_EQUITY"),
            ("columns", "TOTAL_SHARES,LISTED_A_SHARES"),
            ("filter", &format!("(SECUCODE=\"{}\")", secucode)),
            ("pageNumber", "1"),
            ("pageSize", "1"),
            ("sortTypes", "-1"),
            ("sortColumns", "END_DATE"),
            ("source", "HSF10"),
            ("client", "PC"),
        ];

        let shares_response: Result<StockInfoResponse, _> =
            self.request.get_json_with_params(url, &shares_params).await;

        let (total_shares, circulating_shares) = match shares_response {
            Ok(response) => response
                .result
                .and_then(|r| r.data)
                .and_then(|d| d.first().cloned())
                .map(|item| {
                    (
                        item.total_shares.map(|v| v as u64),
                        item.listed_a_shares.map(|v| v as u64),
                    )
                })
                .unwrap_or((None, None)),
            Err(_) => (None, None),
        };

        Ok(StockInfo {
            stock_code: stock_code.to_string(),
            full_name,
            short_name,
            exchange,
            industry,
            total_shares,
            circulating_shares,
            list_date,
        })
    }
}

#[async_trait]
impl FundInfoSource for EastMoneySource {
    /// Fetches all available ETF codes.
    async fn get_all_etf_codes(&self, limit: Option<usize>) -> DataResult<Vec<ETFCode>> {
        let url = "https://82.push2.eastmoney.com/api/qt/clist/get";
        let mut all_codes = Vec::new();
        let page_size = 50;
        let mut page = 1;

        loop {
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

            let response: StockListResponse =
                match self.request.get_json_with_params(url, &params).await {
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

#[async_trait]
impl FundMarketSource for EastMoneySource {
    /// Fetches historical ETF K-line market data.
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
            (
                "fields2",
                "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f116",
            ),
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

    /// Fetches real-time ETF quotes.
    async fn get_etf_current(&self, fund_codes: &[&str]) -> DataResult<Vec<ETFCurrentData>> {
        if fund_codes.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::with_capacity(fund_codes.len());

        for code in fund_codes {
            let secid = Self::get_secid(code);
            let params = [
                ("secid", secid.as_str()),
                ("fields", "f43,f44,f45,f46,f47,f48,f57,f58,f60,f169,f170"),
            ];

            let url = "https://push2.eastmoney.com/api/qt/stock/get";
            debug!("Fetching ETF quote for {}", code);

            #[derive(Deserialize)]
            struct ETFGetResponse {
                data: Option<ETFGetData>,
            }

            #[derive(Deserialize)]
            struct ETFGetData {
                #[serde(rename = "f57")]
                code: String,
                #[serde(rename = "f58")]
                name: String,
                #[serde(rename = "f43", default)]
                price: Option<i64>,
                #[serde(rename = "f44", default)]
                high: Option<i64>,
                #[serde(rename = "f45", default)]
                low: Option<i64>,
                #[serde(rename = "f46", default)]
                open: Option<i64>,
                #[serde(rename = "f47", default)]
                volume: Option<u64>,
                #[serde(rename = "f48", default)]
                amount: Option<f64>,
                #[serde(rename = "f169", default)]
                change: Option<i64>,
                #[serde(rename = "f170", default)]
                change_pct: Option<i64>,
            }

            match self.request.get_json_with_params(url, &params).await {
                Ok(response) => {
                    let response: ETFGetResponse = response;
                    if let Some(data) = response.data {
                        results.push(ETFCurrentData {
                            fund_code: data.code,
                            short_name: data.name,
                            price: data.price.map(|v| v as f64 / 1000.0).unwrap_or(0.0),
                            open: data.open.map(|v| v as f64 / 1000.0),
                            high: data.high.map(|v| v as f64 / 1000.0),
                            low: data.low.map(|v| v as f64 / 1000.0),
                            change: data.change.map(|v| v as f64 / 1000.0),
                            change_pct: data.change_pct.map(|v| v as f64 / 100.0),
                            volume: data.volume.unwrap_or(0),
                            amount: data.amount.unwrap_or(0.0),
                        });
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch ETF {}: {}", code, e);
                }
            }
        }

        Ok(results)
    }

    /// Fetches intraday ETF minute-level data.
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

/// Stock info API response.
#[derive(Debug, Deserialize)]
struct StockInfoResponse {
    /// Result data
    result: Option<StockInfoResult>,
}

/// Stock info result wrapper.
#[derive(Debug, Deserialize)]
struct StockInfoResult {
    /// Stock info items
    data: Option<Vec<StockInfoItem>>,
}

/// Stock info item from API.
#[derive(Debug, Deserialize, Clone)]
struct StockInfoItem {
    /// Total shares
    #[serde(rename = "TOTAL_SHARES", default)]
    total_shares: Option<f64>,
    /// Listed A shares
    #[serde(rename = "LISTED_A_SHARES", default)]
    listed_a_shares: Option<f64>,
}

/// Bond list API response.
#[derive(Debug, Deserialize)]
struct BondListResponse {
    /// Result data
    result: Option<BondListResult>,
}

/// Bond list result wrapper.
#[derive(Debug, Deserialize)]
struct BondListResult {
    /// Bond items
    data: Option<Vec<BondItem>>,
}

/// Bond item from API.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BondItem {
    /// Bond code
    #[serde(rename = "SECURITY_CODE")]
    bond_code: String,
    /// Bond name
    #[serde(rename = "SECURITY_NAME_ABBR")]
    bond_name: String,
    /// Underlying stock code
    #[serde(rename = "CONVERT_STOCK_CODE", default)]
    stock_code: Option<String>,
    /// Short name
    #[serde(rename = "SECURITY_SHORT_NAME", default)]
    short_name: Option<String>,
    /// Subscription date
    #[serde(rename = "PUBLIC_START_DATE", default)]
    sub_date: Option<String>,
    /// Issue amount
    #[serde(rename = "ACTUAL_ISSUE_SCALE", default)]
    issue_amount: Option<f64>,
    /// Listing date
    #[serde(rename = "LISTING_DATE", default)]
    listing_date: Option<String>,
    /// Expiration date
    #[serde(rename = "EXPIRE_DATE", default)]
    expire_date: Option<String>,
    /// Conversion price
    #[serde(rename = "TRANSFER_PRICE", default)]
    convert_price: Option<f64>,
}

/// Bond quote API response.
#[derive(Debug, Deserialize)]
struct BondQuoteResponse {
    /// Quote data
    data: Option<BondQuoteData>,
}

/// Bond quote data wrapper.
#[derive(Debug, Deserialize)]
struct BondQuoteData {
    /// Quote items
    diff: Option<Vec<BondQuoteItem>>,
}

/// Bond quote item from API.
#[derive(Debug, Deserialize)]
struct BondQuoteItem {
    /// Bond code
    #[serde(rename = "f12")]
    bond_code: String,
    /// Bond name
    #[serde(rename = "f14")]
    bond_name: String,
    /// Current price
    #[serde(rename = "f2", default)]
    price: Option<f64>,
    /// Change percentage
    #[serde(rename = "f3", default)]
    change_pct: Option<f64>,
    /// Price change
    #[serde(rename = "f4", default)]
    change: Option<f64>,
    /// Volume
    #[serde(rename = "f5", default)]
    volume: Option<u64>,
    /// Amount
    #[serde(rename = "f6", default)]
    amount: Option<f64>,
    /// High price
    #[serde(rename = "f15", default)]
    high: Option<f64>,
    /// Low price
    #[serde(rename = "f16", default)]
    low: Option<f64>,
    /// Open price
    #[serde(rename = "f17", default)]
    open: Option<f64>,
    /// Previous close
    #[serde(rename = "f18", default)]
    pre_close: Option<f64>,
}

#[async_trait]
impl BondInfoSource for EastMoneySource {
    /// Fetches all available convertible bond codes.
    async fn get_all_bond_codes(
        &self,
        limit: Option<usize>,
    ) -> DataResult<Vec<ConvertibleBondCode>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";
        let mut all_bonds = Vec::new();
        let page_size = 50;
        let mut page = 1;

        loop {
            let params = [
                ("sortColumns", "PUBLIC_START_DATE"),
                ("sortTypes", "-1"),
                ("pageSize", &page_size.to_string()),
                ("pageNumber", &page.to_string()),
                ("reportName", "RPT_BOND_CB_LIST"),
                (
                    "columns",
                    "SECURITY_CODE,SECURITY_NAME_ABBR,CONVERT_STOCK_CODE,SECURITY_SHORT_NAME,PUBLIC_START_DATE,ACTUAL_ISSUE_SCALE,LISTING_DATE,EXPIRE_DATE,TRANSFER_PRICE",
                ),
                ("quoteColumns", ""),
                ("source", "WEB"),
                ("client", "WEB"),
            ];

            debug!("Fetching bond codes page {} from East Money", page);

            let response: BondListResponse =
                match self.request.get_json_with_params(url, &params).await {
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
                let sub_date = item
                    .sub_date
                    .as_ref()
                    .and_then(|s| NaiveDate::parse_from_str(&s[..10], "%Y-%m-%d").ok());
                let listing_date = item
                    .listing_date
                    .as_ref()
                    .and_then(|s| NaiveDate::parse_from_str(&s[..10], "%Y-%m-%d").ok());
                let expire_date = item
                    .expire_date
                    .as_ref()
                    .and_then(|s| NaiveDate::parse_from_str(&s[..10], "%Y-%m-%d").ok());

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
impl BondMarketSource for EastMoneySource {
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

impl EastMoneySource {
    /// Fetches bond data for specific codes directly.
    async fn get_bond_current_by_codes(&self, codes: &[&str]) -> DataResult<Vec<BondCurrentData>> {
        let mut results = Vec::with_capacity(codes.len());

        for code in codes {
            let secid = Self::get_secid(code);
            let params = [
                ("secid", secid.as_str()),
                ("fields", "f43,f44,f45,f46,f47,f48,f57,f58,f60,f169,f170"),
            ];

            let url = "https://push2.eastmoney.com/api/qt/stock/get";
            debug!("Fetching bond quote for {}", code);

            #[derive(Deserialize)]
            struct BondGetResponse {
                data: Option<BondGetData>,
            }

            #[derive(Deserialize)]
            struct BondGetData {
                #[serde(rename = "f57")]
                code: String,
                #[serde(rename = "f58")]
                name: String,
                #[serde(rename = "f43", default)]
                price: Option<i64>,
                #[serde(rename = "f44", default)]
                high: Option<i64>,
                #[serde(rename = "f45", default)]
                low: Option<i64>,
                #[serde(rename = "f46", default)]
                open: Option<i64>,
                #[serde(rename = "f47", default)]
                volume: Option<u64>,
                #[serde(rename = "f48", default)]
                amount: Option<f64>,
                #[serde(rename = "f60", default)]
                pre_close: Option<i64>,
                #[serde(rename = "f169", default)]
                change: Option<i64>,
                #[serde(rename = "f170", default)]
                change_pct: Option<i64>,
            }

            match self.request.get_json_with_params(url, &params).await {
                Ok(response) => {
                    let response: BondGetResponse = response;
                    if let Some(data) = response.data {
                        results.push(BondCurrentData {
                            bond_code: data.code,
                            bond_name: data.name,
                            price: data.price.map(|v| v as f64 / 1000.0).unwrap_or(0.0),
                            open: data.open.map(|v| v as f64 / 1000.0).unwrap_or(0.0),
                            high: data.high.map(|v| v as f64 / 1000.0).unwrap_or(0.0),
                            low: data.low.map(|v| v as f64 / 1000.0).unwrap_or(0.0),
                            pre_close: data.pre_close.map(|v| v as f64 / 1000.0).unwrap_or(0.0),
                            change: data.change.map(|v| v as f64 / 1000.0).unwrap_or(0.0),
                            change_pct: data.change_pct.map(|v| v as f64 / 100.0).unwrap_or(0.0),
                            volume: data.volume.unwrap_or(0),
                            amount: data.amount.unwrap_or(0.0),
                        });
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch bond {}: {}", code, e);
                }
            }
        }

        Ok(results)
    }

    /// Fetches all bond data with pagination.
    async fn get_all_bond_current(&self) -> DataResult<Vec<BondCurrentData>> {
        let url = "https://push2.eastmoney.com/api/qt/clist/get";
        let mut all_bonds = Vec::new();
        let page_size = 100;
        let mut page = 1;

        loop {
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
                (
                    "fields",
                    "f2,f3,f4,f5,f6,f12,f14,f15,f16,f17,f18".to_string(),
                ),
            ];

            debug!("Fetching bond market page {} from East Money", page);

            let response: BondQuoteResponse =
                match self.request.get_json_with_params(url, &params).await {
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
            page += 1;
        }

        Ok(all_bonds)
    }
}

/// News API response.
#[derive(Debug, Deserialize)]
struct NewsResponse {
    /// Response code
    #[serde(default)]
    rc: i32,
    /// News items
    news: Option<Vec<NewsItem>>,
}

/// News item from API.
#[derive(Debug, Deserialize)]
struct NewsItem {
    /// News ID
    id: String,
    /// Title
    title: String,
    /// Digest/summary
    #[serde(default)]
    digest: String,
    /// Web URL
    #[serde(default)]
    url_w: String,
    /// Mobile URL
    #[serde(default)]
    url_m: String,
    /// Media name
    #[serde(rename = "Art_Media_Name", default)]
    media_name: String,
    /// Publish time
    #[serde(default)]
    showtime: String,
    /// Comment count
    #[serde(default)]
    commentnum: String,
    /// Image flag
    #[serde(default)]
    image: String,
    /// Image URL
    #[serde(rename = "Art_24Image", default)]
    image_url: String,
}

#[async_trait]
impl NewsSource for EastMoneySource {
    /// Fetches news articles by category.
    async fn get_news(
        &self,
        category: NewsCategory,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<NewsArticle>> {
        let column = category.to_column_code();
        let url = "https://newsapi.eastmoney.com/kuaixun/v2/api/list";

        let params = [
            ("column", column),
            ("limit", &limit.to_string()),
            ("p", &page.to_string()),
        ];

        debug!(
            "Fetching news from East Money: category={:?}, page={}",
            category, page
        );

        let response: NewsResponse = self.request.get_json_with_params(url, &params).await?;

        if response.rc != 1 {
            return Ok(Vec::new());
        }

        let items = response.news.unwrap_or_default();
        let mut articles = Vec::with_capacity(items.len());

        for item in items {
            let publish_time =
                chrono::NaiveDateTime::parse_from_str(&item.showtime, "%Y-%m-%d %H:%M:%S")
                    .ok()
                    .and_then(|dt| beijing_tz().from_local_datetime(&dt).single())
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now);

            let comment_count: u32 = item.commentnum.parse().unwrap_or(0);
            let has_image = !item.image.is_empty() || !item.image_url.is_empty();
            let image_url = if !item.image_url.is_empty() {
                Some(item.image_url)
            } else if !item.image.is_empty() {
                Some(item.image)
            } else {
                None
            };

            articles.push(NewsArticle {
                id: item.id,
                title: item.title,
                digest: item.digest,
                url: item.url_w,
                url_mobile: if item.url_m.is_empty() {
                    None
                } else {
                    Some(item.url_m)
                },
                source: item.media_name,
                publish_time,
                category,
                comment_count,
                has_image,
                image_url,
            });
        }

        Ok(articles)
    }

    /// Fetches full news content by ID.
    async fn get_news_content(&self, news_id: &str) -> DataResult<NewsContent> {
        let url = "https://newsinfo.eastmoney.com/kuaixun/v2/api/content";
        let params = [("newsid", news_id)];

        debug!("Fetching news content from East Money: id={}", news_id);

        #[derive(Deserialize)]
        struct RelatedStock {
            #[serde(rename = "Code", default)]
            code: String,
        }

        #[derive(Deserialize)]
        struct ContentResponse {
            newsid: String,
            title: String,
            #[serde(default)]
            description: String,
            #[serde(default)]
            body: String,
            #[serde(default)]
            source: String,
            #[serde(default)]
            author: String,
            #[serde(default)]
            showtime: String,
            #[serde(default)]
            relatedstocks: Vec<RelatedStock>,
            #[serde(default, deserialize_with = "deserialize_images")]
            images: Vec<String>,
        }

        fn deserialize_images<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            use serde::de::{SeqAccess, Visitor};
            use std::fmt;

            struct ImagesVisitor;

            impl<'de> Visitor<'de> for ImagesVisitor {
                type Value = Vec<String>;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a sequence of strings or image objects")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    let mut images = Vec::new();
                    while let Some(value) = seq.next_element::<serde_json::Value>()? {
                        match value {
                            serde_json::Value::String(s) => images.push(s),
                            serde_json::Value::Object(obj) => {
                                if let Some(serde_json::Value::String(src)) = obj.get("src") {
                                    images.push(src.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                    Ok(images)
                }
            }

            deserializer.deserialize_seq(ImagesVisitor)
        }

        let response: ContentResponse = self.request.get_json_with_params(url, &params).await?;

        let publish_time =
            chrono::NaiveDateTime::parse_from_str(&response.showtime, "%Y-%m-%d %H:%M:%S")
                .ok()
                .and_then(|dt| beijing_tz().from_local_datetime(&dt).single())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

        let body_text = html2text::from_read(response.body.as_bytes(), usize::MAX)
            .unwrap_or_default()
            .trim()
            .to_string();

        let related_stocks: Vec<String> = response
            .relatedstocks
            .into_iter()
            .flat_map(|rs| rs.code.split(',').map(String::from).collect::<Vec<_>>())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(NewsContent {
            id: response.newsid,
            title: response.title,
            description: response.description,
            body_html: response.body,
            body_text,
            source: response.source,
            author: if response.author.is_empty() {
                None
            } else {
                Some(response.author)
            },
            publish_time,
            related_stocks,
            images: response.images,
        })
    }

    /// Searches news articles by keyword.
    async fn search_news(
        &self,
        keyword: &str,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<NewsArticle>> {
        let url = "https://search-api-web.eastmoney.com/search/jsonp";

        let param = serde_json::json!({
            "uid": "",
            "keyword": keyword,
            "type": ["cmsArticleWebOld"],
            "client": "web",
            "clientType": "web",
            "clientVersion": "curr",
            "param": {
                "cmsArticleWebOld": {
                    "searchScope": "default",
                    "sort": "default",
                    "pageIndex": page,
                    "pageSize": limit,
                    "preTag": "",
                    "postTag": ""
                }
            }
        });

        let callback = format!("jQuery_{}", chrono::Utc::now().timestamp_millis());
        let params = [("cb", callback.as_str()), ("param", &param.to_string())];

        debug!(
            "Searching news from East Money: keyword={}, page={}",
            keyword, page
        );

        let response = self.request.get_with_params(url, &params).await?;
        let response_text = response.text().await.map_err(|e| {
            crate::error::DataError::custom(format!("Failed to read response: {}", e))
        })?;

        let prefix = format!("{}(", callback);
        let json_str = response_text
            .trim()
            .strip_prefix(&prefix)
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or(&response_text);

        #[derive(Deserialize)]
        struct SearchResponse {
            #[serde(default)]
            result: Option<SearchResult>,
        }

        #[derive(Deserialize)]
        struct SearchResult {
            #[serde(rename = "cmsArticleWebOld", default)]
            articles: Vec<SearchArticle>,
        }

        #[derive(Deserialize)]
        struct SearchArticle {
            #[serde(default)]
            code: String,
            #[serde(default)]
            title: String,
            #[serde(default)]
            content: String,
            #[serde(default)]
            url: String,
            #[serde(rename = "mediaName", default)]
            media_name: String,
            #[serde(default)]
            date: String,
            #[serde(rename = "imgUrl", default)]
            img_url: String,
        }

        let search_response: SearchResponse = serde_json::from_str(json_str).map_err(|e| {
            crate::error::DataError::custom(format!("Failed to parse search response: {}", e))
        })?;

        let items = search_response
            .result
            .map(|r| r.articles)
            .unwrap_or_default();

        let mut articles = Vec::with_capacity(items.len());

        for item in items {
            let publish_time =
                chrono::NaiveDateTime::parse_from_str(&item.date, "%Y-%m-%d %H:%M:%S")
                    .or_else(|_| chrono::NaiveDateTime::parse_from_str(&item.date, "%Y-%m-%d"))
                    .ok()
                    .and_then(|dt| beijing_tz().from_local_datetime(&dt).single())
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now);

            let has_image = !item.img_url.is_empty();

            articles.push(NewsArticle {
                id: item.code,
                title: item.title,
                digest: item.content,
                url: item.url,
                url_mobile: None,
                source: item.media_name,
                publish_time,
                category: NewsCategory::Finance,
                comment_count: 0,
                has_image,
                image_url: if has_image { Some(item.img_url) } else { None },
            });
        }

        Ok(articles)
    }
}

#[async_trait]
impl CapitalFlowSource for EastMoneySource {
    /// Fetches capital flow data for stocks.
    async fn get_capital_flow(&self, stock_codes: &[&str]) -> DataResult<Vec<CapitalFlowData>> {
        if stock_codes.is_empty() {
            return Ok(Vec::new());
        }

        let secids: Vec<String> = stock_codes.iter().map(|c| Self::get_secid(c)).collect();
        let secids_str = secids.join(",");

        let url = "https://push2.eastmoney.com/api/qt/ulist.np/get";
        let params = [
            ("fltt", "2"),
            ("secids", &secids_str),
            (
                "fields",
                "f12,f14,f62,f184,f66,f69,f72,f75,f78,f81,f84,f87,f64,f65",
            ),
            ("ut", "b2884a393a59ad64002292a3e90d46a5"),
        ];

        debug!("Fetching capital flow for {} stocks", stock_codes.len());

        #[derive(Deserialize)]
        struct FlowResponse {
            data: Option<FlowData>,
        }

        #[derive(Deserialize)]
        struct FlowData {
            diff: Option<Vec<FlowItem>>,
        }

        #[derive(Deserialize)]
        struct FlowItem {
            #[serde(rename = "f12")]
            code: String,
            #[serde(rename = "f14")]
            name: String,
            #[serde(rename = "f62", default)]
            main_net: Option<f64>,
            #[serde(rename = "f64", default)]
            main_in: Option<f64>,
            #[serde(rename = "f65", default)]
            main_out: Option<f64>,
            #[serde(rename = "f66", default)]
            super_large_net: Option<f64>,
            #[serde(rename = "f72", default)]
            large_net: Option<f64>,
            #[serde(rename = "f78", default)]
            medium_net: Option<f64>,
            #[serde(rename = "f84", default)]
            small_net: Option<f64>,
            #[serde(rename = "f184", default)]
            main_net_ratio: Option<f64>,
        }

        let response: FlowResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.data.and_then(|d| d.diff).unwrap_or_default();

        Ok(items
            .into_iter()
            .map(|item| CapitalFlowData {
                stock_code: item.code,
                stock_name: item.name,
                main_net_inflow: item.main_net.unwrap_or(0.0),
                main_inflow: item.main_in.unwrap_or(0.0),
                main_outflow: item.main_out.unwrap_or(0.0),
                super_large_net_inflow: item.super_large_net.unwrap_or(0.0),
                large_net_inflow: item.large_net.unwrap_or(0.0),
                medium_net_inflow: item.medium_net.unwrap_or(0.0),
                small_net_inflow: item.small_net.unwrap_or(0.0),
                main_net_ratio: item.main_net_ratio.unwrap_or(0.0),
            })
            .collect())
    }

    /// Fetches historical capital flow data for a stock.
    async fn get_capital_flow_history(
        &self,
        stock_code: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<CapitalFlowHistory>> {
        let secid = Self::get_secid(stock_code);
        let lmt = limit.unwrap_or(30).to_string();

        let url = "https://push2his.eastmoney.com/api/qt/stock/fflow/daykline/get";
        let params = [
            ("secid", secid.as_str()),
            ("fields1", "f1,f2,f3,f7"),
            (
                "fields2",
                "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f62,f63,f64,f65",
            ),
            ("lmt", &lmt),
            ("klt", "101"),
            ("ut", "b2884a393a59ad64002292a3e90d46a5"),
        ];

        debug!("Fetching capital flow history for {}", stock_code);

        #[derive(Deserialize)]
        struct HistoryResponse {
            data: Option<HistoryData>,
        }

        #[derive(Deserialize)]
        struct HistoryData {
            klines: Option<Vec<String>>,
        }

        let response: HistoryResponse = self.request.get_json_with_params(url, &params).await?;
        let klines = response.data.and_then(|d| d.klines).unwrap_or_default();

        let mut result = Vec::with_capacity(klines.len());
        for line in klines {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 10 {
                continue;
            }

            let trade_date = NaiveDate::parse_from_str(parts[0], "%Y-%m-%d")
                .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

            result.push(CapitalFlowHistory {
                stock_code: stock_code.to_string(),
                trade_date,
                main_net_inflow: parts[1].parse().unwrap_or(0.0),
                small_net_inflow: parts[2].parse().unwrap_or(0.0),
                medium_net_inflow: parts[3].parse().unwrap_or(0.0),
                large_net_inflow: parts[4].parse().unwrap_or(0.0),
                super_large_net_inflow: parts[5].parse().unwrap_or(0.0),
                close: parts[8].parse().unwrap_or(0.0),
                change_pct: parts[9].parse().unwrap_or(0.0),
            });
        }

        Ok(result)
    }
}

#[async_trait]
impl BillboardSource for EastMoneySource {
    /// Fetches Billboard list for a given date.
    async fn get_billboard_list(&self, _date: Option<&str>) -> DataResult<Vec<BillboardItem>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";

        let filter = "(STATISTICSCYCLE='02')";
        let params = [
            ("reportName", "RPT_RATEDEPT_RETURNT_RANKING"),
            ("columns", "ALL"),
            ("filter", filter),
            ("pageNumber", "1"),
            ("pageSize", "500"),
            ("sortTypes", "-1"),
            ("sortColumns", "TOTAL_BUYER_SALESTIMES_1DAY"),
            ("source", "WEB"),
            ("client", "WEB"),
        ];

        debug!("Fetching Billboard list");

        #[derive(Deserialize)]
        struct DTResponse {
            result: Option<DTResult>,
        }

        #[derive(Deserialize)]
        struct DTResult {
            data: Option<Vec<DTItem>>,
        }

        #[derive(Deserialize)]
        struct DTItem {
            #[serde(rename = "OPERATEDEPT_CODE")]
            code: String,
            #[serde(rename = "OPERATEDEPT_NAME")]
            name: String,
            #[serde(rename = "TRADE_DATE", default)]
            trade_date: Option<String>,
            #[serde(rename = "TOTAL_BUYER_SALESTIMES_1DAY", default)]
            times_1day: Option<f64>,
            #[serde(rename = "TOTAL_BUYER_SALESTIMES_1DAY_BUY", default)]
            buy_amt: Option<f64>,
            #[serde(rename = "TOTAL_BUYER_SALESTIMES_1DAY_SELL", default)]
            sell_amt: Option<f64>,
            #[serde(rename = "TOTAL_BUYER_SALESTIMES_1DAY_NET", default)]
            net_amt: Option<f64>,
        }

        let response: DTResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.result.and_then(|r| r.data).unwrap_or_default();

        let today = chrono::Utc::now().date_naive();

        Ok(items
            .into_iter()
            .map(|item| {
                let date = item
                    .trade_date
                    .as_ref()
                    .and_then(|d| NaiveDate::parse_from_str(&d[..10], "%Y-%m-%d").ok())
                    .unwrap_or(today);
                BillboardItem {
                    stock_code: item.code,
                    stock_name: item.name,
                    trade_date: date,
                    close: 0.0,
                    change_pct: 0.0,
                    turnover_ratio: item.times_1day.unwrap_or(0.0),
                    net_buy_amount: item.net_amt.unwrap_or(0.0),
                    buy_amount: item.buy_amt.unwrap_or(0.0),
                    sell_amount: item.sell_amt.unwrap_or(0.0),
                    reason: format!("上榜次数: {}", item.times_1day.unwrap_or(0.0) as i64),
                }
            })
            .collect())
    }

    /// Fetches Billboard detail for a given stock and date.
    async fn get_billboard_detail(
        &self,
        stock_code: &str,
        date: &str,
    ) -> DataResult<Vec<BillboardDetail>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";
        let filter = format!("(TRADE_DATE='{}')(SECURITY_CODE=\"{}\")", date, stock_code);

        let buy_params = [
            ("reportName", "RPT_BILLBOARD_DAILYDETAILSBUY"),
            ("columns", "ALL"),
            ("filter", &filter),
            ("pageNumber", "1"),
            ("pageSize", "50"),
            ("sortTypes", "-1"),
            ("sortColumns", "BUY"),
            ("source", "WEB"),
            ("client", "WEB"),
        ];

        let sell_params = [
            ("reportName", "RPT_BILLBOARD_DAILYDETAILSSELL"),
            ("columns", "ALL"),
            ("filter", &filter),
            ("pageNumber", "1"),
            ("pageSize", "50"),
            ("sortTypes", "-1"),
            ("sortColumns", "SELL"),
            ("source", "WEB"),
            ("client", "WEB"),
        ];

        debug!(
            "Fetching Dragon Tiger Detail for {} on {}",
            stock_code, date
        );

        #[derive(Deserialize)]
        struct DetailResponse {
            result: Option<DetailResult>,
        }

        #[derive(Deserialize)]
        struct DetailResult {
            data: Option<Vec<DetailItem>>,
        }

        #[derive(Deserialize)]
        struct DetailItem {
            #[serde(rename = "SECURITY_CODE")]
            code: String,
            #[serde(rename = "TRADE_DATE")]
            trade_date: String,
            #[serde(rename = "OPERATEDEPT_NAME")]
            trader: String,
            #[serde(rename = "BUY", default)]
            buy: Option<f64>,
            #[serde(rename = "SELL", default)]
            sell: Option<f64>,
            #[serde(rename = "NET", default)]
            net: Option<f64>,
        }

        let mut results = Vec::new();

        let buy_response: DetailResponse =
            self.request.get_json_with_params(url, &buy_params).await?;
        if let Some(items) = buy_response.result.and_then(|r| r.data) {
            for item in items {
                let date = NaiveDate::parse_from_str(&item.trade_date[..10], "%Y-%m-%d")
                    .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
                results.push(BillboardDetail {
                    stock_code: item.code,
                    trade_date: date,
                    trader_name: item.trader,
                    buy_amount: item.buy.unwrap_or(0.0),
                    sell_amount: item.sell.unwrap_or(0.0),
                    net_amount: item.net.unwrap_or(0.0),
                    direction: "buy".to_string(),
                });
            }
        }

        let sell_response: DetailResponse =
            self.request.get_json_with_params(url, &sell_params).await?;
        if let Some(items) = sell_response.result.and_then(|r| r.data) {
            for item in items {
                let date = NaiveDate::parse_from_str(&item.trade_date[..10], "%Y-%m-%d")
                    .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
                results.push(BillboardDetail {
                    stock_code: item.code,
                    trade_date: date,
                    trader_name: item.trader,
                    buy_amount: item.buy.unwrap_or(0.0),
                    sell_amount: item.sell.unwrap_or(0.0),
                    net_amount: item.net.unwrap_or(0.0),
                    direction: "sell".to_string(),
                });
            }
        }

        Ok(results)
    }
}

#[async_trait]
impl EarningsForecastSource for EastMoneySource {
    /// Fetches earnings forecast for a given report period.
    async fn get_earnings_forecast(
        &self,
        report_period: Option<&str>,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<EarningsForecast>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";

        let filter = report_period
            .map(|p| format!("(REPORT_DATE='{}')", p))
            .unwrap_or_default();

        let params = [
            ("reportName", "RPT_PUBLIC_OP_NEWPREDICT"),
            (
                "columns",
                "SECURITY_CODE,SECURITY_NAME_ABBR,PREDICT_FINANCE_CODE,PREDICT_TYPE,PREDICT_AMT_LOWER,PREDICT_AMT_UPPER,ADD_AMP_LOWER,ADD_AMP_UPPER,REPORT_DATE,NOTICE_DATE,CHANGE_REASON_EXPLAIN",
            ),
            ("filter", &filter),
            ("pageNumber", &page.to_string()),
            ("pageSize", &limit.to_string()),
            ("sortTypes", "-1"),
            ("sortColumns", "NOTICE_DATE"),
            ("source", "WEB"),
            ("client", "WEB"),
        ];

        debug!("Fetching earnings forecast page {}", page);

        #[derive(Deserialize)]
        struct ForecastResponse {
            result: Option<ForecastResult>,
        }

        #[derive(Deserialize)]
        struct ForecastResult {
            data: Option<Vec<ForecastItem>>,
        }

        #[derive(Deserialize)]
        struct ForecastItem {
            #[serde(rename = "SECURITY_CODE")]
            code: String,
            #[serde(rename = "SECURITY_NAME_ABBR")]
            name: String,
            #[serde(rename = "PREDICT_TYPE", default)]
            predict_type: Option<String>,
            #[serde(rename = "PREDICT_AMT_LOWER", default)]
            profit_min: Option<f64>,
            #[serde(rename = "PREDICT_AMT_UPPER", default)]
            profit_max: Option<f64>,
            #[serde(rename = "ADD_AMP_LOWER", default)]
            change_min: Option<f64>,
            #[serde(rename = "ADD_AMP_UPPER", default)]
            change_max: Option<f64>,
            #[serde(rename = "REPORT_DATE", default)]
            report_date: Option<String>,
            #[serde(rename = "NOTICE_DATE")]
            notice_date: String,
            #[serde(rename = "CHANGE_REASON_EXPLAIN", default)]
            summary: Option<String>,
        }

        let response: ForecastResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.result.and_then(|r| r.data).unwrap_or_default();

        Ok(items
            .into_iter()
            .map(|item| {
                let announce_date = NaiveDate::parse_from_str(&item.notice_date[..10], "%Y-%m-%d")
                    .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
                EarningsForecast {
                    stock_code: item.code,
                    stock_name: item.name,
                    forecast_type: item.predict_type.unwrap_or_default(),
                    profit_min: item.profit_min,
                    profit_max: item.profit_max,
                    change_min: item.change_min,
                    change_max: item.change_max,
                    report_period: item.report_date.unwrap_or_default(),
                    announce_date,
                    summary: item.summary,
                }
            })
            .collect())
    }
}

#[async_trait]
impl StockConnectSource for EastMoneySource {
    /// Fetches Stock Connect data.
    async fn get_stock_connect(&self, limit: Option<usize>) -> DataResult<Vec<StockConnectData>> {
        let url = "https://push2his.eastmoney.com/api/qt/kamt.kline/get";
        let params = [
            ("fields1", "f1,f3,f5"),
            ("fields2", "f51,f52,f53,f54,f55,f56"),
            ("klt", "101"),
            ("lmt", &limit.unwrap_or(30).to_string()),
            ("ut", "b2884a393a59ad64002292a3e90d46a5"),
        ];

        debug!("Fetching Stock Connect data");

        #[derive(Deserialize)]
        struct ConnectResponse {
            data: Option<ConnectData>,
        }

        #[derive(Deserialize)]
        struct ConnectData {
            #[serde(default)]
            hk2sh: Vec<String>,
            #[serde(default)]
            hk2sz: Vec<String>,
        }

        let response: ConnectResponse = self.request.get_json_with_params(url, &params).await?;
        let data = match response.data {
            Some(d) => d,
            None => return Ok(Vec::new()),
        };

        let mut result = Vec::new();
        let sh_lines = &data.hk2sh;
        let sz_lines = &data.hk2sz;

        for i in 0..sh_lines.len().min(sz_lines.len()) {
            let sh_parts: Vec<&str> = sh_lines[i].split(',').collect();
            let sz_parts: Vec<&str> = sz_lines[i].split(',').collect();

            if sh_parts.len() < 4 || sz_parts.len() < 4 {
                continue;
            }

            let trade_date = NaiveDate::parse_from_str(sh_parts[0], "%Y-%m-%d")
                .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

            let sh_net: f64 = sh_parts[1].parse().unwrap_or(0.0);
            let sz_net: f64 = sz_parts[1].parse().unwrap_or(0.0);
            let sh_buy: f64 = sh_parts[2].parse().unwrap_or(0.0);
            let sh_sell: f64 = sh_parts[3].parse().unwrap_or(0.0);
            let sz_buy: f64 = sz_parts[2].parse().unwrap_or(0.0);
            let sz_sell: f64 = sz_parts[3].parse().unwrap_or(0.0);

            result.push(StockConnectData {
                trade_date,
                north_net_buy: sh_net + sz_net,
                sh_net_buy: sh_net,
                sz_net_buy: sz_net,
                north_buy: sh_buy + sz_buy,
                north_sell: sh_sell + sz_sell,
            });
        }

        Ok(result)
    }
}

#[async_trait]
impl MarginTradingSource for EastMoneySource {
    /// Fetches margin trading data for a stock.
    async fn get_margin_trading(
        &self,
        stock_code: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<MarginTradingData>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";

        let params = [
            ("reportName", "RPTA_WEB_RZRQ_GGMX"),
            (
                "columns",
                "DATE,SCODE,SECNAME,RZYE,RZMRE,RZCHE,RQYE,RQYL,RZRQYE",
            ),
            ("filter", &format!("(SCODE=\"{}\")", stock_code)),
            ("pageNumber", "1"),
            ("pageSize", &limit.unwrap_or(30).to_string()),
            ("source", "WEB"),
            ("client", "WEB"),
        ];

        debug!("Fetching margin trading data for {}", stock_code);

        #[derive(Deserialize)]
        struct MarginResponse {
            result: Option<MarginResult>,
        }

        #[derive(Deserialize)]
        struct MarginResult {
            data: Option<Vec<MarginItem>>,
        }

        #[derive(Deserialize)]
        struct MarginItem {
            #[serde(rename = "SCODE")]
            code: String,
            #[serde(rename = "SECNAME")]
            name: String,
            #[serde(rename = "DATE")]
            trade_date: String,
            #[serde(rename = "RZYE", default)]
            margin_balance: Option<f64>,
            #[serde(rename = "RZMRE", default)]
            margin_buy: Option<f64>,
            #[serde(rename = "RZCHE", default)]
            margin_repay: Option<f64>,
            #[serde(rename = "RQYE", default)]
            short_balance: Option<f64>,
            #[serde(rename = "RQYL", default)]
            short_volume: Option<f64>,
            #[serde(rename = "RZRQYE", default)]
            total_balance: Option<f64>,
        }

        let response: MarginResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.result.and_then(|r| r.data).unwrap_or_default();

        Ok(items
            .into_iter()
            .map(|item| {
                let trade_date = NaiveDate::parse_from_str(&item.trade_date[..10], "%Y-%m-%d")
                    .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
                MarginTradingData {
                    stock_code: item.code,
                    stock_name: item.name,
                    trade_date,
                    margin_balance: item.margin_balance.unwrap_or(0.0),
                    margin_buy: item.margin_buy.unwrap_or(0.0),
                    margin_repay: item.margin_repay.unwrap_or(0.0),
                    short_balance: item.short_balance.unwrap_or(0.0),
                    short_volume: item.short_volume.unwrap_or(0.0) as u64,
                    total_balance: item.total_balance.unwrap_or(0.0),
                }
            })
            .collect())
    }
}

#[async_trait]
impl IPOSource for EastMoneySource {
    /// Fetches IPO list.
    async fn get_ipo_list(&self, limit: Option<usize>) -> DataResult<Vec<IPOData>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";
        let params = [
            ("reportName", "RPTA_APP_IPOAPPLY"),
            (
                "columns",
                "SECURITY_CODE,SECURITY_NAME,ISSUE_PRICE,APPLY_DATE,LISTING_DATE,ONLINE_ISSUE_LWR,TOTAL_ISSUE_NUM,ONLINE_ISSUE_NUM,INDUSTRY_PE_NEW",
            ),
            ("pageNumber", "1"),
            ("pageSize", &limit.unwrap_or(50).to_string()),
            ("sortTypes", "-1"),
            ("sortColumns", "APPLY_DATE"),
            ("source", "WEB"),
            ("client", "WEB"),
        ];

        debug!("Fetching IPO list");

        #[derive(Deserialize)]
        struct IPOResponse {
            result: Option<IPOResult>,
        }

        #[derive(Deserialize)]
        struct IPOResult {
            data: Option<Vec<IPOItem>>,
        }

        #[derive(Deserialize)]
        struct IPOItem {
            #[serde(rename = "SECURITY_CODE")]
            code: String,
            #[serde(rename = "SECURITY_NAME")]
            name: String,
            #[serde(rename = "ISSUE_PRICE", default)]
            issue_price: Option<f64>,
            #[serde(rename = "APPLY_DATE")]
            apply_date: String,
            #[serde(rename = "LISTING_DATE", default)]
            list_date: Option<String>,
            #[serde(rename = "ONLINE_ISSUE_LWR", default)]
            winning_rate: Option<f64>,
            #[serde(rename = "TOTAL_ISSUE_NUM", default)]
            issue_quantity: Option<f64>,
            #[serde(rename = "ONLINE_ISSUE_NUM", default)]
            online_quantity: Option<f64>,
            #[serde(rename = "INDUSTRY_PE_NEW", default)]
            pe_ratio: Option<f64>,
        }

        let response: IPOResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.result.and_then(|r| r.data).unwrap_or_default();

        Ok(items
            .into_iter()
            .map(|item| {
                let sub_date = NaiveDate::parse_from_str(&item.apply_date[..10], "%Y-%m-%d")
                    .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
                let list_date = item
                    .list_date
                    .as_ref()
                    .and_then(|d| NaiveDate::parse_from_str(&d[..10], "%Y-%m-%d").ok());
                IPOData {
                    stock_code: item.code,
                    stock_name: item.name,
                    issue_price: item.issue_price.unwrap_or(0.0),
                    sub_date,
                    list_date,
                    winning_rate: item.winning_rate,
                    issue_quantity: item.issue_quantity.map(|v| (v * 10000.0) as u64),
                    online_quantity: item.online_quantity.map(|v| v as u64),
                    pe_ratio: item.pe_ratio,
                }
            })
            .collect())
    }
}

#[async_trait]
impl BlockTradeSource for EastMoneySource {
    /// Fetches block trade list with buyer/seller info.
    async fn get_block_trades(&self, limit: Option<usize>) -> DataResult<Vec<BlockTradeData>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";
        let page_size = limit.unwrap_or(50).to_string();
        let params = [
            ("reportName", "RPT_DATA_BLOCKTRADE"),
            (
                "columns",
                "SECUCODE,SECURITY_NAME_ABBR,TRADE_DATE,CLOSE_PRICE,DEAL_PRICE,PREMIUM_RATIO,DEAL_AMT,BUYER_NAME,SELLER_NAME",
            ),
            ("pageNumber", "1"),
            ("pageSize", &page_size),
            ("sortTypes", "-1"),
            ("sortColumns", "TRADE_DATE"),
        ];

        debug!("Fetching block trade list");

        #[derive(Deserialize)]
        struct BlockResponse {
            result: Option<BlockResult>,
        }

        #[derive(Deserialize)]
        struct BlockResult {
            data: Option<Vec<BlockItem>>,
        }

        #[derive(Deserialize)]
        struct BlockItem {
            #[serde(rename = "SECUCODE")]
            secucode: String,
            #[serde(rename = "SECURITY_NAME_ABBR")]
            name: String,
            #[serde(rename = "TRADE_DATE")]
            trade_date: String,
            #[serde(rename = "DEAL_PRICE", default)]
            price: Option<f64>,
            #[serde(rename = "CLOSE_PRICE", default)]
            close_price: Option<f64>,
            #[serde(rename = "PREMIUM_RATIO", default)]
            premium_rate: Option<f64>,
            #[serde(rename = "DEAL_AMT", default)]
            amount: Option<f64>,
            #[serde(rename = "BUYER_NAME", default)]
            buyer: Option<String>,
            #[serde(rename = "SELLER_NAME", default)]
            seller: Option<String>,
        }

        let response: BlockResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.result.and_then(|r| r.data).unwrap_or_default();

        Ok(items
            .into_iter()
            .map(|item| {
                let trade_date = NaiveDate::parse_from_str(&item.trade_date[..10], "%Y-%m-%d")
                    .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
                let stock_code = item
                    .secucode
                    .split('.')
                    .next()
                    .unwrap_or(&item.secucode)
                    .to_string();
                BlockTradeData {
                    stock_code,
                    stock_name: item.name,
                    trade_date,
                    price: item.price.unwrap_or(0.0),
                    close_price: item.close_price.unwrap_or(0.0),
                    premium_rate: item.premium_rate.unwrap_or(0.0) * 100.0,
                    volume: 0,
                    amount: item.amount.unwrap_or(0.0),
                    buyer: item.buyer.unwrap_or_default(),
                    seller: item.seller.unwrap_or_default(),
                }
            })
            .collect())
    }
}

#[async_trait]
impl InstitutionalResearchSource for EastMoneySource {
    /// Fetches institutional research list.
    async fn get_institutional_research(
        &self,
        limit: Option<usize>,
    ) -> DataResult<Vec<InstitutionalResearchData>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";
        let params = [
            ("reportName", "RPT_ORG_SURVEYNEW"),
            (
                "columns",
                "SECUCODE,SECURITY_CODE,SECURITY_NAME_ABBR,NOTICE_DATE,NUMBERNEW,RECEIVE_OBJECT,RECEIVE_WAY_EXPLAIN,RECEPTIONIST",
            ),
            ("pageNumber", "1"),
            ("pageSize", &limit.unwrap_or(50).to_string()),
            ("sortTypes", "-1"),
            ("sortColumns", "NOTICE_DATE"),
            ("source", "WEB"),
            ("client", "WEB"),
        ];

        debug!("Fetching institutional research list");

        #[derive(Deserialize)]
        struct ResearchResponse {
            result: Option<ResearchResult>,
        }

        #[derive(Deserialize)]
        struct ResearchResult {
            data: Option<Vec<ResearchItem>>,
        }

        #[derive(Deserialize)]
        struct ResearchItem {
            #[serde(rename = "SECURITY_CODE")]
            code: String,
            #[serde(rename = "SECURITY_NAME_ABBR")]
            name: String,
            #[serde(rename = "NOTICE_DATE")]
            notice_date: String,
            #[serde(rename = "NUMBERNEW", default)]
            org_num: Option<String>,
            #[serde(rename = "RECEIVE_OBJECT", default)]
            org_name: Option<String>,
            #[serde(rename = "RECEIVE_WAY_EXPLAIN", default)]
            receive_way: Option<String>,
            #[serde(rename = "RECEPTIONIST", default)]
            receptionist: Option<String>,
        }

        let response: ResearchResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.result.and_then(|r| r.data).unwrap_or_default();

        Ok(items
            .into_iter()
            .map(|item| {
                let research_date = NaiveDate::parse_from_str(&item.notice_date[..10], "%Y-%m-%d")
                    .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
                InstitutionalResearchData {
                    stock_code: item.code,
                    stock_name: item.name,
                    research_date,
                    institution_count: item.org_num.and_then(|s| s.parse().ok()).unwrap_or(0),
                    institutions: item.org_name.unwrap_or_default(),
                    research_type: item.receive_way.unwrap_or_default(),
                    researchers: item.receptionist,
                }
            })
            .collect())
    }
}

#[async_trait]
impl ResearchReportSource for EastMoneySource {
    /// Fetches research reports.
    async fn get_research_reports(
        &self,
        stock_code: Option<&str>,
        limit: Option<usize>,
    ) -> DataResult<Vec<ResearchReportData>> {
        let url = "https://reportapi.eastmoney.com/report/list";
        let code = stock_code.unwrap_or("*");
        let params = [
            ("industryCode", code),
            ("pageNo", "1"),
            ("pageSize", &limit.unwrap_or(50).to_string()),
            ("qType", "0"),
            ("beginTime", "2020-01-01"),
            ("endTime", "2030-12-31"),
            ("sortColumn", "publishDate"),
            ("sortType", "-1"),
        ];

        debug!("Fetching research reports");

        #[derive(Deserialize)]
        struct ReportResponse {
            data: Option<Vec<ReportItem>>,
        }

        #[derive(Deserialize)]
        struct ReportItem {
            #[serde(rename = "infoCode")]
            info_code: String,
            #[serde(rename = "stockCode", default)]
            stock_code: Option<String>,
            #[serde(rename = "stockName", default)]
            stock_name: Option<String>,
            #[serde(default)]
            title: String,
            #[serde(rename = "orgSName", default)]
            org_name: Option<String>,
            #[serde(rename = "researcher", default)]
            researcher: Option<String>,
            #[serde(rename = "emRatingName", default)]
            rating: Option<String>,
            #[serde(rename = "publishDate")]
            publish_date: String,
        }

        let response: ReportResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.data.unwrap_or_default();

        Ok(items
            .into_iter()
            .map(|item| {
                let publish_date = NaiveDate::parse_from_str(&item.publish_date[..10], "%Y-%m-%d")
                    .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
                ResearchReportData {
                    report_id: item.info_code,
                    stock_code: item.stock_code.unwrap_or_default(),
                    stock_name: item.stock_name.unwrap_or_default(),
                    title: item.title,
                    institution: item.org_name.unwrap_or_default(),
                    analysts: item.researcher.unwrap_or_default(),
                    rating: item.rating,
                    publish_date,
                }
            })
            .collect())
    }
}

#[async_trait]
impl ValuationSource for EastMoneySource {
    async fn get_valuation(&self, stock_code: &str) -> DataResult<StockValuation> {
        let secid = Self::get_secid(stock_code);

        let url = "https://push2.eastmoney.com/api/qt/stock/get";
        let params = [
            ("secid", secid.as_str()),
            (
                "fields",
                "f43,f57,f58,f116,f117,f162,f163,f167,f168,f164,f165,f166,f173,f183,f184,f185,f186,f187",
            ),
            ("ut", "b2884a393a59ad64002292a3e90d46a5"),
        ];

        debug!("Fetching valuation for {}", stock_code);

        #[derive(Deserialize)]
        struct ValResponse {
            data: Option<ValData>,
        }

        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct ValData {
            f43: Option<f64>,
            f57: Option<String>,
            f58: Option<String>,
            f116: Option<f64>,
            f117: Option<f64>,
            f162: Option<f64>,
            f163: Option<f64>,
            f167: Option<f64>,
            f168: Option<f64>,
            f164: Option<f64>,
            f165: Option<f64>,
            f166: Option<f64>,
            f173: Option<f64>,
            f183: Option<f64>,
            f184: Option<f64>,
            f185: Option<f64>,
            f186: Option<f64>,
            f187: Option<f64>,
        }

        let response: ValResponse = self.request.get_json_with_params(url, &params).await?;
        let data = response
            .data
            .ok_or_else(|| crate::error::DataError::NoDataAvailable)?;

        Ok(StockValuation {
            stock_code: data.f57.unwrap_or_else(|| stock_code.to_string()),
            stock_name: data.f58.unwrap_or_default(),
            price: data.f43.map(|p| p / 100.0).unwrap_or(0.0),
            market_cap: data.f116.unwrap_or(0.0),
            float_cap: data.f117.unwrap_or(0.0),
            pe_ttm: data.f162.map(|p| p / 100.0),
            pe_static: data.f163.map(|p| p / 100.0),
            pb: data.f167.map(|p| p / 100.0),
            ps: data.f168.map(|p| p / 100.0),
            eps: data.f173,
            bps: data.f187,
            roe: data.f164.map(|p| p / 100.0),
            gross_margin: data.f165.map(|p| p / 100.0),
            net_margin: data.f166.map(|p| p / 100.0),
            revenue: data.f183,
            net_profit: data.f184,
            revenue_yoy: data.f185,
            profit_yoy: data.f186,
        })
    }
}

#[async_trait]
impl HoldingsSource for EastMoneySource {
    async fn get_top_holders(&self, stock_code: &str) -> DataResult<Vec<TopHolder>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";
        let secucode = format!(
            "{}.{}",
            stock_code,
            if stock_code.starts_with('6') {
                "SH"
            } else {
                "SZ"
            }
        );
        let filter = format!("(SECUCODE=\"{}\")", secucode);
        let params = [
            ("reportName", "RPT_DMSK_HOLDERS"),
            (
                "columns",
                "SECUCODE,END_DATE,HOLDER_NAME,HOLD_NUM,HOLD_RATIO",
            ),
            ("filter", &filter),
            ("pageNumber", "1"),
            ("pageSize", "10"),
            ("sortColumns", "END_DATE,HOLD_RATIO"),
            ("sortTypes", "-1,-1"),
        ];

        debug!("Fetching top holders for {}", stock_code);

        #[derive(Deserialize)]
        struct HolderResponse {
            result: Option<HolderResult>,
        }

        #[derive(Deserialize)]
        struct HolderResult {
            data: Option<Vec<HolderItem>>,
        }

        #[derive(Deserialize)]
        struct HolderItem {
            #[serde(rename = "END_DATE")]
            end_date: Option<String>,
            #[serde(rename = "HOLDER_NAME")]
            holder_name: Option<String>,
            #[serde(rename = "HOLD_NUM")]
            hold_num: Option<f64>,
            #[serde(rename = "HOLD_RATIO")]
            hold_ratio: Option<f64>,
        }

        let response: HolderResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.result.and_then(|r| r.data).unwrap_or_default();

        Ok(items
            .into_iter()
            .enumerate()
            .map(|(i, item)| {
                let report_date = item
                    .end_date
                    .as_ref()
                    .and_then(|d| NaiveDate::parse_from_str(&d[..10], "%Y-%m-%d").ok())
                    .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
                TopHolder {
                    stock_code: stock_code.to_string(),
                    report_date,
                    rank: (i + 1) as u32,
                    holder_name: item.holder_name.unwrap_or_default(),
                    hold_quantity: item.hold_num.unwrap_or(0.0) as u64,
                    hold_ratio: item.hold_ratio.unwrap_or(0.0),
                    change_quantity: None,
                    holder_type: String::new(),
                }
            })
            .collect())
    }

    async fn get_fund_holdings(
        &self,
        stock_code: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<FundHolding>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";
        let secucode = format!(
            "{}.{}",
            stock_code,
            if stock_code.starts_with('6') {
                "SH"
            } else {
                "SZ"
            }
        );
        let page_size = limit.unwrap_or(20).to_string();
        let filter = format!("(SECUCODE=\"{}\")(HOLDER_TYPE=\"证券投资基金\")", secucode);
        let params = [
            ("reportName", "RPT_F10_EH_FREEHOLDERS"),
            (
                "columns",
                "SECUCODE,SECURITY_NAME_ABBR,END_DATE,HOLDER_NAME,HOLD_NUM,HOLD_RATIO",
            ),
            ("filter", &filter),
            ("pageNumber", "1"),
            ("pageSize", &page_size),
            ("sortColumns", "END_DATE,HOLD_NUM"),
            ("sortTypes", "-1,-1"),
        ];

        debug!("Fetching fund holdings for {}", stock_code);

        #[derive(Deserialize)]
        struct FundResponse {
            result: Option<FundResult>,
        }

        #[derive(Deserialize)]
        struct FundResult {
            data: Option<Vec<FundItem>>,
        }

        #[derive(Deserialize)]
        struct FundItem {
            #[serde(rename = "SECURITY_NAME_ABBR")]
            stock_name: Option<String>,
            #[serde(rename = "END_DATE")]
            end_date: Option<String>,
            #[serde(rename = "HOLDER_NAME")]
            holder_name: Option<String>,
            #[serde(rename = "HOLD_NUM")]
            hold_num: Option<f64>,
            #[serde(rename = "HOLD_RATIO")]
            hold_ratio: Option<f64>,
        }

        let response: FundResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.result.and_then(|r| r.data).unwrap_or_default();

        Ok(items
            .into_iter()
            .map(|item| {
                let report_date = item
                    .end_date
                    .as_ref()
                    .and_then(|d| NaiveDate::parse_from_str(&d[..10], "%Y-%m-%d").ok())
                    .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
                FundHolding {
                    stock_code: stock_code.to_string(),
                    stock_name: item.stock_name.unwrap_or_default(),
                    report_date,
                    fund_name: item.holder_name.unwrap_or_default(),
                    hold_shares: item.hold_num.unwrap_or(0.0) as u64,
                    hold_ratio: item.hold_ratio.unwrap_or(0.0),
                }
            })
            .collect())
    }
}

#[async_trait]
impl DividendSource for EastMoneySource {
    async fn get_dividends(&self, stock_code: &str) -> DataResult<Vec<DividendData>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";
        let secucode = format!(
            "{}.{}",
            stock_code,
            if stock_code.starts_with('6') {
                "SH"
            } else {
                "SZ"
            }
        );
        let params = [
            ("reportName", "RPT_SHAREBONUS_DET"),
            (
                "columns",
                "SECUCODE,SECURITY_NAME_ABBR,REPORT_DATE,EX_DIVIDEND_DATE,EQUITY_RECORD_DATE,PRETAX_BONUS_RMB,BONUS_RATIO,IT_RATIO",
            ),
            ("filter", &format!("(SECUCODE=\"{}\")", secucode)),
            ("pageNumber", "1"),
            ("pageSize", "50"),
            ("sortColumns", "REPORT_DATE"),
            ("sortTypes", "-1"),
        ];

        debug!("Fetching dividends for {}", stock_code);

        #[derive(Deserialize)]
        struct DivResponse {
            result: Option<DivResult>,
        }

        #[derive(Deserialize)]
        struct DivResult {
            data: Option<Vec<DivItem>>,
        }

        #[derive(Deserialize)]
        struct DivItem {
            #[serde(rename = "SECURITY_NAME_ABBR")]
            stock_name: Option<String>,
            #[serde(rename = "REPORT_DATE")]
            report_date: Option<String>,
            #[serde(rename = "EX_DIVIDEND_DATE")]
            ex_date: Option<String>,
            #[serde(rename = "EQUITY_RECORD_DATE")]
            record_date: Option<String>,
            #[serde(rename = "PRETAX_BONUS_RMB")]
            cash_div: Option<f64>,
            #[serde(rename = "BONUS_RATIO")]
            bonus: Option<f64>,
            #[serde(rename = "IT_RATIO")]
            transfer: Option<f64>,
        }

        let response: DivResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.result.and_then(|r| r.data).unwrap_or_default();

        Ok(items
            .into_iter()
            .map(|item| {
                let report_date = item
                    .report_date
                    .as_ref()
                    .and_then(|d| NaiveDate::parse_from_str(&d[..10], "%Y-%m-%d").ok())
                    .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
                let ex_date = item
                    .ex_date
                    .as_ref()
                    .and_then(|d| NaiveDate::parse_from_str(&d[..10], "%Y-%m-%d").ok());
                let record_date = item
                    .record_date
                    .as_ref()
                    .and_then(|d| NaiveDate::parse_from_str(&d[..10], "%Y-%m-%d").ok());
                DividendData {
                    stock_code: stock_code.to_string(),
                    stock_name: item.stock_name.unwrap_or_default(),
                    report_date,
                    ex_date,
                    record_date,
                    dividend_per_share: item.cash_div.unwrap_or(0.0),
                    bonus_shares: item.bonus.unwrap_or(0.0),
                    transfer_shares: item.transfer.unwrap_or(0.0),
                    dividend_yield: None,
                }
            })
            .collect())
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
