use async_trait::async_trait;
use chrono::{NaiveDate, TimeZone, Utc};
use serde::Deserialize;
use tracing::debug;

use super::EastMoneySource;
use crate::data::{
    DerivativesExchange, DerivativesQuote, FinancialRecord, FinancialReportKind, FuturesContract,
    KLineType, MarketData, OptionContract, OptionExchange,
};
use crate::error::DataResult;
use crate::util::parse_trade_date;
use crate::traits::{FinancialSource, FuturesSource, OptionsSource};

const FUTURES_FS: &str = "m:113,m:114,m:115,m:8,m:142,m:225";

impl EastMoneySource {
    async fn fetch_derivatives_clist(
        &self,
        fs: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<(String, String, String, i64)>> {
        let url = "https://push2.eastmoney.com/api/qt/clist/get";
        let mut items = Vec::new();
        let page_size = 100;
        let mut page = 1u32;

        loop {
            let params = [
                ("pn", page.to_string()),
                ("pz", page_size.to_string()),
                ("po", "1".to_string()),
                ("np", "1".to_string()),
                ("fltt", "2".to_string()),
                ("invt", "2".to_string()),
                ("fid", "f3".to_string()),
                ("fs", fs.to_string()),
                ("fields", "f12,f14,f13".to_string()),
            ];

            #[derive(Deserialize)]
            struct Resp {
                data: Option<RespData>,
            }
            #[derive(Deserialize)]
            struct RespData {
                diff: Option<Vec<Row>>,
            }
            #[derive(Deserialize)]
            struct Row {
                #[serde(rename = "f12")]
                code: String,
                #[serde(rename = "f14")]
                name: String,
                #[serde(rename = "f13", default)]
                market: Option<i64>,
            }

            let response: Resp = self.request.get_json_with_params(url, &params).await?;
            let rows = response.data.and_then(|d| d.diff).unwrap_or_default();
            if rows.is_empty() {
                break;
            }
            let count = rows.len();
            for row in rows {
                let market = row.market.unwrap_or(113);
                let secid = format!("{market}.{}", row.code);
                items.push((row.code, row.name, secid, market));
                if let Some(lim) = limit {
                    if items.len() >= lim {
                        return Ok(items);
                    }
                }
            }
            if count < page_size {
                break;
            }
            page += 1;
        }
        Ok(items)
    }

    async fn fetch_derivatives_quotes(&self, secids: &[String]) -> DataResult<Vec<DerivativesQuote>> {
        let mut results = Vec::with_capacity(secids.len());
        for secid in secids {
            let params = [
                ("secid", secid.as_str()),
                ("fields", "f43,f44,f45,f46,f47,f48,f57,f58,f60,f169,f170,f109"),
                ("ut", "fa5fd1943c7b386f172d6893dbfba10b"),
            ];
            let url = "https://push2.eastmoney.com/api/qt/stock/get";
            #[derive(Deserialize)]
            struct Resp {
                data: Option<Row>,
            }
            #[derive(Deserialize)]
            struct Row {
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
                #[serde(rename = "f109", default)]
                open_interest: Option<f64>,
            }
            let response: Resp = self.request.get_json_with_params(url, &params).await?;
            if let Some(data) = response.data {
                let scale = if secid.starts_with("10.") || secid.starts_with("11.") || secid.starts_with("12.") {
                    10000.0
                } else {
                    100.0
                };
                results.push(DerivativesQuote {
                    contract_code: data.code,
                    contract_name: data.name,
                    secid: secid.clone(),
                    price: data.price.map(|v| v as f64 / scale).unwrap_or(0.0),
                    open: data.open.map(|v| v as f64 / scale),
                    high: data.high.map(|v| v as f64 / scale),
                    low: data.low.map(|v| v as f64 / scale),
                    pre_close: data.pre_close.map(|v| v as f64 / scale),
                    change: data.change.map(|v| v as f64 / scale).unwrap_or(0.0),
                    change_pct: data.change_pct.map(|v| v as f64 / 100.0).unwrap_or(0.0),
                    volume: data.volume.unwrap_or(0),
                    amount: data.amount.unwrap_or(0.0),
                    open_interest: data.open_interest.map(|v| v as u64),
                    trade_date: None,
                });
            }
        }
        Ok(results)
    }

    async fn fetch_derivatives_kline(
        &self,
        secid: &str,
        label_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        let klt = k_type.to_api_value();
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
            ("fqt", "0"),
            ("secid", secid),
            ("beg", &start),
            ("end", &end),
        ];
        let url = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
        #[derive(Deserialize)]
        struct KResp {
            data: Option<KData>,
        }
        #[derive(Deserialize)]
        struct KData {
            klines: Option<Vec<String>>,
        }
        let response: KResp = self.request.get_json_with_params(url, &params).await?;
        let klines = response.data.and_then(|d| d.klines).unwrap_or_default();
        let mut result = Vec::with_capacity(klines.len());
        for line in klines {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 11 {
                continue;
            }
            let trade_date = parse_trade_date(parts[0]);
            let open: f64 = parts[1].parse().unwrap_or(0.0);
            let close: f64 = parts[2].parse().unwrap_or(0.0);
            let high: f64 = parts[3].parse().unwrap_or(0.0);
            let low: f64 = parts[4].parse().unwrap_or(0.0);
            let volume: u64 = parts[5].parse::<f64>().unwrap_or(0.0) as u64;
            let amount: f64 = parts[6].parse().unwrap_or(0.0);
            let change_pct: f64 = parts[8].parse().unwrap_or(0.0);
            let change: f64 = parts[9].parse().unwrap_or(0.0);
            let turnover_ratio: f64 = parts[10].parse().unwrap_or(0.0);
            result.push(MarketData {
                stock_code: label_code.to_string(),
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
                pre_close: close - change,
            });
        }
        Ok(result)
    }
}

