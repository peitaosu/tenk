//! CLI client wiring.

use tenk::{ClientBuilder, DataClient, DataResult, SourceKind};

pub fn build_client(sources: &[SourceKind], proxy: Option<&str>) -> DataResult<DataClient> {
    let mut builder = ClientBuilder::with_sources(sources);
    if let Some(proxy_url) = proxy {
        builder = builder.with_proxy(proxy_url.to_string());
    }
    builder.build()
}

pub fn default_client(proxy: Option<&str>) -> DataResult<DataClient> {
    build_client(SourceKind::ALL, proxy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_client_builds() {
        let client = default_client(None).unwrap();
        assert!(client.market_source_count() >= 3);
    }

    #[test]
    fn test_build_client_single_source() {
        let client = build_client(&[SourceKind::Ths], None).unwrap();
        assert!(client.market_source_count() >= 1);
    }
}
