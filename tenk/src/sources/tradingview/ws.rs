use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use rustls::pki_types::ServerName;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_rustls::TlsConnector;
use tokio_tungstenite::{
    client_async, connect_async,
    tungstenite::http::{header, Request},
    tungstenite::{client::IntoClientRequest, Message},
    MaybeTlsStream,
};

use crate::data::{
    TvAnalystEstimates, TvChartBar, TvChartOptions, TvEstimatePoint, TvEstimateSeries,
    TvIndicatorPoint, TvIndicatorSeries, TvIndicatorSpec, TvMarketInfo, TvQuote, TvReplayResult,
    TvStrategyPerformance, TvStrategyReport, TvTradeReport,
};
use crate::error::{DataError, DataResult};

use super::protocol::{
    format_message, format_ping_response, parse_compressed, parse_packets, value_as_f64,
    value_as_i64,
};
use super::rest::build_study_inputs;
use super::symbol::{gen_session_id, to_tv_symbol};

const TV_ORIGIN: &str = "https://www.tradingview.com";
const TV_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
const WS_URL: &str = "wss://data.tradingview.com/socket.io/websocket?from=chart&type=chart";
const WS_HOST: &str = "data.tradingview.com";
const WS_PORT: u16 = 443;
const WS_TIMEOUT: Duration = Duration::from_secs(45);
const CHART_SERIES_ID: &str = "$prices";
const CHART_SERIES_KEY: &str = "s1";
const CHART_SYMBOL_ALIAS: &str = "ser_1";
const CHART_STUDY_PARENT: &str = "st1";

fn create_study_params(
    chart_session: &str,
    study_id: &str,
    study_type: &str,
    study_inputs: Value,
) -> Vec<Value> {
    vec![
        Value::from(chart_session),
        Value::from(study_id),
        Value::from(CHART_STUDY_PARENT),
        Value::from(CHART_SERIES_ID),
        Value::from(study_type),
        study_inputs,
    ]
}

pub async fn fetch_quotes(
    auth_token: &str,
    symbols: &[String],
    proxy: Option<&str>,
    cookie: Option<&str>,
) -> DataResult<Vec<TvQuote>> {
    let mut session = WsSession::connect(auth_token, proxy, cookie).await?;
    let quote_session = gen_session_id("qs");
    session.queue(format_message(
        "quote_create_session",
        &[Value::from(quote_session.clone())],
    )).await?;
    session.queue(format_message(
        "quote_set_fields",
        &[
            Value::from(quote_session.clone()),
            Value::from("lp"),
            Value::from("ch"),
            Value::from("chp"),
            Value::from("volume"),
            Value::from("open_price"),
            Value::from("high_price"),
            Value::from("low_price"),
            Value::from("prev_close_price"),
            Value::from("bid"),
            Value::from("ask"),
            Value::from("description"),
            Value::from("exchange"),
            Value::from("currency_code"),
            Value::from("market_cap_basic"),
            Value::from("price_earnings_ttm"),
            Value::from("sector"),
        ],
    )).await?;

    let mut keys = Vec::new();
    for symbol in symbols {
        let tv_symbol = to_tv_symbol(symbol);
        let key = format!(
            "={}",
            json!({ "symbol": tv_symbol, "adjustment": "splits", "session": "regular" })
        );
        keys.push((tv_symbol, key.clone()));
        session.queue(format_message(
            "quote_add_symbols",
            &[Value::from(quote_session.clone()), Value::from(key)],
        )).await?;
    }

    let deadline = tokio::time::Instant::now() + WS_TIMEOUT;
    let mut quotes: HashMap<String, TvQuote> = HashMap::new();

    while tokio::time::Instant::now() < deadline {
        let ready = keys.iter().all(|(symbol, _)| {
            quotes
                .get(symbol)
                .and_then(|quote| quote.last_price)
                .is_some()
        });
        if ready {
            break;
        }
        if let Some(packet) = session.recv_until(deadline).await? {
            let packet_type = packet.get("m").and_then(Value::as_str).unwrap_or("");
            let params = packet.get("p").and_then(Value::as_array).cloned().unwrap_or_default();
            if packet_type == "qsd" {
                if params.len() < 2 {
                    continue;
                }
                let payload = &params[1];
                let status = payload.get("s").and_then(Value::as_str).unwrap_or("");
                if status != "ok" {
                    continue;
                }
                let key = payload.get("n").and_then(Value::as_str).unwrap_or("");
                let values = payload.get("v").cloned().unwrap_or(Value::Null);
                if let Some(obj) = values.as_object() {
                    let map = obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                    if let Some(symbol) = find_quote_symbol(&keys, key) {
                        quotes.insert(symbol.clone(), map_quote(&symbol, map));
                    }
                }
            }
        }
    }

    session.queue(format_message(
        "quote_delete_session",
        &[Value::from(quote_session)],
    )).await?;
    session.close().await?;

    let results: Vec<TvQuote> = keys
        .into_iter()
        .filter_map(|(symbol, _)| quotes.remove(&symbol))
        .collect();
    if results.is_empty() {
        return Err(DataError::NoDataAvailable);
    }
    Ok(results)
}

const ANALYST_ESTIMATE_FIELDS: &[&str] = &[
    "earnings_fiscal_period_fq_h",
    "revenue_forecast_fq_h",
    "earnings_per_share_forecast_fq_h",
    "earnings_per_share_fq_h",
    "earnings_fiscal_period_fy_h",
    "revenue_forecast_fy_h",
    "earnings_per_share_forecast_fy_h",
    "earnings_per_share_fy_h",
];