#[async_trait]
impl FuturesSource for EastMoneySource {
    async fn get_futures_list(&self, limit: Option<usize>) -> DataResult<Vec<FuturesContract>> {
        debug!("Fetching futures list from EastMoney");
        let rows = self.fetch_derivatives_clist(FUTURES_FS, limit).await?;
        Ok(rows
            .into_iter()
            .map(|(code, name, secid, market)| FuturesContract {
                contract_code: code,
                contract_name: name,
                secid,
                exchange: DerivativesExchange::from_market_id(market),
            })
            .collect())
    }

    async fn get_futures_current(&self, secids: &[&str]) -> DataResult<Vec<DerivativesQuote>> {
        if secids.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<String> = secids.iter().map(|s| s.to_string()).collect();
        self.fetch_derivatives_quotes(&ids).await
    }

    async fn get_futures_market(
        &self,
        secid: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        let label = secid.split('.').nth(1).unwrap_or(secid);
        self.fetch_derivatives_kline(secid, label, start_date, end_date, k_type)
            .await
    }
}

#[async_trait]
impl OptionsSource for EastMoneySource {
    async fn get_options_list(
        &self,
        exchange: OptionExchange,
        limit: Option<usize>,
    ) -> DataResult<Vec<OptionContract>> {
        let url = "https://31.push2.eastmoney.com/api/qt/clist/get";
        let page_size = limit.unwrap_or(500).min(500);
        let params = [
            ("pn", "1".to_string()),
            ("pz", page_size.to_string()),
            ("po", "1".to_string()),
            ("np", "1".to_string()),
            ("fltt", "2".to_string()),
            ("invt", "2".to_string()),
            ("fid", "f3".to_string()),
            ("fs", exchange.eastmoney_fs().to_string()),
            ("fields", "f12,f14,f2,f3".to_string()),
        ];
        #[derive(Deserialize)]
        struct Resp {
            data: Option<RespData>,
        }
        #[derive(Deserialize)]
        struct RespData {
            diff: Option<Vec<Row>>,
        }
        #[derive(Deserialize)]
        struct Row {
            #[serde(rename = "f12")]
            code: String,
            #[serde(rename = "f14")]
            name: String,
            #[serde(rename = "f2", default)]
            price: Option<f64>,
            #[serde(rename = "f3", default)]
            change_pct: Option<f64>,
        }
        let response: Resp = self.request.get_json_with_params(url, &params).await?;
        let rows = response.data.and_then(|d| d.diff).unwrap_or_default();
        Ok(rows
            .into_iter()
            .map(|row| OptionContract {
                contract_code: row.code,
                contract_name: row.name,
                exchange,
                price: row.price.unwrap_or(0.0),
                change_pct: row.change_pct.unwrap_or(0.0),
            })
            .collect())
    }

