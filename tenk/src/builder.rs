//! DataClient builder with default sources and proxy support.

use crate::client::DataClient;
use crate::error::DataResult;
use crate::sources::{EastMoneySource, SinaSource, THSSource};

/// Data provider selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Eastmoney,
    Sina,
    Ths,
}

impl SourceKind {
    pub const ALL: &'static [Self] = &[Self::Eastmoney, Self::Sina, Self::Ths];
}

/// Builds a configured [`DataClient`].
#[derive(Debug, Clone, Default)]
pub struct ClientBuilder {
    sources: Vec<SourceKind>,
    proxy: Option<String>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_sources(sources: &[SourceKind]) -> Self {
        Self {
            sources: sources.to_vec(),
            proxy: None,
        }
    }

    pub fn with_proxy(mut self, proxy: impl Into<String>) -> Self {
        self.proxy = Some(proxy.into());
        self
    }

    pub fn build(self) -> DataResult<DataClient> {
        let sources = if self.sources.is_empty() {
            SourceKind::ALL.to_vec()
        } else {
            self.sources
        };

        let proxy = self.proxy.as_deref();
        let mut client = DataClient::new();

        for source in sources {
            match source {
                SourceKind::Eastmoney => {
                    let em = EastMoneySource::try_new(proxy)?;
                    client = client
                        .with_source(em.clone())
                        .with_fund_source(em.clone())
                        .with_bond_source(em.clone())
                        .with_news_source(em.clone())
                        .with_extended_market(em);
                }
                SourceKind::Sina => {
                    let sina = SinaSource::try_new(proxy)?;
                    client = client
                        .with_source(sina.clone())
                        .with_fund_source(sina.clone())
                        .with_bond_info_source(sina.clone())
                        .with_bond_market_source(sina.clone())
                        .with_index_source(sina.clone())
                        .with_futures_source(sina);
                }
                SourceKind::Ths => {
                    let ths = THSSource::try_new(proxy)?;
                    client = client
                        .with_source(ths.clone())
                        .with_fund_source(ths.clone())
                        .with_bond_info_source(ths.clone())
                        .with_bond_market_source(ths.clone())
                        .with_board_source(ths.clone())
                        .with_news_source(ths);
                }
            }
        }

        Ok(client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default_sources() {
        let client = ClientBuilder::new().build().unwrap();
        assert!(client.market_source_count() >= 3);
        assert!(client.info_source_count() >= 3);
    }

    #[test]
    fn test_builder_with_single_source() {
        let client = ClientBuilder::with_sources(&[SourceKind::Sina])
            .build()
            .unwrap();
        assert!(client.market_source_count() >= 1);
    }

    #[test]
    fn test_source_kind_all() {
        assert_eq!(SourceKind::ALL.len(), 3);
        assert!(SourceKind::ALL.contains(&SourceKind::Eastmoney));
    }
}
