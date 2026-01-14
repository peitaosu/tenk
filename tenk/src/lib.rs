//! tenk - Multi-source market data library for Rust.

pub mod client;
pub mod data;
pub mod error;
pub mod request;
pub mod sources;
pub mod traits;

pub use client::DataClient;
pub use data::{
    AdjustType, BondCurrentData, ConvertibleBondCode, CurrentMarketData, ETFCode, ETFCurrentData,
    ETFMarketData, ETFMinuteData, Exchange, KLineType, MarketData, MinuteData, OrderBookData,
    StockCode, StockInfo, TickData,
};
pub use error::{DataError, DataResult};
pub use request::{RequestConfig, RequestManager};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Creates a new DataClient.
pub fn create_client() -> DataClient {
    DataClient::new()
}

/// Creates a RequestConfig with proxy.
pub fn configure_proxy(proxy_url: &str) -> RequestConfig {
    RequestConfig::default().with_proxy(proxy_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_create_client() {
        let client = create_client();
        assert_eq!(client.market_source_count(), 0);
    }

    #[test]
    fn test_configure_proxy() {
        let config = configure_proxy("http://127.0.0.1:8080");
        assert_eq!(config.proxy, Some("http://127.0.0.1:8080".to_string()));
    }
}