pub async fn fetch_analyst_estimates(
    auth_token: &str,
    symbol: &str,
    proxy: Option<&str>,
    cookie: Option<&str>,
) -> DataResult<TvAnalystEstimates> {
    let tv_symbol = to_tv_symbol(symbol);
    let mut session = WsSession::connect(auth_token, proxy, cookie).await?;
    let quote_session = gen_session_id("qs");
    session.queue(format_message(
        "quote_create_session",
        &[Value::from(quote_session.clone())],
    )).await?;
    let mut fields = vec![Value::from(quote_session.clone())];
    fields.extend(ANALYST_ESTIMATE_FIELDS.iter().copied().map(Value::from));
    session.queue(format_message("quote_set_fields", &fields)).await?;
    let key = format!(
        "={}",
        json!({ "symbol": tv_symbol, "adjustment": "splits", "session": "regular" })
    );
    session.queue(format_message(
        "quote_add_symbols",
        &[Value::from(quote_session.clone()), Value::from(key.clone())],
    )).await?;

    let deadline = tokio::time::Instant::now() + WS_TIMEOUT;
    let mut values: HashMap<String, Value> = HashMap::new();
    while tokio::time::Instant::now() < deadline {
        if ANALYST_ESTIMATE_FIELDS
            .iter()
            .any(|field| values.contains_key(*field))
        {
            break;
        }
        if let Some(packet) = session.recv_until(deadline).await? {
            let packet_type = packet.get("m").and_then(Value::as_str).unwrap_or("");
            let params = packet.get("p").and_then(Value::as_array).cloned().unwrap_or_default();
            if packet_type != "qsd" || params.len() < 2 {
                continue;
            }
            let payload = &params[1];
            if payload.get("s").and_then(Value::as_str) != Some("ok") {
                continue;
            }
            if payload.get("n").and_then(Value::as_str) != Some(key.as_str()) {
                continue;
            }
            if let Some(map) = payload.get("v").and_then(Value::as_object) {
                for field in ANALYST_ESTIMATE_FIELDS {
                    if let Some(value) = map.get(*field) {
                        values.insert(field.to_string(), value.clone());
                    }
                }
            }
        }
    }

    session.queue(format_message(
        "quote_delete_session",
        &[Value::from(quote_session)],
    )).await?;
    session.close().await?;

    if values.is_empty() {
        return Err(DataError::NoDataAvailable);
    }

    Ok(TvAnalystEstimates {
        earnings_fq: parse_estimate_series(values.get("earnings_fiscal_period_fq_h")),
        revenue_fq: parse_estimate_series(values.get("revenue_forecast_fq_h")),
        eps_forecast_fq: parse_estimate_series(values.get("earnings_per_share_forecast_fq_h")),
        eps_actual_fq: parse_estimate_series(values.get("earnings_per_share_fq_h")),
        earnings_fy: parse_estimate_series(values.get("earnings_fiscal_period_fy_h")),
        revenue_fy: parse_estimate_series(values.get("revenue_forecast_fy_h")),
        eps_forecast_fy: parse_estimate_series(values.get("earnings_per_share_forecast_fy_h")),
        eps_actual_fy: parse_estimate_series(values.get("earnings_per_share_fy_h")),
    })
}

