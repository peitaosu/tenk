//! Data client with multi-source fallback.

use std::sync::Arc;

use tracing::info;

use crate::data::{
    BillboardDetail, BillboardItem, BlockTradeData, BondCurrentData, BoardCrosswalkItem,
    BoardCrosswalkKind, BoardItem, CapitalFlowData,
    CapitalFlowHistory, ConvertibleBondCode, CurrentMarketData, DerivativesQuote, DividendData,
    ETFCode, ETFCurrentData, ETFMarketData, ETFMinuteData, EarningsForecast, FinancialRecord,
    FinancialReportKind, FundHolding, FuturesContract, IndexCode, IPOData,
    InstitutionalResearchData, KLineType, LimitPoolItem, LimitPoolKind, MacroRecord,
    MarginTradingData, MarketData, MinuteData, NewsArticle, NewsCategory, NewsContent,
    OptionContract, OptionExchange, OrderBookData, ResearchReportData, StockCode,
    StockConnectData, StockInfo, StockValuation, TickData, TopHolder,
};
use crate::error::DataResult;
use crate::traits::{
    BillboardSource, BlockTradeSource, BoardMarketSource, BondInfoSource, BondMarketSource,
    CapitalFlowSource, DividendSource, EarningsForecastSource, FinancialSource,
    FundInfoSource, FundMarketSource, FuturesSource, GlobalMarketSource, HoldingsSource,
    IndexMarketSource, InstitutionalResearchSource, IPOSource, LimitPoolSource, MacroSource,
    MarginTradingSource, NewsSource, OptionsSource, ResearchReportSource, StockConnectSource,
    StockInfoSource, StockMarketSource, ValuationSource,
};

macro_rules! add_source {
    ($self:expr, $field:ident, $source:expr) => {{
        let mut client = $self;
        client.$field.push(Arc::new($source));
        client.$field.sort_by_key(|s| s.priority());
        client
    }};
}

macro_rules! try_sources_vec {
    ($sources:expr, $msg:expr, |$src:ident| $call:expr) => {{
        use $crate::error::DataError;
        use tracing::{debug, info, warn};

        if $sources.is_empty() {
            Err(DataError::custom($msg))
        } else {
            for $src in $sources.iter() {
                debug!("Trying source: {}", $src.name());
                if !$src.is_available().await {
                    continue;
                }
                match $call.await {
                    Ok(data) if !data.is_empty() => {
                        info!(
                            "Successfully fetched {} records from {}",
                            data.len(),
                            $src.name()
                        );
                        return Ok(data);
                    }
                    Ok(_) => {}
                    Err(error) => {
                        warn!("Source {} failed: {}", $src.name(), error);
                        if !error.is_recoverable() {
                            return Err(error);
                        }
                    }
                }
            }
            Err(DataError::NoDataAvailable)
        }
    }};
}

macro_rules! try_sources_one {
    ($sources:expr, $msg:expr, |$src:ident| $call:expr) => {{
        use $crate::error::DataError;

        if $sources.is_empty() {
            Err(DataError::custom($msg))
        } else {
            for $src in $sources.iter() {
                if !$src.is_available().await {
                    continue;
                }
                match $call.await {
                    Ok(data) => return Ok(data),
                    Err(error) if error.is_recoverable() => continue,
                    Err(error) => return Err(error),
                }
            }
            Err(DataError::NoDataAvailable)
        }
    }};
}

/// Client for fetching financial data from multiple sources.
pub struct DataClient {
    market_sources: Vec<Arc<dyn StockMarketSource>>,
    info_sources: Vec<Arc<dyn StockInfoSource>>,
    fund_info_sources: Vec<Arc<dyn FundInfoSource>>,
    fund_market_sources: Vec<Arc<dyn FundMarketSource>>,
    bond_info_sources: Vec<Arc<dyn BondInfoSource>>,
    bond_market_sources: Vec<Arc<dyn BondMarketSource>>,
    news_sources: Vec<Arc<dyn NewsSource>>,
    capital_flow_sources: Vec<Arc<dyn CapitalFlowSource>>,
    billboard_sources: Vec<Arc<dyn BillboardSource>>,
    earnings_forecast_sources: Vec<Arc<dyn EarningsForecastSource>>,
    stock_connect_sources: Vec<Arc<dyn StockConnectSource>>,
    margin_trading_sources: Vec<Arc<dyn MarginTradingSource>>,
    ipo_sources: Vec<Arc<dyn IPOSource>>,
    block_trade_sources: Vec<Arc<dyn BlockTradeSource>>,
    institutional_research_sources: Vec<Arc<dyn InstitutionalResearchSource>>,
    research_report_sources: Vec<Arc<dyn ResearchReportSource>>,
    valuation_sources: Vec<Arc<dyn ValuationSource>>,
    holdings_sources: Vec<Arc<dyn HoldingsSource>>,
    dividend_sources: Vec<Arc<dyn DividendSource>>,
    index_sources: Vec<Arc<dyn IndexMarketSource>>,
    board_sources: Vec<Arc<dyn BoardMarketSource>>,
    limit_pool_sources: Vec<Arc<dyn LimitPoolSource>>,
    macro_sources: Vec<Arc<dyn MacroSource>>,
    global_sources: Vec<Arc<dyn GlobalMarketSource>>,
    futures_sources: Vec<Arc<dyn FuturesSource>>,
    options_sources: Vec<Arc<dyn OptionsSource>>,
    financial_sources: Vec<Arc<dyn FinancialSource>>,
}

