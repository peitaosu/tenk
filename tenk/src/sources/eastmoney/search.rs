use serde::Deserialize;
use tracing::debug;

use crate::data::{Exchange, StockSearchHit};
use crate::error::DataResult;

use super::EastMoneySource;

const SEARCH_TOKEN: &str = "D43BF722C8E33BDC906FB84D85E326E8";

#[derive(Debug, Deserialize)]
struct SuggestResponse {
    #[serde(rename = "QuotationCodeTable")]
    table: Option<SuggestTable>,
}

#[derive(Debug, Deserialize)]
struct SuggestTable {
    #[serde(default, rename = "Data")]
    data: Option<Vec<SuggestItem>>,
}

#[derive(Debug, Deserialize)]
struct SuggestItem {
    #[serde(rename = "Code", default)]
    code: String,
    #[serde(rename = "Name", default)]
    name: String,
    #[serde(rename = "SecurityTypeName", default)]
    security_type_name: String,
    #[serde(rename = "Classify", default)]
    classify: String,
}

impl EastMoneySource {
    pub(crate) async fn fetch_stock_search(&self, keyword: &str, limit: usize) -> DataResult<Vec<StockSearchHit>> {
        let keyword = keyword.trim();
        if keyword.is_empty() {
            return Ok(Vec::new());
        }

        let count = limit.max(1).min(50).to_string();
        let params = [
            ("input", keyword),
            ("type", "14"),
            ("token", SEARCH_TOKEN),
            ("count", count.as_str()),
        ];

        let url = "https://searchapi.eastmoney.com/api/suggest/get";
        debug!("Searching stocks from East Money: {}", keyword);

        let response: SuggestResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response
            .table
            .and_then(|table| table.data)
            .unwrap_or_default();

        let mut result = Vec::with_capacity(items.len());
        for item in items {
            if !is_watchlist_search_hit(&item) {
                continue;
            }
            let exchange = exchange_for_search_hit(&item);
            let market = if item.security_type_name.is_empty() {
                exchange.to_string()
            } else {
                item.security_type_name
            };
            result.push(StockSearchHit {
                stock_code: item.code,
                short_name: item.name,
                exchange,
                market,
            });
            if result.len() >= limit {
                break;
            }
        }

        Ok(result)
    }
}

fn is_watchlist_search_hit(item: &SuggestItem) -> bool {
    if item.code.is_empty() || item.name.is_empty() {
        return false;
    }
    matches!(item.classify.as_str(), "AStock" | "HK")
}

fn exchange_for_search_hit(item: &SuggestItem) -> Exchange {
    match item.classify.as_str() {
        "HK" => Exchange::HK,
        "AStock" => Exchange::from_stock_code(&item.code),
        _ => Exchange::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_suggest_response_data_field() {
        let json = r#"{"QuotationCodeTable":{"Data":[{"Code":"600519","Name":"贵州茅台","SecurityTypeName":"沪A","Classify":"AStock"}]}}"#;
        let response: SuggestResponse = serde_json::from_str(json).unwrap();
        let items = response.table.unwrap().data.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].code, "600519");
        assert_eq!(items[0].name, "贵州茅台");
    }

    #[test]
    fn watchlist_search_filters_hits() {
        let hits = [
            SuggestItem {
                code: "6869".into(),
                name: "希森美康".into(),
                security_type_name: "日股".into(),
                classify: "JPX".into(),
            },
            SuggestItem {
                code: "06869".into(),
                name: "长飞光纤光缆".into(),
                security_type_name: "港股".into(),
                classify: "HK".into(),
            },
            SuggestItem {
                code: "136869".into(),
                name: "16广核01".into(),
                security_type_name: "债券".into(),
                classify: "Bond".into(),
            },
            SuggestItem {
                code: "688981".into(),
                name: "中芯国际".into(),
                security_type_name: "沪A".into(),
                classify: "AStock".into(),
            },
        ];
        assert!(!is_watchlist_search_hit(&hits[0]));
        assert!(is_watchlist_search_hit(&hits[1]));
        assert!(!is_watchlist_search_hit(&hits[2]));
        assert!(is_watchlist_search_hit(&hits[3]));
    }
}
