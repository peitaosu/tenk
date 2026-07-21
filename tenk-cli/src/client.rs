//! CLI client wiring.

use tenk::{ClientBuilder, DataClient, DataResult, SourceKind};

pub fn build_client(sources: &[SourceKind], proxy: Option<&str>) -> DataResult<DataClient> {
    let mut builder = ClientBuilder::with_sources(sources);
    if let Some(proxy_url) = resolve_proxy(proxy) {
        builder = builder.with_proxy(proxy_url);
    }
    builder.build()
}

fn resolve_proxy(explicit: Option<&str>) -> Option<String> {
    explicit.map(str::to_string).or_else(|| {
        ["TENK_PROXY", "TENK_TV_PROXY", "HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"]
            .into_iter()
            .find_map(|key| std::env::var(key).ok())
    })
}

pub fn resolve_tui_sources(sources: &[SourceKind]) -> Vec<SourceKind> {
    if sources.is_empty() {
        vec![SourceKind::Eastmoney]
    } else {
        sources.to_vec()
    }
}

pub fn resolve_cli_sources(sources: &[SourceKind]) -> Vec<SourceKind> {
    if sources.is_empty() {
        SourceKind::ALL.to_vec()
    } else {
        sources.to_vec()
    }
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

    #[test]
    fn test_resolve_tui_sources_defaults_to_eastmoney() {
        assert_eq!(resolve_tui_sources(&[]), vec![SourceKind::Eastmoney]);
    }

    #[test]
    fn test_resolve_tui_sources_uses_explicit_only() {
        assert_eq!(
            resolve_tui_sources(&[SourceKind::Tradingview]),
            vec![SourceKind::Tradingview]
        );
    }

    #[test]
    fn test_resolve_cli_sources_defaults_to_all() {
        assert_eq!(resolve_cli_sources(&[]).len(), SourceKind::ALL.len());
    }
}