impl DataClient {
    pub fn new() -> Self {
        Self {
            market_sources: Vec::new(),
            info_sources: Vec::new(),
            fund_info_sources: Vec::new(),
            fund_market_sources: Vec::new(),
            bond_info_sources: Vec::new(),
            bond_market_sources: Vec::new(),
            news_sources: Vec::new(),
            capital_flow_sources: Vec::new(),
            billboard_sources: Vec::new(),
            earnings_forecast_sources: Vec::new(),
            stock_connect_sources: Vec::new(),
            margin_trading_sources: Vec::new(),
            ipo_sources: Vec::new(),
            block_trade_sources: Vec::new(),
            institutional_research_sources: Vec::new(),
            research_report_sources: Vec::new(),
            valuation_sources: Vec::new(),
            holdings_sources: Vec::new(),
            dividend_sources: Vec::new(),
            index_sources: Vec::new(),
            board_sources: Vec::new(),
            limit_pool_sources: Vec::new(),
            macro_sources: Vec::new(),
            global_sources: Vec::new(),
            futures_sources: Vec::new(),
            options_sources: Vec::new(),
            financial_sources: Vec::new(),
        }
    }

    pub fn with_market_source<S: StockMarketSource + 'static>(self, source: S) -> Self {
        add_source!(self, market_sources, source)
    }

    pub fn with_info_source<S: StockInfoSource + 'static>(self, source: S) -> Self {
        add_source!(self, info_sources, source)
    }

    pub fn with_source<S: StockMarketSource + StockInfoSource + Clone + 'static>(
        mut self,
        source: S,
    ) -> Self {
        self = add_source!(self, market_sources, source.clone());
        add_source!(self, info_sources, source)
    }

    pub fn with_fund_info_source<S: FundInfoSource + 'static>(self, source: S) -> Self {
        add_source!(self, fund_info_sources, source)
    }

    pub fn with_fund_market_source<S: FundMarketSource + 'static>(self, source: S) -> Self {
        add_source!(self, fund_market_sources, source)
    }

    pub fn with_bond_info_source<S: BondInfoSource + 'static>(self, source: S) -> Self {
        add_source!(self, bond_info_sources, source)
    }

    pub fn with_bond_market_source<S: BondMarketSource + 'static>(self, source: S) -> Self {
        add_source!(self, bond_market_sources, source)
    }

    pub fn with_fund_source<S: FundInfoSource + FundMarketSource + Clone + 'static>(
        mut self,
        source: S,
    ) -> Self {
        self = add_source!(self, fund_info_sources, source.clone());
        add_source!(self, fund_market_sources, source)
    }

    pub fn with_bond_source<S: BondInfoSource + BondMarketSource + Clone + 'static>(
        mut self,
        source: S,
    ) -> Self {
        self = add_source!(self, bond_info_sources, source.clone());
        add_source!(self, bond_market_sources, source)
    }

    pub fn with_news_source<S: NewsSource + 'static>(self, source: S) -> Self {
        add_source!(self, news_sources, source)
    }

    pub fn with_capital_flow_source<S: CapitalFlowSource + 'static>(self, source: S) -> Self {
        add_source!(self, capital_flow_sources, source)
    }

    pub fn with_billboard_source<S: BillboardSource + 'static>(self, source: S) -> Self {
        add_source!(self, billboard_sources, source)
    }

    pub fn with_earnings_forecast_source<S: EarningsForecastSource + 'static>(
        self,
        source: S,
    ) -> Self {
        add_source!(self, earnings_forecast_sources, source)
    }

    pub fn with_stock_connect_source<S: StockConnectSource + 'static>(self, source: S) -> Self {
        add_source!(self, stock_connect_sources, source)
    }

    pub fn with_margin_trading_source<S: MarginTradingSource + 'static>(self, source: S) -> Self {
        add_source!(self, margin_trading_sources, source)
    }

    pub fn with_ipo_source<S: IPOSource + 'static>(self, source: S) -> Self {
        add_source!(self, ipo_sources, source)
    }

    pub fn with_block_trade_source<S: BlockTradeSource + 'static>(self, source: S) -> Self {
        add_source!(self, block_trade_sources, source)
    }

    pub fn with_institutional_research_source<S: InstitutionalResearchSource + 'static>(
        self,
        source: S,
    ) -> Self {
        add_source!(self, institutional_research_sources, source)
    }

    pub fn with_research_report_source<S: ResearchReportSource + 'static>(self, source: S) -> Self {
        add_source!(self, research_report_sources, source)
    }

    pub fn with_valuation_source<S: ValuationSource + 'static>(self, source: S) -> Self {
        add_source!(self, valuation_sources, source)
    }

    pub fn with_holdings_source<S: HoldingsSource + 'static>(self, source: S) -> Self {
        add_source!(self, holdings_sources, source)
    }

    pub fn with_dividend_source<S: DividendSource + 'static>(self, source: S) -> Self {
        add_source!(self, dividend_sources, source)
    }

    pub fn with_index_source<S: IndexMarketSource + 'static>(self, source: S) -> Self {
        add_source!(self, index_sources, source)
    }

    pub fn with_board_source<S: BoardMarketSource + 'static>(self, source: S) -> Self {
        add_source!(self, board_sources, source)
    }

    pub fn with_limit_pool_source<S: LimitPoolSource + 'static>(self, source: S) -> Self {
        add_source!(self, limit_pool_sources, source)
    }

    pub fn with_macro_source<S: MacroSource + 'static>(self, source: S) -> Self {
        add_source!(self, macro_sources, source)
    }

    pub fn with_global_source<S: GlobalMarketSource + 'static>(self, source: S) -> Self {
        add_source!(self, global_sources, source)
    }

    pub fn with_futures_source<S: FuturesSource + 'static>(self, source: S) -> Self {
        add_source!(self, futures_sources, source)
    }

    pub fn with_options_source<S: OptionsSource + 'static>(self, source: S) -> Self {
        add_source!(self, options_sources, source)
    }

    pub fn with_financial_source<S: FinancialSource + 'static>(self, source: S) -> Self {
        add_source!(self, financial_sources, source)
    }

    pub fn with_extended_market<S>(mut self, source: S) -> Self
    where
        S: CapitalFlowSource
            + BillboardSource
            + EarningsForecastSource
            + StockConnectSource
            + MarginTradingSource
            + IPOSource
            + BlockTradeSource
            + InstitutionalResearchSource
            + ResearchReportSource
            + ValuationSource
            + HoldingsSource
            + DividendSource
            + IndexMarketSource
            + BoardMarketSource
            + LimitPoolSource
            + MacroSource
            + GlobalMarketSource
            + FuturesSource
            + OptionsSource
            + FinancialSource
            + Clone
            + 'static,
    {
        self = self.with_capital_flow_source(source.clone());
        self = self.with_billboard_source(source.clone());
        self = self.with_earnings_forecast_source(source.clone());
        self = self.with_stock_connect_source(source.clone());
        self = self.with_margin_trading_source(source.clone());
        self = self.with_ipo_source(source.clone());
        self = self.with_block_trade_source(source.clone());
        self = self.with_institutional_research_source(source.clone());
        self = self.with_research_report_source(source.clone());
        self = self.with_valuation_source(source.clone());
        self = self.with_holdings_source(source.clone());
        self = self.with_dividend_source(source.clone());
        self = self.with_index_source(source.clone());
        self = self.with_board_source(source.clone());
        self = self.with_limit_pool_source(source.clone());
        self = self.with_macro_source(source.clone());
        self = self.with_global_source(source.clone());
        self = self.with_futures_source(source.clone());
        self = self.with_options_source(source.clone());
        self.with_financial_source(source)
    }

    pub fn market_source_count(&self) -> usize {
        self.market_sources.len()
    }

    pub fn info_source_count(&self) -> usize {
        self.info_sources.len()
    }

    pub async fn get_market(
        &self,
        stock_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        info!("Fetching market data for {} ({k_type:?})", stock_code);
        try_sources_vec!(self.market_sources, "No market sources configured", |source| {
            source.get_market(stock_code, start_date, end_date, k_type)
        })
    }

    pub async fn get_market_current(
        &self,
        stock_codes: &[&str],
    ) -> DataResult<Vec<CurrentMarketData>> {
        if stock_codes.is_empty() {
            return Ok(Vec::new());
        }
        try_sources_vec!(
            &self.market_sources,
            "No market sources configured",
            |source| source.get_market_current(stock_codes)
        )
    }

    pub async fn get_market_min(&self, stock_code: &str) -> DataResult<Vec<MinuteData>> {
        try_sources_vec!(
            &self.market_sources,
            "No market sources configured",
            |source| source.get_market_min(stock_code)
        )
    }

    pub async fn get_order_book(&self, stock_code: &str) -> DataResult<OrderBookData> {
        try_sources_one!(
            &self.market_sources,
            "No market sources configured",
            |source| source.get_order_book(stock_code)
        )
    }

    pub async fn get_ticks(&self, stock_code: &str) -> DataResult<Vec<TickData>> {
        try_sources_vec!(
            &self.market_sources,
            "No market sources configured",
            |source| source.get_ticks(stock_code)
        )
    }

    pub async fn get_all_codes(&self, limit: Option<usize>) -> DataResult<Vec<StockCode>> {
        try_sources_vec!(
            &self.info_sources,
            "No info sources configured",
            |source| source.get_all_codes(limit)
        )
    }

    pub async fn get_stock_info(&self, stock_code: &str) -> DataResult<StockInfo> {
        try_sources_one!(
            &self.info_sources,
            "No info sources configured",
            |source| source.get_stock_info(stock_code)
        )
    }

    pub async fn get_all_etf_codes(&self, limit: Option<usize>) -> DataResult<Vec<ETFCode>> {
        try_sources_vec!(
            &self.fund_info_sources,
            "No fund info sources configured",
            |source| source.get_all_etf_codes(limit)
        )
    }

    pub async fn get_etf_market(
        &self,
        fund_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<ETFMarketData>> {
        try_sources_vec!(
            &self.fund_market_sources,
            "No fund market sources configured",
            |source| source.get_etf_market(fund_code, start_date, end_date, k_type)
        )
    }

    pub async fn get_etf_current(&self, fund_codes: &[&str]) -> DataResult<Vec<ETFCurrentData>> {
        if fund_codes.is_empty() {
            return Ok(Vec::new());
        }
        try_sources_vec!(
            &self.fund_market_sources,
            "No fund market sources configured",
            |source| source.get_etf_current(fund_codes)
        )
    }

    pub async fn get_etf_min(&self, fund_code: &str) -> DataResult<Vec<ETFMinuteData>> {
        try_sources_vec!(
            &self.fund_market_sources,
            "No fund market sources configured",
            |source| source.get_etf_min(fund_code)
        )
    }

    pub async fn get_all_bond_codes(
        &self,
        limit: Option<usize>,
    ) -> DataResult<Vec<ConvertibleBondCode>> {
        try_sources_vec!(
            &self.bond_info_sources,
            "No bond info sources configured",
            |source| source.get_all_bond_codes(limit)
        )
    }

    pub async fn get_bond_current(
        &self,
        bond_codes: Option<&[&str]>,
    ) -> DataResult<Vec<BondCurrentData>> {
        try_sources_vec!(
            &self.bond_market_sources,
            "No bond market sources configured",
            |source| source.get_bond_current(bond_codes)
        )
    }

    pub async fn get_news(
        &self,
        category: NewsCategory,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<NewsArticle>> {
        try_sources_vec!(
            &self.news_sources,
            "No news sources configured",
            |source| source.get_news(category, page, limit)
        )
    }

    pub async fn search_news(
        &self,
        keyword: &str,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<NewsArticle>> {
        try_sources_vec!(
            &self.news_sources,
            "No news sources configured",
            |source| source.search_news(keyword, page, limit)
        )
    }

    pub async fn get_news_content(&self, news_id: &str) -> DataResult<NewsContent> {
        try_sources_one!(
            &self.news_sources,
            "No news sources configured",
            |source| source.get_news_content(news_id)
        )
    }

    pub async fn get_capital_flow(&self, stock_codes: &[&str]) -> DataResult<Vec<CapitalFlowData>> {
        try_sources_vec!(
            &self.capital_flow_sources,
            "No capital flow sources configured",
            |source| source.get_capital_flow(stock_codes)
        )
    }

    pub async fn get_capital_flow_history(
        &self,
        stock_code: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<CapitalFlowHistory>> {
        try_sources_vec!(
            &self.capital_flow_sources,
            "No capital flow sources configured",
            |source| source.get_capital_flow_history(stock_code, limit)
        )
    }

    pub async fn get_billboard_list(
        &self,
        date: Option<&str>,
    ) -> DataResult<Vec<BillboardItem>> {
        try_sources_vec!(
            &self.billboard_sources,
            "No billboard sources configured",
            |source| source.get_billboard_list(date)
        )
    }

    pub async fn get_billboard_detail(
        &self,
        stock_code: &str,
        date: &str,
    ) -> DataResult<Vec<BillboardDetail>> {
        try_sources_vec!(
            &self.billboard_sources,
            "No billboard sources configured",
            |source| source.get_billboard_detail(stock_code, date)
        )
    }

    pub async fn get_earnings_forecast(
        &self,
        report_period: Option<&str>,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<EarningsForecast>> {
        try_sources_vec!(
            &self.earnings_forecast_sources,
            "No earnings forecast sources configured",
            |source| source.get_earnings_forecast(report_period, page, limit)
        )
    }

    pub async fn get_stock_connect(
        &self,
        limit: Option<usize>,
    ) -> DataResult<Vec<StockConnectData>> {
        try_sources_vec!(
            &self.stock_connect_sources,
            "No stock connect sources configured",
            |source| source.get_stock_connect(limit)
        )
    }

    pub async fn get_margin_trading(
        &self,
        stock_code: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<MarginTradingData>> {
        try_sources_vec!(
            &self.margin_trading_sources,
            "No margin trading sources configured",
            |source| source.get_margin_trading(stock_code, limit)
        )
    }

    pub async fn get_ipo_list(&self, limit: Option<usize>) -> DataResult<Vec<IPOData>> {
        try_sources_vec!(
            &self.ipo_sources,
            "No IPO sources configured",
            |source| source.get_ipo_list(limit)
        )
    }

    pub async fn get_block_trades(&self, limit: Option<usize>) -> DataResult<Vec<BlockTradeData>> {
        try_sources_vec!(
            &self.block_trade_sources,
            "No block trade sources configured",
            |source| source.get_block_trades(limit)
        )
    }

    pub async fn get_institutional_research(
        &self,
        limit: Option<usize>,
    ) -> DataResult<Vec<InstitutionalResearchData>> {
        try_sources_vec!(
            &self.institutional_research_sources,
            "No institutional research sources configured",
            |source| source.get_institutional_research(limit)
        )
    }

    pub async fn get_research_reports(
        &self,
        stock_code: Option<&str>,
        limit: Option<usize>,
    ) -> DataResult<Vec<ResearchReportData>> {
        try_sources_vec!(
            &self.research_report_sources,
            "No research report sources configured",
            |source| source.get_research_reports(stock_code, limit)
        )
    }

    pub async fn get_valuation(&self, stock_code: &str) -> DataResult<StockValuation> {
        try_sources_one!(
            &self.valuation_sources,
            "No valuation sources configured",
            |source| source.get_valuation(stock_code)
        )
    }

    pub async fn get_top_holders(&self, stock_code: &str) -> DataResult<Vec<TopHolder>> {
        try_sources_vec!(
            &self.holdings_sources,
            "No holdings sources configured",
            |source| source.get_top_holders(stock_code)
        )
    }

    pub async fn get_fund_holdings(
        &self,
        stock_code: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<FundHolding>> {
        try_sources_vec!(
            &self.holdings_sources,
            "No holdings sources configured",
            |source| source.get_fund_holdings(stock_code, limit)
        )
    }

    pub async fn get_dividends(&self, stock_code: &str) -> DataResult<Vec<DividendData>> {
        try_sources_vec!(
            &self.dividend_sources,
            "No dividend sources configured",
            |source| source.get_dividends(stock_code)
        )
    }

    pub async fn get_index_list(&self, limit: Option<usize>) -> DataResult<Vec<IndexCode>> {
        try_sources_vec!(
            &self.index_sources,
            "No index sources configured",
            |source| source.get_index_list(limit)
        )
    }

    pub async fn get_index_current(
        &self,
        index_codes: &[&str],
    ) -> DataResult<Vec<CurrentMarketData>> {
        if index_codes.is_empty() {
            return Ok(Vec::new());
        }
        try_sources_vec!(
            &self.index_sources,
            "No index sources configured",
            |source| source.get_index_current(index_codes)
        )
    }

    pub async fn get_index_market(
        &self,
        index_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        try_sources_vec!(
            &self.index_sources,
            "No index sources configured",
            |source| source.get_index_market(index_code, start_date, end_date, k_type)
        )
    }

    pub async fn get_industry_boards(&self, limit: Option<usize>) -> DataResult<Vec<BoardItem>> {
        try_sources_vec!(
            &self.board_sources,
            "No board sources configured",
            |source| source.get_industry_boards(limit)
        )
    }

    pub async fn get_concept_boards(&self, limit: Option<usize>) -> DataResult<Vec<BoardItem>> {
        try_sources_vec!(
            &self.board_sources,
            "No board sources configured",
            |source| source.get_concept_boards(limit)
        )
    }

    pub async fn get_board_market(
        &self,
        board_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        try_sources_vec!(
            &self.board_sources,
            "No board sources configured",
            |source| source.get_board_market(board_code, start_date, end_date, k_type)
        )
    }

    pub async fn get_board_constituents(
        &self,
        board_code: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<StockCode>> {
        try_sources_vec!(
            &self.board_sources,
            "No board sources configured",
            |source| source.get_board_constituents(board_code, limit)
        )
    }

    pub async fn resolve_board_crosswalk(
        &self,
        kind: BoardCrosswalkKind,
        limit: Option<usize>,
    ) -> DataResult<Vec<BoardCrosswalkItem>> {
        use std::collections::{HashMap, HashSet};

        use crate::util::normalize_board_name;

        let mut em_boards = Vec::new();
        let mut ths_boards = Vec::new();
        for source in &self.board_sources {
            if !source.is_available().await {
                continue;
            }
            let boards = match kind {
                BoardCrosswalkKind::Industry => source.get_industry_boards(limit).await,
                BoardCrosswalkKind::Concept => source.get_concept_boards(limit).await,
            };
            if let Ok(data) = boards {
                if data.is_empty() {
                    continue;
                }
                match source.name() {
                    "eastmoney" => em_boards = data,
                    "ths" => ths_boards = data,
                    _ => {}
                }
            }
        }

        let mut ths_by_name: HashMap<String, String> = HashMap::new();
        for board in &ths_boards {
            ths_by_name.insert(normalize_board_name(&board.board_name), board.board_code.clone());
        }

        let mut matched = Vec::new();
        let mut used_ths = HashSet::new();
        for em in &em_boards {
            let key = normalize_board_name(&em.board_name);
            let ths_code = ths_by_name.get(&key).cloned();
            if let Some(ref code) = ths_code {
                used_ths.insert(code.clone());
            }
            matched.push(BoardCrosswalkItem {
                board_name: em.board_name.clone(),
                eastmoney_code: Some(em.board_code.clone()),
                ths_code,
            });
        }
        for ths in &ths_boards {
            if used_ths.contains(&ths.board_code) {
                continue;
            }
            matched.push(BoardCrosswalkItem {
                board_name: ths.board_name.clone(),
                eastmoney_code: None,
                ths_code: Some(ths.board_code.clone()),
            });
        }
        Ok(matched)
    }

    pub async fn resolve_ths_board_for_eastmoney(
        &self,
        eastmoney_board_code: &str,
        candidate_limit: Option<usize>,
    ) -> DataResult<Option<BoardCrosswalkItem>> {
        use std::collections::HashSet;

        let em_members = self
            .get_board_constituents(eastmoney_board_code, Some(100))
            .await?;
        if em_members.is_empty() {
            return Ok(None);
        }
        let em_codes: HashSet<String> = em_members
            .iter()
            .map(|stock| stock.stock_code.clone())
            .collect();

        let mut ths_boards = Vec::new();
        for source in &self.board_sources {
            if source.name() != "ths" || !source.is_available().await {
                continue;
            }
            if let Ok(boards) = source.get_industry_boards(candidate_limit).await {
                if !boards.is_empty() {
                    ths_boards = boards;
                    break;
                }
            }
        }
        if ths_boards.is_empty() {
            return Ok(None);
        }

        let mut best: Option<(BoardCrosswalkItem, usize)> = None;
        for board in ths_boards {
            let ths_members = self
                .get_board_constituents(&board.board_code, Some(100))
                .await
                .unwrap_or_default();
            let overlap = ths_members
                .iter()
                .filter(|stock| em_codes.contains(&stock.stock_code))
                .count();
            if overlap >= 5 && overlap > best.as_ref().map(|(_, count)| *count).unwrap_or(0) {
                best = Some((
                    BoardCrosswalkItem {
                        board_name: board.board_name.clone(),
                        eastmoney_code: Some(eastmoney_board_code.to_string()),
                        ths_code: Some(board.board_code.clone()),
                    },
                    overlap,
                ));
            }
        }
        Ok(best.map(|(item, _)| item))
    }

    pub async fn get_limit_pool(
        &self,
        kind: LimitPoolKind,
        date: Option<&str>,
        limit: Option<usize>,
    ) -> DataResult<Vec<LimitPoolItem>> {
        try_sources_vec!(
            &self.limit_pool_sources,
            "No limit pool sources configured",
            |source| source.get_limit_pool(kind, date, limit)
        )
    }

    pub async fn get_macro_cpi(&self, limit: Option<usize>) -> DataResult<Vec<MacroRecord>> {
        try_sources_vec!(
            &self.macro_sources,
            "No macro sources configured",
            |source| source.get_macro_cpi(limit)
        )
    }

    pub async fn get_macro_gdp(&self, limit: Option<usize>) -> DataResult<Vec<MacroRecord>> {
        try_sources_vec!(
            &self.macro_sources,
            "No macro sources configured",
            |source| source.get_macro_gdp(limit)
        )
    }

    pub async fn get_hk_current(&self, codes: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        if codes.is_empty() {
            return Ok(Vec::new());
        }
        try_sources_vec!(
            &self.global_sources,
            "No global market sources configured",
            |source| source.get_hk_current(codes)
        )
    }

    pub async fn get_us_current(&self, symbols: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        if symbols.is_empty() {
            return Ok(Vec::new());
        }
        try_sources_vec!(
            &self.global_sources,
            "No global market sources configured",
            |source| source.get_us_current(symbols)
        )
    }

    pub async fn get_futures_list(&self, limit: Option<usize>) -> DataResult<Vec<FuturesContract>> {
        try_sources_vec!(
            &self.futures_sources,
            "No futures sources configured",
            |source| source.get_futures_list(limit)
        )
    }

    pub async fn get_futures_current(&self, secids: &[&str]) -> DataResult<Vec<DerivativesQuote>> {
        if secids.is_empty() {
            return Ok(Vec::new());
        }
        try_sources_vec!(
            &self.futures_sources,
            "No futures sources configured",
            |source| source.get_futures_current(secids)
        )
    }

    pub async fn get_futures_market(
        &self,
        secid: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        try_sources_vec!(
            &self.futures_sources,
            "No futures sources configured",
            |source| source.get_futures_market(secid, start_date, end_date, k_type)
        )
    }

    pub async fn get_options_list(
        &self,
        exchange: OptionExchange,
        limit: Option<usize>,
    ) -> DataResult<Vec<OptionContract>> {
        try_sources_vec!(
            &self.options_sources,
            "No options sources configured",
            |source| source.get_options_list(exchange, limit)
        )
    }

    pub async fn get_options_current(
        &self,
        contract_codes: &[&str],
    ) -> DataResult<Vec<DerivativesQuote>> {
        if contract_codes.is_empty() {
            return Ok(Vec::new());
        }
        try_sources_vec!(
            &self.options_sources,
            "No options sources configured",
            |source| source.get_options_current(contract_codes)
        )
    }

    pub async fn get_financial_statement(
        &self,
        stock_code: &str,
        kind: FinancialReportKind,
        limit: Option<usize>,
    ) -> DataResult<Vec<FinancialRecord>> {
        try_sources_vec!(
            &self.financial_sources,
            "No financial sources configured",
            |source| source.get_financial_statement(stock_code, kind, limit)
        )
    }
}

impl Default for DataClient {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for DataClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataClient")
            .field("market_sources", &self.market_sources.len())
            .field("info_sources", &self.info_sources.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use chrono::{NaiveDate, Utc};

    use super::*;
    use crate::data::{CurrentMarketData, MarketData, MinuteData, StockCode};
    use crate::error::DataError;
    use crate::traits::{DataSource, StockInfoSource, StockMarketSource};

    #[test]
    fn test_client_creation() {
        let client = DataClient::new();
        assert_eq!(client.market_source_count(), 0);
        assert_eq!(client.info_source_count(), 0);
    }

    #[derive(Clone)]
    struct UnavailableMarketSource;

    #[async_trait]
    impl DataSource for UnavailableMarketSource {
        fn name(&self) -> &'static str {
            "unavailable"
        }

        fn priority(&self) -> u8 {
            1
        }

        async fn is_available(&self) -> bool {
            false
        }
    }

    #[async_trait]
    impl StockMarketSource for UnavailableMarketSource {
        async fn get_market(
            &self,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
            _: KLineType,
        ) -> DataResult<Vec<MarketData>> {
            Ok(vec![])
        }

        async fn get_market_current(&self, _: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
            Ok(vec![])
        }

        async fn get_market_min(&self, _: &str) -> DataResult<Vec<MinuteData>> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl StockInfoSource for UnavailableMarketSource {
        async fn get_all_codes(&self, _: Option<usize>) -> DataResult<Vec<StockCode>> {
            Ok(vec![])
        }
    }

    #[derive(Clone)]
    struct EmptyMarketSource;

    #[async_trait]
    impl DataSource for EmptyMarketSource {
        fn name(&self) -> &'static str {
            "empty"
        }

        fn priority(&self) -> u8 {
            1
        }
    }

    #[async_trait]
    impl StockMarketSource for EmptyMarketSource {
        async fn get_market(
            &self,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
            _: KLineType,
        ) -> DataResult<Vec<MarketData>> {
            Ok(vec![])
        }

        async fn get_market_current(&self, _: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
            Ok(vec![])
        }

        async fn get_market_min(&self, _: &str) -> DataResult<Vec<MinuteData>> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl StockInfoSource for EmptyMarketSource {
        async fn get_all_codes(&self, _: Option<usize>) -> DataResult<Vec<StockCode>> {
            Ok(vec![])
        }
    }

    #[derive(Clone)]
    struct FailThenSucceedMarketSource {
        fail: bool,
    }

    #[async_trait]
    impl DataSource for FailThenSucceedMarketSource {
        fn name(&self) -> &'static str {
            if self.fail {
                "fail"
            } else {
                "ok"
            }
        }

        fn priority(&self) -> u8 {
            if self.fail {
                1
            } else {
                2
            }
        }
    }

    #[async_trait]
    impl StockMarketSource for FailThenSucceedMarketSource {
        async fn get_market(
            &self,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
            _: KLineType,
        ) -> DataResult<Vec<MarketData>> {
            if self.fail {
                Err(DataError::source_unavailable("down"))
            } else {
                Ok(vec![sample_market_data()])
            }
        }

        async fn get_market_current(&self, _: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
            if self.fail {
                Err(DataError::source_unavailable("down"))
            } else {
                Ok(vec![CurrentMarketData {
                    stock_code: "600519".to_string(),
                    short_name: "Moutai".to_string(),
                    price: 1800.0,
                    change: 1.0,
                    change_pct: 0.05,
                    volume: 1000,
                    amount: 1_800_000.0,
                    open: None,
                    high: None,
                    low: None,
                    pre_close: Some(1799.0),
                }])
            }
        }

        async fn get_market_min(&self, _: &str) -> DataResult<Vec<MinuteData>> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl StockInfoSource for FailThenSucceedMarketSource {
        async fn get_all_codes(&self, _: Option<usize>) -> DataResult<Vec<StockCode>> {
            Ok(vec![])
        }
    }

    fn sample_market_data() -> MarketData {
        MarketData {
            stock_code: "600519".to_string(),
            trade_time: Utc::now(),
            trade_date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
            open: 100.0,
            close: 101.0,
            high: 102.0,
            low: 99.0,
            volume: 1000,
            amount: 100000.0,
            change: 1.0,
            change_pct: 1.0,
            turnover_ratio: 0.5,
            pre_close: 100.0,
        }
    }

    #[tokio::test]
    async fn test_market_fallback_skips_unavailable() {
        let client = DataClient::new()
            .with_source(UnavailableMarketSource)
            .with_source(EmptyMarketSource);
        let err = client.get_market_current(&["600519"]).await.unwrap_err();
        assert!(matches!(err, DataError::NoDataAvailable));
    }

    #[tokio::test]
    async fn test_market_fallback_uses_second_source() {
        let client = DataClient::new()
            .with_source(FailThenSucceedMarketSource { fail: true })
            .with_source(FailThenSucceedMarketSource { fail: false });
        let data = client.get_market_current(&["600519"]).await.unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].stock_code, "600519");
    }

    #[tokio::test]
    async fn test_market_fallback_returns_data_from_kline() {
        let client = DataClient::new().with_source(FailThenSucceedMarketSource { fail: false });
        let data = client
            .get_market("600519", None, None, KLineType::Daily)
            .await
            .unwrap();
        assert_eq!(data.len(), 1);
        assert!(data[0].is_valid());
    }

    #[tokio::test]
    async fn test_market_no_sources_configured() {
        let client = DataClient::new();
        let err = client.get_market_current(&["600519"]).await.unwrap_err();
        assert!(matches!(err, DataError::Custom(_)));
    }
}