pub async fn fetch_chart(
    auth_token: &str,
    symbol: &str,
    options: &TvChartOptions,
    proxy: Option<&str>,
    cookie: Option<&str>,
) -> DataResult<(Vec<TvChartBar>, TvMarketInfo)> {
    let tv_symbol = to_tv_symbol(symbol);
    let mut session = WsSession::connect(auth_token, proxy, cookie).await?;
    let chart_session = gen_session_id("cs");
    let replay_session = gen_session_id("rs");

    session.queue(format_message(
        "chart_create_session",
        &[Value::from(chart_session.clone()), Value::from("")],
    )).await?;

    let symbol_init = chart_symbol_json(&tv_symbol, options);

    let mut chart_init = symbol_init.clone();
    if let Some(chart_type) = options.chart_type {
        chart_init = json!({
            "symbol": symbol_init,
            "type": chart_type.study_id(),
        });
    }

    if options.replay_from.is_some() {
        session.queue(format_message(
            "replay_create_session",
            &[Value::from(replay_session.clone())],
        )).await?;
        session.queue(format_message(
            "replay_add_series",
            &[
                Value::from(replay_session.clone()),
                Value::from("req_replay_addseries"),
                Value::from(format!("={symbol_init}")),
                Value::from(options.timeframe.as_api_str()),
            ],
        )).await?;
        if let Some(replay_from) = options.replay_from {
            session.queue(format_message(
                "replay_reset",
                &[
                    Value::from(replay_session.clone()),
                    Value::from("req_replay_reset"),
                    Value::from(replay_from),
                ],
            )).await?;
            chart_init = json!({
                "replay": replay_session,
                "symbol": symbol_init,
            });
        }
    }

    session.queue(format_message(
        "resolve_symbol",
        &[
            Value::from(chart_session.clone()),
            Value::from(CHART_SYMBOL_ALIAS),
            Value::from(format!("={chart_init}")),
        ],
    )).await?;

    let range = if let Some(to) = options.to {
        json!(["bar_count", to, options.range])
    } else {
        json!(options.range)
    };

    session.queue(format_message(
        "create_series",
        &[
            Value::from(chart_session.clone()),
            Value::from(CHART_SERIES_ID),
            Value::from(CHART_SERIES_KEY),
            Value::from(CHART_SYMBOL_ALIAS),
            Value::from(options.timeframe.as_api_str()),
            range,
        ],
    )).await?;
    session
        .queue(format_message(
            "switch_timezone",
            &[Value::from(chart_session.clone()), Value::from("exchange")],
        ))
        .await?;

    let deadline = tokio::time::Instant::now() + WS_TIMEOUT;
    let mut bars: HashMap<i64, TvChartBar> = HashMap::new();
    let mut info = TvMarketInfo {
        full_name: tv_symbol.clone(),
        description: tv_symbol.clone(),
        exchange: String::new(),
        currency: String::new(),
        asset_type: String::new(),
        timezone: String::new(),
        has_intraday: false,
        is_replayable: false,
    };
    let mut symbol_loaded = false;
    let mut series_completed = false;

    while (!symbol_loaded || (!series_completed && bars.len() < options.range))
        && tokio::time::Instant::now() < deadline
    {
        if let Some(packet) = session.recv_until(deadline).await? {
            let packet_type = packet.get("m").and_then(Value::as_str).unwrap_or("");
            let params = packet.get("p").and_then(Value::as_array).cloned().unwrap_or_default();
            if let Some(error) = chart_error_message(packet_type, &params) {
                return Err(DataError::custom(error));
            }
            if packet_type == "symbol_resolved" && params.len() > 2 {
                symbol_loaded = true;
                if let Some(meta) = params[2].as_object() {
                    info = map_market_info(&tv_symbol, meta);
                }
            }
            if packet_type == "series_completed" {
                series_completed = true;
            }
            if matches!(packet_type, "timescale_update" | "du") && params.len() > 1 {
                if let Some(prices) = params[1].get(CHART_SERIES_ID) {
                    merge_bars(&mut bars, prices);
                    if !bars.is_empty() {
                        symbol_loaded = true;
                    }
                }
            }
        }
    }

    session.queue(format_message(
        "chart_delete_session",
        &[Value::from(chart_session)],
    )).await?;
    session.close().await?;

    if bars.is_empty() {
        return Err(DataError::NoDataAvailable);
    }

    let mut bars: Vec<TvChartBar> = bars.into_values().collect();
    bars.sort_by_key(|bar| bar.time);
    if bars.is_empty() {
        return Err(DataError::NoDataAvailable);
    }
    Ok((bars, info))
}

pub async fn fetch_indicator_series(
    auth_token: &str,
    symbol: &str,
    spec: &TvIndicatorSpec,
    options: &TvChartOptions,
    proxy: Option<&str>,
    cookie: Option<&str>,
) -> DataResult<TvIndicatorSeries> {
    let tv_symbol = to_tv_symbol(symbol);
    let (study_type, study_inputs) = build_study_inputs(spec);
    let mut session = WsSession::connect(auth_token, proxy, cookie).await?;
    let chart_session = gen_session_id("cs");
    let study_id = gen_session_id("st");
    let symbol_json = chart_symbol_json(&tv_symbol, options);

    session.queue(format_message(
        "chart_create_session",
        &[Value::from(chart_session.clone()), Value::from("")],
    )).await?;
    session.queue(format_message(
        "resolve_symbol",
        &[
            Value::from(chart_session.clone()),
            Value::from(CHART_SYMBOL_ALIAS),
            Value::from(format!("={symbol_json}")),
        ],
    )).await?;
    session.queue(format_message(
        "create_series",
        &[
            Value::from(chart_session.clone()),
            Value::from(CHART_SERIES_ID),
            Value::from(CHART_SERIES_KEY),
            Value::from(CHART_SYMBOL_ALIAS),
            Value::from(options.timeframe.as_api_str()),
            Value::from(options.range),
        ],
    )).await?;
    session
        .queue(format_message(
            "switch_timezone",
            &[Value::from(chart_session.clone()), Value::from("exchange")],
        ))
        .await?;
    session.queue(format_message(
        "create_study",
        &create_study_params(
            &chart_session,
            &study_id,
            &study_type,
            study_inputs,
        ),
    )).await?;

    let deadline = tokio::time::Instant::now() + WS_TIMEOUT;
    let mut points: HashMap<i64, TvIndicatorPoint> = HashMap::new();
    let plots = spec.plots.clone();
    let mut ready = false;

    while (!ready || points.is_empty()) && tokio::time::Instant::now() < deadline {
        if let Some(packet) = session.recv_until(deadline).await? {
            let packet_type = packet.get("m").and_then(Value::as_str).unwrap_or("");
            let params = packet.get("p").and_then(Value::as_array).cloned().unwrap_or_default();
            if let Some(error) = chart_error_message(packet_type, &params) {
                return Err(DataError::custom(error));
            }
            if matches!(packet_type, "timescale_update" | "du" | "study_completed") && params.len() > 1 {
                for study in study_updates_from_packet(&params, &study_id) {
                    if let Some(payload) = study_payload_from_update(study) {
                        merge_indicator_study(&mut points, &payload, &plots);
                    } else if let Some(raw) = study.get("ns").and_then(|ns| ns.get("d")) {
                        if let Some(payload) = parse_study_payload(raw) {
                            merge_indicator_study(&mut points, &payload, &plots);
                        }
                    }
                }
            }
            if packet_type == "study_completed" {
                ready = true;
                if !points.is_empty() {
                    break;
                }
            }
        }
    }

    session.queue(format_message(
        "remove_study",
        &[Value::from(chart_session.clone()), Value::from(study_id)],
    )).await?;
    session.queue(format_message(
        "chart_delete_session",
        &[Value::from(chart_session)],
    )).await?;
    session.close().await?;

    if points.is_empty() {
        return Err(DataError::NoDataAvailable);
    }

    let mut points: Vec<TvIndicatorPoint> = points.into_values().collect();
    points.sort_by_key(|point| point.time);
    Ok(TvIndicatorSeries {
        symbol: tv_symbol,
        indicator: spec.pine_id.clone(),
        points,
    })
}

