use super::*;
use async_trait::async_trait;
use chrono::NaiveDate;
use serde::Deserialize;
use tracing::{debug, warn};

use crate::data::{
    BondCurrentData, ConvertibleBondCode, Exchange,
};
use crate::error::DataResult;
use crate::traits::{
    BondInfoSource, BondMarketSource,
};

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
            let secid = Exchange::infer_eastmoney_secid(code);
            let params = [
                ("secid", secid.as_str()),
                ("fields", "f43,f44,f45,f46,f47,f48,f57,f58,f60,f169,f170"),
            ];

            let url = "https://push2delay.eastmoney.com/api/qt/stock/get";
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
        let url = "https://push2delay.eastmoney.com/api/qt/clist/get";
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

