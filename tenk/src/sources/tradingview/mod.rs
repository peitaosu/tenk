mod analyst;
mod convert;
mod market;
mod news;
mod pine_perm;
mod protocol;
mod rest;
mod study;
mod symbol;
mod ws;

pub use pine_perm::TvPinePerm;

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};

use crate::data::{
    TvAssetFilter, TvCalendarEvent, TvChartBar, TvChartOptions, TvDrawing, TvHotlistKind,
    TvIndicatorMeta, TvIndicatorSeries, TvIndicatorSpec, TvMarketInfo, TvQuote, TvReplayResult,
    TvScreenerRequest, TvScreenerResult, TvStrategyReport, TvSymbolMatch, TvTechnicalAnalysis,
    TvUserSession,
};
use crate::error::{DataError, DataResult};
use crate::request::{RequestConfig, RequestManager};
use crate::traits::DataSource;

use rest::TvRestClient;
use symbol::{normalize_strategy_id, to_tv_symbol};

#[derive(Clone)]
pub struct TradingViewSource {
    rest: TvRestClient,
    auth_token: String,
    proxy: Option<String>,
}

impl TradingViewSource {
    pub fn try_new(proxy: Option<&str>) -> DataResult<Self> {
        let session = std::env::var("TENK_TV_SESSION").unwrap_or_default();
        let signature = std::env::var("TENK_TV_SIGNATURE").unwrap_or_default();
        let auth_token = std::env::var("TENK_TV_AUTH_TOKEN")
            .unwrap_or_else(|_| "unauthorized_user_token".to_string());

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
            ),
        );

        let config = RequestConfig::default()
            .with_proxy_opt(proxy)
            .with_headers(headers);
        let http = RequestManager::new(config)?;
        Ok(Self {
            rest: TvRestClient::new(http, session, signature),
            auth_token,
            proxy: proxy.map(str::to_string),
        })
    }

    pub async fn resolve_auth_token(&self) -> DataResult<String> {
        if self.auth_token != "unauthorized_user_token" {
            return Ok(self.auth_token.clone());
        }
        if self.rest.session.is_empty() {
            return Ok(self.auth_token.clone());
        }
        match self.rest.fetch_session_auth_token().await {
            Ok(token) if !token.is_empty() => Ok(token),
            Ok(_) => Ok(self.auth_token.clone()),
            Err(_) => Ok(self.auth_token.clone()),
        }
    }

    pub fn pine_perm(&self, pine_id: &str) -> DataResult<TvPinePerm<'_>> {
        TvPinePerm::new(&self.rest, pine_id)
    }

    pub async fn search_symbols(
        &self,
        query: &str,
        filter: Option<TvAssetFilter>,
        offset: u32,
    ) -> DataResult<Vec<TvSymbolMatch>> {
        self.rest.search_symbols(query, filter, offset).await
    }

    pub async fn technical_analysis(&self, symbol: &str) -> DataResult<TvTechnicalAnalysis> {
        self.rest.technical_analysis(symbol).await
    }

    pub async fn screener(&self, request: &TvScreenerRequest) -> DataResult<TvScreenerResult> {
        self.rest.screener(request).await
    }

    pub async fn hotlist(
        &self,
        market: &str,
        kind: TvHotlistKind,
        limit: usize,
    ) -> DataResult<TvScreenerResult> {
        self.rest.hotlist(market, kind, limit).await
    }

    pub async fn calendar(
        &self,
        from: &str,
        to: &str,
        countries: &str,
    ) -> DataResult<Vec<TvCalendarEvent>> {
        self.rest.calendar(from, to, countries).await
    }

    pub async fn search_indicators(&self, query: &str) -> DataResult<Vec<TvIndicatorMeta>> {
        self.rest.search_indicators(query).await
    }

    pub async fn get_indicator(&self, id: &str, version: &str) -> DataResult<TvIndicatorSpec> {
        self.rest.get_indicator(id, version).await
    }

    pub async fn private_indicators(&self) -> DataResult<Vec<TvIndicatorMeta>> {
        if self.rest.session.is_empty() {
            return Err(DataError::custom("TradingView session required"));
        }
        self.rest.private_indicators().await
    }

    pub async fn login(&self, username: &str, password: &str) -> DataResult<TvUserSession> {
        self.rest.login(username, password).await
    }

    pub async fn get_drawings(
        &self,
        layout: &str,
        symbol: &str,
        user_id: i64,
    ) -> DataResult<Vec<TvDrawing>> {
        if self.rest.session.is_empty() {
            return Err(DataError::custom(
                "TradingView session required for chart drawings: set TENK_TV_SESSION and TENK_TV_SIGNATURE",
            ));
        }
        self.rest.get_drawings(layout, symbol, user_id).await
    }

    pub async fn quote(&self, symbols: &[&str]) -> DataResult<Vec<TvQuote>> {
        let token = self.resolve_auth_token().await?;
        let list = symbols
            .iter()
            .map(|symbol| to_tv_symbol(symbol))
            .collect::<Vec<_>>();
        ws::fetch_quotes(
            &token,
            &list,
            self.proxy.as_deref(),
            self.rest.ws_cookie().as_deref(),
        )
        .await
    }

    pub async fn chart(
        &self,
        symbol: &str,
        options: &TvChartOptions,
    ) -> DataResult<(Vec<TvChartBar>, TvMarketInfo)> {
        let token = self.resolve_auth_token().await?;
        ws::fetch_chart(
            &token,
            symbol,
            options,
            self.proxy.as_deref(),
            self.rest.ws_cookie().as_deref(),
        )
        .await
    }

    pub async fn indicator_series(
        &self,
        symbol: &str,
        indicator_id: &str,
        version: &str,
        options: &TvChartOptions,
    ) -> DataResult<TvIndicatorSeries> {
        let resolved_id = symbol::resolve_study_id(indicator_id);
        let spec = self.get_indicator(resolved_id, version).await?;
        let token = self.resolve_auth_token().await?;
        let mut series = ws::fetch_indicator_series(
            &token,
            symbol,
            &spec,
            options,
            self.proxy.as_deref(),
            self.rest.ws_cookie().as_deref(),
        )
        .await?;
        series.indicator = indicator_id.to_string();
        Ok(series)
    }

    pub async fn strategy_report(
        &self,
        symbol: &str,
        indicator_id: &str,
        version: &str,
        options: &TvChartOptions,
    ) -> DataResult<TvStrategyReport> {
        let strategy_id = normalize_strategy_id(indicator_id);
        let spec = self.get_indicator(&strategy_id, version).await?;
        let token = self.resolve_auth_token().await?;
        ws::fetch_strategy_report(
            &token,
            symbol,
            &spec,
            options,
            self.proxy.as_deref(),
            self.rest.ws_cookie().as_deref(),
        )
        .await
    }

    pub async fn replay(
        &self,
        symbol: &str,
        replay_from: i64,
        steps: u32,
        options: &TvChartOptions,
    ) -> DataResult<TvReplayResult> {
        let token = self.resolve_auth_token().await?;
        ws::fetch_replay(
            &token,
            symbol,
            replay_from,
            steps,
            options,
            self.proxy.as_deref(),
            self.rest.ws_cookie().as_deref(),
        )
        .await
    }
}

#[async_trait]
impl DataSource for TradingViewSource {
    fn name(&self) -> &'static str {
        "tradingview"
    }

    fn priority(&self) -> u8 {
        4
    }
}