pub async fn fetch_strategy_report(
    auth_token: &str,
    symbol: &str,
    spec: &TvIndicatorSpec,
    options: &TvChartOptions,
    proxy: Option<&str>,
    cookie: Option<&str>,
) -> DataResult<TvStrategyReport> {
    let tv_symbol = to_tv_symbol(symbol);
    let (study_type, study_inputs) = build_study_inputs(spec);
    let mut session = WsSession::connect(auth_token, proxy, cookie).await?;
    let chart_session = gen_session_id("cs");
    let study_id = gen_session_id("st");
    let symbol_json = chart_symbol_json(&tv_symbol, options);

    session.queue(format_message(
        "chart_create_session",
        &[Value::from(chart_session.clone()), Value::from("")],
    )).await?;
    session.queue(format_message(
        "resolve_symbol",
        &[
            Value::from(chart_session.clone()),
            Value::from(CHART_SYMBOL_ALIAS),
            Value::from(format!("={symbol_json}")),
        ],
    )).await?;
    session.queue(format_message(
        "create_series",
        &[
            Value::from(chart_session.clone()),
            Value::from(CHART_SERIES_ID),
            Value::from(CHART_SERIES_KEY),
            Value::from(CHART_SYMBOL_ALIAS),
            Value::from(options.timeframe.as_api_str()),
            Value::from(options.range),
        ],
    )).await?;
    session
        .queue(format_message(
            "switch_timezone",
            &[Value::from(chart_session.clone()), Value::from("exchange")],
        ))
        .await?;
    session.queue(format_message(
        "create_study",
        &create_study_params(
            &chart_session,
            &study_id,
            &study_type,
            study_inputs,
        ),
    )).await?;

    let deadline = tokio::time::Instant::now() + WS_TIMEOUT;
    let mut report = TvStrategyReport {
        symbol: tv_symbol.clone(),
        indicator: spec.pine_id.clone(),
        currency: None,
        trades: Vec::new(),
        performance: TvStrategyPerformance {
            net_profit: None,
            net_profit_percent: None,
            gross_profit: None,
            gross_loss: None,
            total_trades: None,
            winning_trades: None,
            losing_trades: None,
            percent_profitable: None,
            profit_factor: None,
            max_drawdown: None,
            max_drawdown_percent: None,
            sharpe_ratio: None,
            sortino_ratio: None,
        },
        equity: Vec::new(),
        drawdown: Vec::new(),
    };

    while !strategy_report_ready(&report) && tokio::time::Instant::now() < deadline {
        if let Some(packet) = session.recv_until(deadline).await? {
            let packet_type = packet.get("m").and_then(Value::as_str).unwrap_or("");
            let params = packet.get("p").and_then(Value::as_array).cloned().unwrap_or_default();
            if let Some(error) = chart_error_message(packet_type, &params) {
                return Err(DataError::custom(error));
            }
            if matches!(packet_type, "timescale_update" | "du" | "study_completed") && params.len() > 1 {
                for study in study_updates_from_packet(&params, &study_id) {
                    if let Some(raw) = study.get("ns").and_then(|ns| ns.get("d")) {
                        if let Some(text) = raw.as_str() {
                            if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                                ingest_strategy_payload(&mut report, &parsed);
                            }
                        } else if let Some(payload) = parse_study_payload(raw) {
                            ingest_strategy_payload(&mut report, &payload);
                        }
                    }
                }
            }
            if packet_type == "study_completed" && strategy_report_ready(&report) {
                break;
            }
        }
    }

    session.close().await?;
    if !strategy_report_ready(&report) {
        return Err(DataError::NoDataAvailable);
    }
    Ok(report)
}

pub async fn fetch_replay(
    auth_token: &str,
    symbol: &str,
    replay_from: i64,
    steps: u32,
    options: &TvChartOptions,
    proxy: Option<&str>,
    cookie: Option<&str>,
) -> DataResult<TvReplayResult> {
    let mut replay_options = options.clone();
    replay_options.replay_from = Some(replay_from);
    let (mut bars, _) = fetch_chart(auth_token, symbol, &replay_options, proxy, cookie).await?;
    if steps > 1 {
        bars.truncate(steps as usize);
    }
    Ok(TvReplayResult {
        symbol: to_tv_symbol(symbol),
        replay_end: bars.len() < options.range,
        bars,
    })
}

