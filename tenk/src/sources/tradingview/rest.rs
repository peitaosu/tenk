use reqwest::header::{HeaderMap, HeaderValue, ORIGIN, REFERER};
use serde_json::{json, Value};

use crate::data::{
    TvAdvice, TvAnalystForecasts, TvAnalystPriceTargets, TvAnalystRatings, TvAssetFilter,
    TvCalendarEvent, TvDrawing, TvDrawingPoint, TvHotlistKind, TvIndicatorInput, TvIndicatorMeta,
    TvIndicatorSpec, TvPeriodAdvice, TvScreenerRequest, TvScreenerResult, TvScreenerRow,
    TvSymbolMatch, TvTechnicalAnalysis, TvUserSession,
};
use crate::error::{DataError, DataResult};
use crate::request::RequestManager;

use super::symbol::{auth_cookie, scanner_market, to_tv_symbol};

const TV_ORIGIN: &str = "https://www.tradingview.com";

#[derive(Clone)]
pub struct TvRestClient {
    pub(crate) http: RequestManager,
    pub(crate) session: String,
    signature: String,
}

impl TvRestClient {
    pub fn new(http: RequestManager, session: String, signature: String) -> Self {
        Self {
            http,
            session,
            signature,
        }
    }

    pub(crate) fn tv_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, HeaderValue::from_static(TV_ORIGIN));
        headers.insert(REFERER, HeaderValue::from_static(TV_ORIGIN));
        if let Some(cookie) = self.ws_cookie() {
            if let Ok(value) = HeaderValue::from_str(&cookie) {
                headers.insert(reqwest::header::COOKIE, value);
            }
        }
        headers
    }

    pub(crate) fn ws_cookie(&self) -> Option<String> {
        if self.session.is_empty() {
            None
        } else {
            Some(auth_cookie(&self.session, &self.signature))
        }
    }

    pub async fn search_symbols(
        &self,
        query: &str,
        filter: Option<TvAssetFilter>,
        offset: u32,
    ) -> DataResult<Vec<TvSymbolMatch>> {
        let parts: Vec<&str> = query.split(':').collect();
        let (exchange, text) = if parts.len() == 2 {
            (Some(parts[0]), parts[1])
        } else {
            (None, query)
        };

        let mut params = vec![
            ("text", text.replace(' ', "+")),
            ("start", offset.to_string()),
        ];
        if let Some(filter) = filter {
            params.push(("search_type", filter.as_api_str().to_string()));
        }
        if let Some(exchange) = exchange {
            params.push(("exchange", exchange.to_string()));
        }

        let url = "https://symbol-search.tradingview.com/symbol_search/v3";
        let data: Value = self
            .http
            .get_with_headers(url, &params, Some(self.tv_headers()))
            .await?;

        let symbols = data
            .get("symbols")
            .and_then(Value::as_array)
            .ok_or_else(|| DataError::custom("missing symbols in search response"))?;

        Ok(symbols
            .iter()
            .filter_map(|item| {
                let exchange = item.get("exchange")?.as_str()?.split(' ').next()?.to_string();
                let symbol = item.get("symbol")?.as_str()?.to_string();
                let prefix = item.get("prefix").and_then(Value::as_str);
                let id = if let Some(prefix) = prefix {
                    format!("{prefix}:{symbol}")
                } else {
                    format!("{}:{symbol}", exchange.to_uppercase())
                };
                Some(TvSymbolMatch {
                    id,
                    exchange: exchange.clone(),
                    full_exchange: item.get("exchange")?.as_str()?.to_string(),
                    symbol,
                    description: item.get("description")?.as_str()?.to_string(),
                    asset_type: item.get("type")?.as_str()?.to_string(),
                })
            })
            .collect())
    }

    pub async fn technical_analysis(&self, symbol: &str) -> DataResult<TvTechnicalAnalysis> {
        let id = to_tv_symbol(symbol);
        let periods = ["1", "5", "15", "60", "240", "1D", "1W", "1M"];
        let indicators = ["Recommend.Other", "Recommend.All", "Recommend.MA"];
        let mut columns = Vec::new();
        for period in periods {
            for indicator in indicators {
                if period == "1D" {
                    columns.push(indicator.to_string());
                } else {
                    columns.push(format!("{indicator}|{period}"));
                }
            }
        }

        let body = json!({
            "symbols": { "tickers": [id.clone()] },
            "columns": columns,
        });

        let data: Value = self
            .http
            .post_json_with_headers(
                "https://scanner.tradingview.com/global/scan",
                &body,
                Some(self.tv_headers()),
            )
            .await?;

        let row = data
            .get("data")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .ok_or_else(|| DataError::NoDataAvailable)?;

        let values = row
            .get("d")
            .and_then(Value::as_array)
            .ok_or_else(|| DataError::custom("missing TA values"))?;

        let mut periods_map = std::collections::HashMap::new();
        for (index, value) in values.iter().enumerate() {
            let column = columns.get(index).cloned().unwrap_or_default();
            let (name, period) = if let Some((name, period)) = column.split_once('|') {
                (name, period.to_string())
            } else {
                (column.as_str(), "1D".to_string())
            };
            let score = value.as_f64().unwrap_or(0.0) * 1000.0 / 500.0;
            let advice = TvAdvice::from_score(score);
            let entry = periods_map
                .entry(period)
                .or_insert(TvPeriodAdvice {
                    oscillators: TvAdvice::Neutral,
                    moving_averages: TvAdvice::Neutral,
                    overall: TvAdvice::Neutral,
                });
            match name.split('.').next_back().unwrap_or(name) {
                "Other" => entry.oscillators = advice,
                "MA" => entry.moving_averages = advice,
                _ => entry.overall = advice,
            }
        }

        Ok(TvTechnicalAnalysis {
            symbol: id,
            periods: periods_map,
        })
    }

    pub async fn analyst_snapshot(
        &self,
        symbol: &str,
    ) -> DataResult<(TvAnalystRatings, TvAnalystPriceTargets, TvAnalystForecasts)> {
        let id = to_tv_symbol(symbol);
        let market = scanner_market(&id);
        let columns = [
            "recommendation_buy",
            "recommendation_sell",
            "recommendation_hold",
            "recommendation_over",
            "recommendation_under",
            "recommendation_total",
            "recommendation_mark",
            "price_target_average",
            "price_target_high",
            "price_target_low",
            "price_target_median",
            "earnings_per_share_forecast_next_fy",
            "revenue_forecast_next_fy",
        ];
        let body = json!({
            "symbols": { "tickers": [id.clone()] },
            "columns": columns,
        });
        let url = format!("https://scanner.tradingview.com/{market}/scan");
        let data: Value = self
            .http
            .post_json_with_headers(&url, &body, Some(self.tv_headers()))
            .await?;
        let values = data
            .get("data")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(|row| row.get("d"))
            .and_then(Value::as_array)
            .ok_or_else(|| DataError::NoDataAvailable)?;
        let number = |index: usize| values.get(index).and_then(Value::as_f64);
        let count = |index: usize| number(index).map(|value| value.round() as u32).unwrap_or(0);
        Ok((
            TvAnalystRatings {
                buy: count(0),
                sell: count(1),
                hold: count(2),
                over: count(3),
                under: count(4),
                total: count(5),
                mark: number(6),
            },
            TvAnalystPriceTargets {
                average: number(7),
                high: number(8),
                low: number(9),
                median: number(10),
            },
            TvAnalystForecasts {
                eps_next_fy: number(11),
                revenue_next_fy: number(12),
            },
        ))
    }

    pub async fn screener(&self, request: &TvScreenerRequest) -> DataResult<TvScreenerResult> {
        let filters: Vec<Value> = request
            .filters
            .iter()
            .map(|filter| {
                json!({
                    "left": filter.field,
                    "operation": filter.operation,
                    "right": filter.value,
                })
            })
            .collect();

        let body = json!({
            "filter": filters,
            "columns": request.columns,
            "sort": {
                "sortBy": request.sort_by,
                "sortOrder": request.sort_order,
            },
            "range": [request.range_start, request.range_end],
            "preset": request.preset,
            "options": { "lang": "en" },
        });

        let url = format!(
            "https://scanner.tradingview.com/{}/scan",
            request.market
        );
        let data: Value = self
            .http
            .post_json_with_headers(&url, &body, Some(self.tv_headers()))
            .await?;

        let total_count = data.get("totalCount").and_then(Value::as_i64).unwrap_or(0);
        let rows = data
            .get("data")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        let symbol = item.get("s")?.as_str()?.to_string();
                        let values_array = item.get("d")?.as_array()?;
                        let mut values = std::collections::HashMap::new();
                        for (index, column) in request.columns.iter().enumerate() {
                            if let Some(value) = values_array.get(index) {
                                values.insert(column.clone(), value.clone());
                            }
                        }
                        Some(TvScreenerRow { symbol, values })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(TvScreenerResult {
            market: request.market.clone(),
            total_count,
            rows,
        })
    }

    pub async fn hotlist(
        &self,
        market: &str,
        kind: TvHotlistKind,
        limit: usize,
    ) -> DataResult<TvScreenerResult> {
        let (sort_by, sort_order, preset) = match kind {
            TvHotlistKind::Gainers => ("change", "desc", "gainers"),
            TvHotlistKind::Losers => ("change", "asc", "losers"),
            TvHotlistKind::Active => ("volume", "desc", "all_stocks"),
            TvHotlistKind::PreMarketGainers => ("premarket_change", "desc", "all_stocks"),
            TvHotlistKind::AfterHoursGainers => {
                ("postmarket_change", "desc", "after_hours_gainers")
            }
        };

        let request = TvScreenerRequest {
            market: market.to_string(),
            columns: vec![
                "name".into(),
                "close".into(),
                "change".into(),
                "volume".into(),
                "market".into(),
            ],
            filters: Vec::new(),
            sort_by: sort_by.into(),
            sort_order: sort_order.into(),
            range_start: 0,
            range_end: limit,
            preset: preset.into(),
        };
        self.screener(&request).await
    }

    pub async fn calendar(
        &self,
        from: &str,
        to: &str,
        countries: &str,
    ) -> DataResult<Vec<TvCalendarEvent>> {
        let params = [
            ("from", from.to_string()),
            ("to", to.to_string()),
            ("countries", countries.to_string()),
        ];
        let data: Value = self
            .http
            .get_with_headers(
                "https://economic-calendar.tradingview.com/events",
                &params,
                Some(self.tv_headers()),
            )
            .await?;

        let rows = data
            .get("result")
            .and_then(Value::as_array)
            .ok_or_else(|| DataError::custom("missing calendar result"))?;

        Ok(rows
            .iter()
            .filter_map(|row| {
                Some(TvCalendarEvent {
                    id: row.get("id")?.as_str()?.to_string(),
                    title: row.get("title")?.as_str()?.to_string(),
                    country: row.get("country")?.as_str()?.to_string(),
                    indicator: row.get("indicator")?.as_str()?.to_string(),
                    period: row.get("period")?.as_str()?.to_string(),
                    source: row.get("source")?.as_str()?.to_string(),
                    actual: row.get("actual").and_then(parse_number),
                    previous: row.get("previous").and_then(parse_number),
                    forecast: row.get("forecast").and_then(parse_number),
                    currency: row.get("currency")?.as_str()?.to_string(),
                    unit: row.get("unit").and_then(|v| v.as_str().map(str::to_string)),
                    importance: row.get("importance")?.as_i64()?,
                    date: row.get("date")?.as_str()?.to_string(),
                    ticker: row.get("ticker").and_then(|v| v.as_str().map(str::to_string)),
                })
            })
            .collect())
    }

    pub async fn search_indicators(&self, query: &str) -> DataResult<Vec<TvIndicatorMeta>> {
        let mut results = Vec::new();
        for filter in ["standard", "candlestick", "fundamental"] {
            let params = [("filter", filter.to_string())];
            let data: Value = self
                .http
                .get_with_headers(
                    "https://pine-facade.tradingview.com/pine-facade/list",
                    &params,
                    Some(self.tv_headers()),
                )
                .await?;
            if let Some(items) = data.as_array() {
                for item in items {
                    let name = item.get("scriptName").and_then(Value::as_str).unwrap_or("");
                    if !query.is_empty()
                        && !name.to_ascii_uppercase().contains(&query.to_ascii_uppercase())
                    {
                        continue;
                    }
                    results.push(map_builtin_indicator(item));
                }
            }
        }

        let params = [("search", query.replace(' ', "%20"))];
        let data: Value = self
            .http
            .get_with_headers(
                "https://www.tradingview.com/pubscripts-suggest-json",
                &params,
                Some(self.tv_headers()),
            )
            .await?;

        if let Some(items) = data.get("results").and_then(Value::as_array) {
            for item in items {
                let access = item
                    .get("access")
                    .and_then(Value::as_i64)
                    .map(|value| match value {
                        1 => "open_source",
                        2 => "closed_source",
                        3 => "invite_only",
                        _ => "other",
                    })
                    .unwrap_or("other");
                results.push(TvIndicatorMeta {
                    id: item
                        .get("scriptIdPart")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    version: item
                        .get("version")
                        .and_then(Value::as_str)
                        .unwrap_or("last")
                        .to_string(),
                    name: item
                        .get("scriptName")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    author_id: item
                        .get("author")
                        .and_then(|v| v.get("id"))
                        .and_then(Value::as_i64)
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    author_name: item
                        .get("author")
                        .and_then(|v| v.get("username"))
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    image: item
                        .get("imageUrl")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    access: access.to_string(),
                    indicator_type: item
                        .get("extra")
                        .and_then(|v| v.get("kind"))
                        .and_then(Value::as_str)
                        .unwrap_or("study")
                        .to_string(),
                    source: item
                        .get("scriptSource")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                });
            }
        }

        Ok(results)
    }

    pub async fn get_indicator(&self, id: &str, version: &str) -> DataResult<TvIndicatorSpec> {
        if is_builtin_study(id) {
            return Ok(build_builtin_spec(id, version));
        }
        let encoded = id.replace([' ', '%'], "%25");
        let url = format!(
            "https://pine-facade.tradingview.com/pine-facade/translate/{encoded}/{version}"
        );
        let data: Value = self
            .http
            .get_with_headers(&url, &[], Some(self.tv_headers()))
            .await?;

        if !data.get("success").and_then(Value::as_bool).unwrap_or(false) {
            let reason = data
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("unknown indicator");
            return Err(DataError::custom(reason));
        }

        let meta = data
            .get("result")
            .and_then(|v| v.get("metaInfo"))
            .ok_or_else(|| DataError::custom("missing indicator meta"))?;

        let mut inputs = Vec::new();
        if let Some(raw_inputs) = meta.get("inputs").and_then(Value::as_array) {
            for input in raw_inputs {
                let id = input.get("id").and_then(Value::as_str).unwrap_or_default();
                if matches!(id, "text" | "pineId" | "pineVersion") {
                    continue;
                }
                inputs.push(crate::data::TvIndicatorInput {
                    id: id.to_string(),
                    name: input
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    input_type: input
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    value: input.get("defval").cloned().unwrap_or(Value::Null),
                    options: input.get("options").and_then(|v| v.as_array().cloned()),
                });
            }
        }

        let mut plots = std::collections::HashMap::new();
        if let Some(styles) = meta.get("styles").and_then(Value::as_object) {
            for (plot_id, style) in styles {
                let title = style
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or(plot_id)
                    .replace([' ', '-'], "_");
                plots.insert(plot_id.clone(), title);
            }
        }

        Ok(TvIndicatorSpec {
            pine_id: meta
                .get("scriptIdPart")
                .and_then(Value::as_str)
                .unwrap_or(id)
                .to_string(),
            pine_version: meta
                .get("pine")
                .and_then(|v| v.get("version"))
                .and_then(Value::as_str)
                .unwrap_or(version)
                .to_string(),
            description: meta
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            short_description: meta
                .get("shortDescription")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            inputs,
            plots,
            script: data
                .get("result")
                .and_then(|v| v.get("ilTemplate"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            kind: indicator_kind(meta, id),
        })
    }

    pub async fn private_indicators(&self) -> DataResult<Vec<TvIndicatorMeta>> {
        let params = [("filter", "saved".to_string())];
        let data: Value = self
            .http
            .get_with_headers(
                "https://pine-facade.tradingview.com/pine-facade/list",
                &params,
                Some(self.tv_headers()),
            )
            .await?;

        Ok(data
            .as_array()
            .map(|items| items.iter().map(map_builtin_indicator).collect())
            .unwrap_or_default())
    }

    pub async fn login(&self, username: &str, password: &str) -> DataResult<TvUserSession> {
        let body = format!(
            "username={}&password={}&remember=on",
            urlencoding_encode(username),
            urlencoding_encode(password)
        );
        let client = self.http.client();
        let response = client
            .post("https://www.tradingview.com/accounts/signin/")
            .header(ORIGIN, TV_ORIGIN)
            .header(REFERER, TV_ORIGIN)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(body)
            .send()
            .await?;

        let headers = response.headers().clone();
        let data: Value = response
            .json()
            .await
            .map_err(|e| DataError::custom(e.to_string()))?;

        if let Some(error) = data.get("error").and_then(Value::as_str) {
            return Err(DataError::custom(error));
        }

        let user = data
            .get("user")
            .ok_or_else(|| DataError::custom("missing user in login response"))?;

        let cookies: Vec<String> = headers
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok().map(str::to_string))
            .collect();

        let session = extract_cookie(&cookies, "sessionid=");
        let signature = extract_cookie(&cookies, "sessionid_sign=");

        Ok(TvUserSession {
            user_id: user.get("id").and_then(Value::as_i64).unwrap_or(0),
            username: user
                .get("username")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            first_name: user
                .get("first_name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            last_name: user
                .get("last_name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            session,
            signature,
            auth_token: user
                .get("auth_token")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        })
    }

    pub async fn get_drawings(
        &self,
        layout: &str,
        symbol: &str,
        user_id: i64,
    ) -> DataResult<Vec<TvDrawing>> {
        let token_url = "https://www.tradingview.com/chart-token/";
        let params = [
            ("image_url", layout.to_string()),
            ("user_id", user_id.to_string()),
        ];
        let token_data: Value = self
            .http
            .get_with_headers(token_url, &params, Some(self.tv_headers()))
            .await
            .map_err(|error| {
                if error.to_string().contains("403") {
                    DataError::custom(
                        "TradingView session required for chart drawings: set TENK_TV_SESSION and TENK_TV_SIGNATURE",
                    )
                } else {
                    error
                }
            })?;
        let token = token_data
            .get("token")
            .and_then(Value::as_str)
            .ok_or_else(|| DataError::custom("invalid layout or credentials"))?;

        let url = format!(
            "https://charts-storage.tradingview.com/charts-storage/get/layout/{layout}/sources"
        );
        let chart_ids = ["1", "2", "_shared"];
        let mut drawings = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for chart_id in chart_ids {
            let mut params = vec![
                ("chart_id", chart_id.to_string()),
                ("jwt", token.to_string()),
            ];
            if !symbol.is_empty() {
                params.push(("symbol", symbol.to_string()));
            }
            let data: Value = match self
                .http
                .get_with_headers(&url, &params, Some(self.tv_headers()))
                .await
            {
                Ok(data) => data,
                Err(_) => continue,
            };
            let sources = data
                .get("payload")
                .and_then(|value| value.get("sources"))
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            for drawing in parse_drawing_sources(sources) {
                if seen.insert(drawing.id.clone()) {
                    drawings.push(drawing);
                }
            }
        }

        Ok(drawings)
    }

    pub async fn fetch_session_auth_token(&self) -> DataResult<String> {
        if self.session.is_empty() {
            return Ok(String::new());
        }
        let mut location = "https://www.tradingview.com/".to_string();
        for _ in 0..5 {
            let response = self
                .http
                .client()
                .get(&location)
                .headers(self.tv_headers())
                .send()
                .await
                .map_err(DataError::Network)?;
            if response.status().is_redirection() {
                let next = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .map(|value| resolve_redirect_url(&location, value))
                    .ok_or_else(|| DataError::custom("TradingView auth redirect missing location"))?;
                location = next;
                continue;
            }
            let body = response
                .text()
                .await
                .map_err(|error| DataError::custom(error.to_string()))?;
            let token = extract_auth_token(&body);
            if !token.is_empty() {
                return Ok(token);
            }
            return Err(DataError::custom("wrong or expired sessionid/signature"));
        }
        Err(DataError::custom("too many TradingView auth redirects"))
    }
}

fn parse_drawing_sources(
    sources: serde_json::Map<String, Value>,
) -> Vec<TvDrawing> {
    sources
        .values()
        .filter_map(|drawing| {
            let state = drawing.get("state").cloned().unwrap_or(Value::Null);
            let points = state
                .get("points")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|point| {
                            Some(TvDrawingPoint {
                                time: point.get("time_t")?.as_i64()?,
                                price: point.get("price")?.as_f64()?,
                                offset: point.get("offset")?.as_f64().unwrap_or(0.0),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(TvDrawing {
                id: drawing
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                symbol: drawing
                    .get("symbol")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                drawing_type: drawing
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                points,
                state,
            })
        })
        .collect()
}

pub(crate) fn extract_auth_token(body: &str) -> String {
    body.split("\"auth_token\":\"")
        .nth(1)
        .and_then(|part| part.split('"').next())
        .unwrap_or_default()
        .to_string()
}

fn resolve_redirect_url(base: &str, location: &str) -> String {
    if location.starts_with("http://") || location.starts_with("https://") {
        location.to_string()
    } else if location.starts_with('/') {
        format!("https://www.tradingview.com{location}")
    } else {
        format!("{base}/{location}")
    }
}

fn map_builtin_indicator(item: &Value) -> TvIndicatorMeta {
    TvIndicatorMeta {
        id: item
            .get("scriptIdPart")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        version: item
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("last")
            .to_string(),
        name: item
            .get("scriptName")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        author_id: item
            .get("userId")
            .and_then(Value::as_i64)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-1".to_string()),
        author_name: "@TRADINGVIEW@".to_string(),
        image: item
            .get("imageUrl")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        access: "closed_source".to_string(),
        indicator_type: item
            .get("extra")
            .and_then(|v| v.get("kind"))
            .and_then(Value::as_str)
            .unwrap_or("study")
            .to_string(),
        source: String::new(),
    }
}

fn parse_number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
}

fn extract_cookie(cookies: &[String], key: &str) -> String {
    cookies
        .iter()
        .find(|cookie| cookie.contains(key))
        .and_then(|cookie| {
            cookie
                .split(';')
                .find(|part| part.trim_start().starts_with(key))
                .map(|part| part.trim_start()[key.len()..].to_string())
        })
        .unwrap_or_default()
}

fn urlencoding_encode(input: &str) -> String {
    input
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn indicator_kind(meta: &Value, id: &str) -> String {
    if meta
        .get("isTVScriptStrategy")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return "strategy".to_string();
    }
    if let Some(kind) = meta
        .get("extra")
        .and_then(|extra| extra.get("kind"))
        .and_then(Value::as_str)
    {
        return kind.to_string();
    }
    let script_id = meta
        .get("scriptIdPart")
        .and_then(Value::as_str)
        .unwrap_or(id);
    if script_id.contains("Strategy") {
        return "strategy".to_string();
    }
    "study".to_string()
}

pub fn build_study_inputs(spec: &TvIndicatorSpec) -> (String, Value) {
    if is_builtin_study(&spec.pine_id) {
        let mut inputs = json!({});
        for input in &spec.inputs {
            inputs[&input.id] = input.value.clone();
        }
        return (spec.pine_id.clone(), inputs);
    }
    let study_type = if spec.kind == "strategy" {
        "StrategyScript@tv-scripting-101!"
    } else {
        "Script@tv-scripting-101!"
    };
    let mut inputs = json!({
        "text": spec.script,
        "pineId": spec.pine_id,
        "pineVersion": spec.pine_version,
    });
    for (index, input) in spec.inputs.iter().enumerate() {
        let value = if input.input_type == "color" {
            json!(index)
        } else {
            input.value.clone()
        };
        inputs[&input.id] = json!({
            "v": value,
            "f": false,
            "t": input.input_type,
        });
    }
    (study_type.to_string(), inputs)
}

fn is_builtin_study(id: &str) -> bool {
    id.contains("@tv-basicstudies")
        || id.contains("@tv-volumebyprice")
        || id.contains("@tv-prostudies")
        || id.contains("@tv-chart_patterns")
}

fn build_builtin_spec(id: &str, version: &str) -> TvIndicatorSpec {
    let mut inputs = Vec::new();
    let mut plots = std::collections::HashMap::new();
    if id.starts_with("Volume@") {
        inputs.push(TvIndicatorInput {
            id: "length".into(),
            name: "length".into(),
            input_type: "integer".into(),
            value: json!(20),
            options: None,
        });
        inputs.push(TvIndicatorInput {
            id: "col_prev_close".into(),
            name: "col_prev_close".into(),
            input_type: "bool".into(),
            value: json!(false),
            options: None,
        });
        plots.insert("plot_0".into(), "Volume".into());
    } else if id.starts_with("RSI@") {
        inputs.push(TvIndicatorInput {
            id: "length".into(),
            name: "length".into(),
            input_type: "integer".into(),
            value: json!(14),
            options: None,
        });
        plots.insert("plot_0".into(), "RSI".into());
    }
    TvIndicatorSpec {
        pine_id: id.to_string(),
        pine_version: version.to_string(),
        description: id.to_string(),
        short_description: id.to_string(),
        inputs,
        plots,
        script: String::new(),
        kind: "study".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_redirect_url() {
        assert_eq!(
            resolve_redirect_url("https://www.tradingview.com/", "/chart/"),
            "https://www.tradingview.com/chart/"
        );
        assert_eq!(
            resolve_redirect_url("https://www.tradingview.com/", "https://fr.tradingview.com/"),
            "https://fr.tradingview.com/"
        );
    }

    #[test]
    fn test_extract_auth_token() {
        let body = r#"{"auth_token":"abc123","username":"demo"}"#;
        assert_eq!(extract_auth_token(body), "abc123");
    }

    #[test]
    fn test_build_study_inputs_strategy_type() {
        let spec = TvIndicatorSpec {
            pine_id: "STD;RSI%1Strategy".into(),
            pine_version: "last".into(),
            description: String::new(),
            short_description: String::new(),
            inputs: vec![],
            plots: Default::default(),
            script: "script".into(),
            kind: "strategy".into(),
        };
        let (study_type, _) = build_study_inputs(&spec);
        assert_eq!(study_type, "StrategyScript@tv-scripting-101!");
    }
}
