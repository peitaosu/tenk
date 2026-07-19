//! tenk - Multi-source market data library for Rust.

pub mod builder;
pub mod client;
pub mod data;
pub mod error;
pub mod request;
pub mod sources;
pub mod traits;
pub mod util;

pub use builder::{ClientBuilder, SourceKind};
pub use client::DataClient;
pub use data::{
    format_related_stocks, format_related_stocks_display, AdjustType, BillboardDetail,
    BillboardItem, BlockTradeData, BoardCrosswalkItem, BoardCrosswalkKind, BoardItem, BondCurrentData, CapitalFlowData, CapitalFlowHistory,
    ConvertibleBondCode, CurrentMarketData, DerivativesExchange, DerivativesQuote, DividendData,
    ETFCode, ETFCurrentData, ETFMarketData, ETFMinuteData, EarningsForecast, Exchange,
    FinancialRecord, FinancialReportKind, FundHolding, FuturesContract, IndexCode, IPOData,
    InstitutionalResearchData, KLineType, LimitPoolItem, LimitPoolKind, MacroRecord,
    MarginTradingData, MarketData, MinuteData, NewsArticle, NewsCategory, NewsContent,
    NewsListResult, NewsSearchResult, OptionContract, OptionExchange, OrderBookData, RelatedStock,
    ResearchReportData, StockCode, StockConnectData, StockInfo, StockValuation, TickData, TopHolder,
};
pub use error::{DataError, DataResult};
pub use request::{RequestConfig, RequestManager};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_client_builder() {
        let client = ClientBuilder::new().build().unwrap();
        assert!(client.market_source_count() >= 3);
    }

    #[test]
    fn test_source_kind_export() {
        assert!(SourceKind::ALL.contains(&SourceKind::Ths));
    }
}