fn parse_proxy_endpoint(proxy_url: &str) -> DataResult<(String, u16)> {
    let trimmed = proxy_url
        .strip_prefix("http://")
        .or_else(|| proxy_url.strip_prefix("https://"))
        .unwrap_or(proxy_url);
    if let Some((host, port)) = trimmed.rsplit_once(':') {
        let port = port
            .parse::<u16>()
            .map_err(|_| DataError::custom(format!("invalid proxy port in '{proxy_url}'")))?;
        Ok((host.to_string(), port))
    } else {
        Ok((trimmed.to_string(), 8080))
    }
}

async fn tcp_via_http_proxy(proxy_url: &str, host: &str, port: u16) -> DataResult<TcpStream> {
    let (proxy_host, proxy_port) = parse_proxy_endpoint(proxy_url)?;
    let mut stream = TcpStream::connect((proxy_host.as_str(), proxy_port))
        .await
        .map_err(|e| DataError::custom(format!("proxy connect: {e}")))?;
    let request = format!(
        "CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\nProxy-Connection: Keep-Alive\r\nConnection: Keep-Alive\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .await
        .map_err(|e| DataError::custom(format!("proxy write: {e}")))?;

    let mut response = Vec::with_capacity(512);
    let mut chunk = [0u8; 128];
    loop {
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|e| DataError::custom(format!("proxy read: {e}")))?;
        if read == 0 {
            break;
        }
        response.extend_from_slice(&chunk[..read]);
        if response.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if response.len() >= 4096 {
            return Err(DataError::custom("proxy response too large"));
        }
    }
    let status = std::str::from_utf8(&response)
        .map_err(|e| DataError::custom(e.to_string()))?
        .lines()
        .next()
        .unwrap_or_default();
    if !status.contains(" 200 ") {
        return Err(DataError::custom(format!("proxy CONNECT failed: {status}")));
    }
    Ok(stream)
}

async fn connect_tcp(proxy: Option<&str>) -> DataResult<TcpStream> {
    if let Some(proxy_url) = proxy {
        tcp_via_http_proxy(proxy_url, WS_HOST, WS_PORT).await
    } else {
        TcpStream::connect((WS_HOST, WS_PORT))
            .await
            .map_err(|e| DataError::custom(format!("tcp connect: {e}")))
    }
}

fn ws_client_request(cookie: Option<&str>) -> DataResult<Request<()>> {
    let mut request = WS_URL
        .into_client_request()
        .map_err(|e| DataError::custom(e.to_string()))?;
    let headers = request.headers_mut();
    headers.insert(header::ORIGIN, header::HeaderValue::from_static(TV_ORIGIN));
    headers.insert(header::REFERER, header::HeaderValue::from_static(TV_ORIGIN));
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static(TV_USER_AGENT),
    );
    if let Some(cookie) = cookie.filter(|value| !value.is_empty()) {
        headers.insert(
            header::COOKIE,
            header::HeaderValue::from_str(cookie)
                .map_err(|e| DataError::custom(e.to_string()))?,
        );
    }
    Ok(request)
}

async fn tls_connect(tcp: TcpStream) -> DataResult<MaybeTlsStream<TcpStream>> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));
    let domain = ServerName::try_from(WS_HOST)
        .map_err(|e| DataError::custom(format!("invalid websocket host: {e}")))?;
    connector
        .connect(domain, tcp)
        .await
        .map(|stream| MaybeTlsStream::Rustls(stream))
        .map_err(|e| DataError::custom(format!("tls connect: {e}")))
}

struct WsSession {
    sink: futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        Message,
    >,
    stream: futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    >,
    logged: bool,
    pending: Vec<String>,
    inbox: Vec<Value>,
}

impl WsSession {
    async fn connect(auth_token: &str, proxy: Option<&str>, cookie: Option<&str>) -> DataResult<Self> {
        let request = ws_client_request(cookie)?;
        let (ws, _) = if proxy.is_some() {
            let tcp = connect_tcp(proxy).await?;
            let tls = tls_connect(tcp).await?;
            client_async(request, tls)
                .await
                .map_err(|e| DataError::custom(format!("websocket connect: {e}")))?
        } else {
            connect_async(request)
                .await
                .map_err(|e| DataError::custom(format!("websocket connect: {e}")))?
        };
        let (sink, stream) = ws.split();
        let mut session = Self {
            sink,
            stream,
            logged: false,
            pending: vec![
                format_message("set_auth_token", &[Value::from(auth_token)]),
                format_message("set_locale", &[Value::from("en"), Value::from("US")]),
            ],
            inbox: Vec::new(),
        };
        session.flush_pending().await?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        while !session.logged && tokio::time::Instant::now() < deadline {
            if let Some(packet) = session.recv_until(deadline).await? {
                if packet.get("m").is_none() {
                    session.logged = true;
                    session.flush_pending().await?;
                    break;
                }
            }
        }
        if !session.logged {
            return Err(DataError::custom("TradingView websocket auth timeout"));
        }
        Ok(session)
    }

    async fn queue(&mut self, packet: String) -> DataResult<()> {
        self.pending.push(packet);
        if self.logged {
            self.flush_pending().await?;
        }
        Ok(())
    }

    async fn flush_pending(&mut self) -> DataResult<()> {
        let pending = std::mem::take(&mut self.pending);
        for packet in pending {
            self.sink
                .send(Message::Text(packet.into()))
                .await
                .map_err(|e| DataError::custom(format!("websocket send: {e}")))?;
        }
        Ok(())
    }

