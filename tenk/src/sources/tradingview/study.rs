use async_trait::async_trait;

use crate::data::{
    TvAssetFilter, TvCalendarEvent, TvChartOptions, TvDrawing, TvHotlistKind, TvIndicatorMeta,
    TvIndicatorSeries, TvIndicatorSpec, TvReplayResult, TvScreenerRequest, TvScreenerResult,
    TvStrategyReport, TvSymbolMatch, TvTechnicalAnalysis,
};
use crate::error::DataResult;
use crate::traits::{
    EconomicCalendarSource, ScreenerSource, StudySource, SymbolSearchSource,
    TechnicalAnalysisSource,
};

use super::TradingViewSource;

#[async_trait]
impl TechnicalAnalysisSource for TradingViewSource {
    async fn get_technical_analysis(&self, symbol: &str) -> DataResult<TvTechnicalAnalysis> {
        self.technical_analysis(symbol).await
    }
}

#[async_trait]
impl SymbolSearchSource for TradingViewSource {
    async fn search_symbols(
        &self,
        query: &str,
        filter: Option<TvAssetFilter>,
        offset: u32,
    ) -> DataResult<Vec<TvSymbolMatch>> {
        self.search_symbols(query, filter, offset).await
    }
}

#[async_trait]
impl ScreenerSource for TradingViewSource {
    async fn run_screener(&self, request: &TvScreenerRequest) -> DataResult<TvScreenerResult> {
        self.screener(request).await
    }

    async fn get_hotlist(
        &self,
        market: &str,
        kind: TvHotlistKind,
        limit: usize,
    ) -> DataResult<TvScreenerResult> {
        self.hotlist(market, kind, limit).await
    }
}

#[async_trait]
impl EconomicCalendarSource for TradingViewSource {
    async fn get_economic_calendar(
        &self,
        from: &str,
        to: &str,
        countries: &str,
    ) -> DataResult<Vec<TvCalendarEvent>> {
        self.calendar(from, to, countries).await
    }
}

#[async_trait]
impl StudySource for TradingViewSource {
    async fn search_indicators(&self, query: &str) -> DataResult<Vec<TvIndicatorMeta>> {
        self.search_indicators(query).await
    }

    async fn get_indicator_spec(&self, id: &str, version: &str) -> DataResult<TvIndicatorSpec> {
        self.get_indicator(id, version).await
    }

    async fn get_indicator_series(
        &self,
        symbol: &str,
        id: &str,
        version: &str,
        options: &TvChartOptions,
    ) -> DataResult<TvIndicatorSeries> {
        self.indicator_series(symbol, id, version, options).await
    }

    async fn get_strategy_report(
        &self,
        symbol: &str,
        id: &str,
        version: &str,
        options: &TvChartOptions,
    ) -> DataResult<TvStrategyReport> {
        self.strategy_report(symbol, id, version, options).await
    }

    async fn get_chart_replay(
        &self,
        symbol: &str,
        replay_from: i64,
        steps: u32,
        options: &TvChartOptions,
    ) -> DataResult<TvReplayResult> {
        self.replay(symbol, replay_from, steps, options).await
    }

    async fn get_chart_drawings(
        &self,
        layout: &str,
        symbol: &str,
        user_id: i64,
    ) -> DataResult<Vec<TvDrawing>> {
        self.get_drawings(layout, symbol, user_id).await
    }
}