    async fn get_options_current(&self, contract_codes: &[&str]) -> DataResult<Vec<DerivativesQuote>> {
        if contract_codes.is_empty() {
            return Ok(Vec::new());
        }
        let secids: Vec<String> = contract_codes
            .iter()
            .map(|code| {
                if code.contains('.') {
                    code.to_string()
                } else if code.starts_with("IO") || code.contains('-') {
                    format!("11.{code}")
                } else if code.starts_with('9') {
                    format!("12.{code}")
                } else {
                    format!("10.{code}")
                }
            })
            .collect();
        self.fetch_derivatives_quotes(&secids).await
    }
}

#[async_trait]
impl FinancialSource for EastMoneySource {
    async fn get_financial_statement(
        &self,
        stock_code: &str,
        kind: FinancialReportKind,
        limit: Option<usize>,
    ) -> DataResult<Vec<FinancialRecord>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";
        let page_size = limit.unwrap_or(20).min(500);
        let filter = format!("(SECURITY_CODE=\"{stock_code}\")");
        let params = [
            ("reportName", kind.eastmoney_report_name()),
            ("columns", "ALL"),
            ("filter", filter.as_str()),
            ("pageNumber", "1"),
            ("pageSize", &page_size.to_string()),
            ("sortColumns", kind.sort_column()),
            ("sortTypes", "-1"),
            ("source", "WEB"),
            ("client", "WEB"),
        ];
        #[derive(Deserialize)]
        struct Resp {
            result: Option<RespResult>,
        }
        #[derive(Deserialize)]
        struct RespResult {
            data: Option<Vec<serde_json::Value>>,
        }
        let response: Resp = self.request.get_json_with_params(url, &params).await?;
        let rows = response.result.and_then(|r| r.data).unwrap_or_default();
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let stock_code = row.get("SECURITY_CODE")?.as_str()?.to_string();
                let stock_name = row
                    .get("SECURITY_NAME_ABBR")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let date_key = if kind == FinancialReportKind::PerformanceSummary {
                    "REPORTDATE"
                } else {
                    "REPORT_DATE"
                };
                let report_date = row
                    .get(date_key)
                    .and_then(|v| v.as_str())
                    .map(|s| parse_trade_date(&s[..10.min(s.len())]))
                    .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
                let mut values = Vec::new();
                if let Some(obj) = row.as_object() {
                    for (key, value) in obj {
                        if key == "SECURITY_CODE"
                            || key == "SECURITY_NAME_ABBR"
                            || key == "REPORT_DATE"
                            || key == "REPORTDATE"
                        {
                            continue;
                        }
                        if let Some(num) = value.as_f64() {
                            values.push((key.clone(), num));
                        }
                    }
                }
                Some(FinancialRecord {
                    stock_code,
                    stock_name,
                    report_date,
                    kind,
                    values,
                })
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_futures_fs_constant() {
        assert!(FUTURES_FS.contains("113"));
        assert!(FUTURES_FS.contains("225"));
    }

    #[test]
    fn test_option_exchange_fs() {
        assert_eq!(OptionExchange::Cffex.eastmoney_fs(), "m:11");
    }
}