    async fn recv_until(&mut self, deadline: tokio::time::Instant) -> DataResult<Option<Value>> {
        loop {
            if !self.inbox.is_empty() {
                return Ok(Some(self.inbox.remove(0)));
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Ok(None);
            }
            let message = match timeout(remaining, self.stream.next()).await {
                Ok(message) => message,
                Err(_) => return Ok(None),
            };
            let Some(message) = message else {
                return Ok(None);
            };
            let message = message.map_err(|e| DataError::custom(format!("websocket recv: {e}")))?;
            match message {
                Message::Text(text) => {
                    self.ingest_packets(parse_packets(&text)).await?;
                }
                Message::Ping(payload) => {
                    self.sink
                        .send(Message::Pong(payload))
                        .await
                        .map_err(|e| DataError::custom(format!("websocket pong: {e}")))?;
                }
                Message::Pong(_) | Message::Close(_) | Message::Frame(_) => {}
                Message::Binary(data) => {
                    if let Ok(text) = String::from_utf8(data.to_vec()) {
                        self.ingest_packets(parse_packets(&text)).await?;
                    }
                }
            }
            if !self.inbox.is_empty() {
                return Ok(Some(self.inbox.remove(0)));
            }
        }
    }

    async fn ingest_packets(&mut self, packets: Vec<Value>) -> DataResult<()> {
        for packet in packets {
            if packet.is_number() {
                let seq = packet.as_i64().unwrap_or(0);
                self.sink
                    .send(Message::Text(format_ping_response(seq).into()))
                    .await
                    .map_err(|e| DataError::custom(format!("websocket send: {e}")))?;
                continue;
            }
            if !self.logged && packet.get("m").is_none() {
                self.logged = true;
                self.flush_pending().await?;
            }
            if packet.get("m").is_some() {
                self.inbox.push(packet);
            }
        }
        Ok(())
    }

    async fn close(mut self) -> DataResult<()> {
        let _ = self.sink.close().await;
        Ok(())
    }
}

fn study_update_payload<'a>(params: &'a [Value], study_id: &str) -> Option<&'a Value> {
    if params.len() < 2 {
        return None;
    }
    if params[1].as_str() == Some(study_id) {
        return params.get(2);
    }
    params[1]
        .as_object()
        .and_then(|map| map.get(study_id))
}

fn study_updates_from_packet<'a>(params: &'a [Value], study_id: &str) -> Vec<&'a Value> {
    if let Some(study) = study_update_payload(params, study_id) {
        return vec![study];
    }
    let Some(map) = params.get(1).and_then(Value::as_object) else {
        return Vec::new();
    };
    map.iter()
        .filter_map(|(key, value)| {
            if key == CHART_SERIES_ID || key == "$prices" {
                return None;
            }
            if key == study_id || key.starts_with("st") {
                Some(value)
            } else {
                None
            }
        })
        .collect()
}

fn parse_study_payload(raw: &Value) -> Option<Value> {
    match raw {
        Value::String(text) => {
            if let Ok(value) = serde_json::from_str(text) {
                return Some(value);
            }
            parse_compressed(text).ok()
        }
        Value::Object(_) | Value::Array(_) => Some(raw.clone()),
        _ => None,
    }
}

fn study_payload_from_update(study: &Value) -> Option<Value> {
    if study.get("st").is_some() {
        return Some(study.clone());
    }
    study
        .get("ns")
        .and_then(|ns| ns.get("d"))
        .and_then(parse_study_payload)
}

fn merge_indicator_study(
    points: &mut HashMap<i64, TvIndicatorPoint>,
    payload: &Value,
    plots: &HashMap<String, String>,
) {
    let st = payload
        .get("st")
        .and_then(Value::as_array)
        .or_else(|| payload.as_array());
    let Some(st) = st else {
        return;
    };
    for item in st {
        let values = item
            .get("v")
            .and_then(Value::as_array)
            .cloned()
            .or_else(|| item.as_array().cloned())
            .unwrap_or_default();
        if values.is_empty() {
            continue;
        }
        let Some(time) = value_as_i64(&values[0]) else {
            continue;
        };
        let mut map = HashMap::new();
        for (index, value) in values.iter().skip(1).enumerate() {
            let plot_key = format!("plot_{index}");
            let key = plots
                .get(&plot_key)
                .or_else(|| plots.values().nth(index))
                .cloned()
                .unwrap_or(plot_key);
            if let Some(number) = value_as_f64(value) {
                map.insert(key, number);
            }
        }
        points.insert(time, TvIndicatorPoint { time, values: map });
    }
}

fn ingest_strategy_payload(report: &mut TvStrategyReport, payload: &Value) {
    if let Some(compressed) = payload.get("dataCompressed").and_then(Value::as_str) {
        if let Ok(value) = parse_compressed(compressed) {
            if let Some(report_data) = value.get("report") {
                merge_strategy_report(report, &json!({ "report": report_data }));
            } else {
                merge_strategy_report(report, &value);
            }
            return;
        }
    }
    if let Some(report_data) = payload.get("data").and_then(|data| data.get("report")) {
        merge_strategy_report(report, &json!({ "report": report_data }));
        return;
    }
    if payload.get("report").is_some() {
        merge_strategy_report(report, payload);
    }
}

fn merge_bars(bars: &mut HashMap<i64, TvChartBar>, prices: &Value) {
    if let Some(items) = prices.get("s").and_then(Value::as_array) {
        let mut ordered: Vec<i64> = bars.keys().copied().collect();
        ordered.sort_unstable();
        for item in items {
            if let Some(values) = item.get("v").and_then(Value::as_array) {
                if values.len() >= 6 {
                    if let Some(bar) = bar_from_values(values) {
                        bars.insert(bar.time, bar);
                        continue;
                    }
                }
            }
            if let Some(index) = item.get("i").and_then(value_as_i64) {
                let idx = index as usize;
                if idx < ordered.len() {
                    let time = ordered[idx];
                    if let Some(bar) = bars.get_mut(&time) {
                        if let Some(values) = item.get("v").and_then(Value::as_array) {
                            apply_bar_values(bar, values, Some(time));
                        }
                    }
                }
            }
        }
    }
}

fn bar_from_values(values: &[Value]) -> Option<TvChartBar> {
    if values.len() < 6 {
        return None;
    }
    let time = value_as_i64(&values[0]).unwrap_or(0);
    Some(TvChartBar {
        time,
        open: value_as_f64(&values[1]).unwrap_or(0.0),
        high: value_as_f64(&values[2]).unwrap_or(0.0),
        low: value_as_f64(&values[3]).unwrap_or(0.0),
        close: value_as_f64(&values[4]).unwrap_or(0.0),
        volume: value_as_f64(&values[5]).unwrap_or(0.0),
    })
}

fn apply_bar_values(bar: &mut TvChartBar, values: &[Value], time_hint: Option<i64>) {
    if values.is_empty() {
        return;
    }
    if values.len() >= 6 {
        if let Some(time) = value_as_i64(&values[0]) {
            bar.time = time;
        } else if let Some(time) = time_hint {
            bar.time = time;
        }
        bar.open = value_as_f64(&values[1]).unwrap_or(bar.open);
        bar.high = value_as_f64(&values[2]).unwrap_or(bar.high);
        bar.low = value_as_f64(&values[3]).unwrap_or(bar.low);
        bar.close = value_as_f64(&values[4]).unwrap_or(bar.close);
        bar.volume = value_as_f64(&values[5]).unwrap_or(bar.volume);
    }
}

fn chart_symbol_json(tv_symbol: &str, options: &TvChartOptions) -> Value {
    let mut obj = json!({
        "symbol": tv_symbol,
        "adjustment": options.adjustment.clone().unwrap_or_else(|| "splits".into()),
    });
    if let Some(session_name) = &options.session {
        obj["session"] = Value::from(session_name.clone());
    }
    if let Some(currency) = &options.currency {
        obj["currency-id"] = Value::from(currency.clone());
    }
    obj
}

fn chart_error_message(packet_type: &str, params: &[Value]) -> Option<String> {
    match packet_type {
        "symbol_error" => Some(format!(
            "TradingView symbol error: {}",
            params.get(2).and_then(Value::as_str).unwrap_or("unknown")
        )),
        "series_error" => Some(format!(
            "TradingView series error: {}",
            params.get(3).and_then(Value::as_str).unwrap_or("unknown")
        )),
        "study_error" => {
            let details: Vec<String> = params
                .iter()
                .skip(2)
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect();
            let message = if details.is_empty() {
                "unknown".to_string()
            } else {
                details.join(" | ")
            };
            Some(format!("TradingView study error: {message}"))
        }
        "critical_error" => {
            let name = params.get(1).and_then(Value::as_str).unwrap_or("unknown");
            let description = params
                .get(2)
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            Some(format!("TradingView critical error: {name} | {description}"))
        }
        _ => None,
    }
}

fn strategy_report_ready(report: &TvStrategyReport) -> bool {
    !report.trades.is_empty()
        || report.performance.total_trades.is_some()
        || report.performance.net_profit.is_some()
        || !report.equity.is_empty()
        || !report.drawdown.is_empty()
}

fn find_quote_symbol(keys: &[(String, String)], incoming_key: &str) -> Option<String> {
    keys.iter()
        .find(|(symbol, stored)| stored == incoming_key || incoming_key.contains(symbol))
        .map(|(symbol, _)| symbol.clone())
}

fn map_market_info(symbol: &str, meta: &serde_json::Map<String, Value>) -> TvMarketInfo {
    TvMarketInfo {
        full_name: meta
            .get("full_name")
            .and_then(Value::as_str)
            .unwrap_or(symbol)
            .to_string(),
        description: meta
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or(symbol)
            .to_string(),
        exchange: meta
            .get("exchange")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        currency: meta
            .get("currency_code")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        asset_type: meta
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        timezone: meta
            .get("timezone")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        has_intraday: meta.get("has_intraday").and_then(Value::as_bool).unwrap_or(false),
        is_replayable: meta.get("is_replayable").and_then(Value::as_bool).unwrap_or(false),
    }
}

fn map_quote(symbol: &str, values: HashMap<String, Value>) -> TvQuote {
    TvQuote {
        symbol: symbol.to_string(),
        last_price: values.get("lp").and_then(value_as_f64),
        change: values.get("ch").and_then(value_as_f64),
        change_percent: values.get("chp").and_then(value_as_f64),
        volume: values.get("volume").and_then(value_as_f64),
        open: values.get("open_price").and_then(value_as_f64),
        high: values.get("high_price").and_then(value_as_f64),
        low: values.get("low_price").and_then(value_as_f64),
        prev_close: values.get("prev_close_price").and_then(value_as_f64),
        bid: values.get("bid").and_then(value_as_f64),
        ask: values.get("ask").and_then(value_as_f64),
        description: values
            .get("description")
            .and_then(|v| v.as_str().map(str::to_string)),
        exchange: values
            .get("exchange")
            .and_then(|v| v.as_str().map(str::to_string)),
        currency: values
            .get("currency_code")
            .and_then(|v| v.as_str().map(str::to_string)),
        market_cap: values.get("market_cap_basic").and_then(value_as_f64),
        pe_ratio: values.get("price_earnings_ttm").and_then(value_as_f64),
        sector: values.get("sector").and_then(|v| v.as_str().map(str::to_string)),
        raw: values,
    }
}

fn merge_strategy_report(report: &mut TvStrategyReport, payload: &Value) {
    let report_data = payload.get("report").unwrap_or(payload);
    if let Some(currency) = report_data.get("currency").and_then(Value::as_str) {
        report.currency = Some(currency.to_string());
    }
    if let Some(performance) = report_data.get("performance") {
        let all = performance.get("all").unwrap_or(performance);
        report.performance = TvStrategyPerformance {
            net_profit: all.get("netProfit").and_then(value_as_f64),
            net_profit_percent: all.get("netProfitPercent").and_then(value_as_f64),
            gross_profit: all.get("grossProfit").and_then(value_as_f64),
            gross_loss: all.get("grossLoss").and_then(value_as_f64),
            total_trades: all.get("totalTrades").and_then(Value::as_i64),
            winning_trades: all.get("numberOfWiningTrades").and_then(Value::as_i64),
            losing_trades: all.get("numberOfLosingTrades").and_then(Value::as_i64),
            percent_profitable: all.get("percentProfitable").and_then(value_as_f64),
            profit_factor: all.get("profitFactor").and_then(value_as_f64),
            max_drawdown: performance
                .get("maxDrawDown")
                .and_then(value_as_f64)
                .or_else(|| all.get("maxDrawDown").and_then(value_as_f64)),
            max_drawdown_percent: performance
                .get("maxDrawDownPercent")
                .and_then(value_as_f64)
                .or_else(|| all.get("maxDrawDownPercent").and_then(value_as_f64)),
            sharpe_ratio: performance
                .get("sharpeRatio")
                .and_then(value_as_f64)
                .or_else(|| all.get("sharpeRatio").and_then(value_as_f64)),
            sortino_ratio: performance
                .get("sortinoRatio")
                .and_then(value_as_f64)
                .or_else(|| all.get("sortinoRatio").and_then(value_as_f64)),
        };
    }
    if let Some(trades) = report_data.get("trades").and_then(Value::as_array) {
        report.trades = trades
            .iter()
            .rev()
            .filter_map(|trade| {
                Some(TvTradeReport {
                    entry_type: if trade.get("e")?.get("tp")?.as_str()?.starts_with('s') {
                        "short".into()
                    } else {
                        "long".into()
                    },
                    entry_price: trade.get("e")?.get("p")?.as_f64()?,
                    entry_time: trade.get("e")?.get("tm").and_then(value_as_i64)?,
                    exit_price: trade.get("x")?.get("p")?.as_f64()?,
                    exit_time: trade.get("x")?.get("tm").and_then(value_as_i64)?,
                    quantity: trade.get("q")?.as_f64()?,
                    profit: trade.get("tp")?.as_f64()?,
                    cumulative: trade.get("cp")?.as_f64()?,
                })
            })
            .collect();
    }
    if let Some(equity) = report_data.get("equity").and_then(Value::as_array) {
        report.equity = equity.iter().filter_map(value_as_f64).collect();
    }
    if let Some(drawdown) = report_data.get("drawDown").and_then(Value::as_array) {
        report.drawdown = drawdown.iter().filter_map(value_as_f64).collect();
    }
}

fn parse_estimate_series(value: Option<&Value>) -> TvEstimateSeries {
    let Some(value) = value else {
        return TvEstimateSeries::default();
    };
    let Some(items) = value.as_array() else {
        return TvEstimateSeries::default();
    };
    if items.len() == 2 {
        let periods = items[0].as_array().cloned().unwrap_or_default();
        let values = items[1].as_array().cloned().unwrap_or_default();
        if periods.iter().all(|period| period.is_string())
            && values.iter().all(|value| value.is_number())
        {
            let points = periods
                .into_iter()
                .zip(values)
                .filter_map(|(period, value)| {
                    Some(TvEstimatePoint {
                        period: period.as_str()?.to_string(),
                        value: value_as_f64(&value)?,
                    })
                })
                .collect();
            return TvEstimateSeries { points };
        }
    }
    let points = items
        .iter()
        .filter_map(|item| {
            let pair = item.as_array()?;
            if pair.len() < 2 {
                return None;
            }
            Some(TvEstimatePoint {
                period: pair[0].as_str()?.to_string(),
                value: value_as_f64(&pair[1])?,
            })
        })
        .collect();
    TvEstimateSeries { points }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_estimate_series_parallel_arrays() {
        let value = json!([["2025-Q1", "2025-Q2"], [1.2, 1.4]]);
        let series = parse_estimate_series(Some(&value));
        assert_eq!(series.points.len(), 2);
        assert_eq!(series.points[0].period, "2025-Q1");
        assert!((series.points[0].value - 1.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_estimate_series_pairs() {
        let value = json!([["2025-Q1", 1.2], ["2025-Q2", 1.4]]);
        let series = parse_estimate_series(Some(&value));
        assert_eq!(series.points.len(), 2);
    }
}
