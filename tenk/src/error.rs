//! Error types for tenk.

use thiserror::Error;

/// Error type for data operations.
#[derive(Error, Debug)]
pub enum DataError {
    #[error("Network Error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Parse Error: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("Source Unavailable: {0}")]
    SourceUnavailable(String),

    #[error("Not Supported: {0}")]
    NotSupported(String),

    #[error("No Data Available")]
    NoDataAvailable,

    #[error("Invalid Stock Code: {0}")]
    InvalidStockCode(String),

    #[error("Rate Limit Exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Invalid Date: {0}")]
    InvalidDate(String),

    #[error("Configuration Error: {0}")]
    Config(String),

    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Custom(String),
}

pub type DataResult<T> = Result<T, DataError>;

impl DataError {
    pub fn custom<S: Into<String>>(message: S) -> Self {
        DataError::Custom(message.into())
    }

    pub fn source_unavailable<S: Into<String>>(source: S) -> Self {
        DataError::SourceUnavailable(source.into())
    }

    pub fn not_supported<S: Into<String>>(feature: S) -> Self {
        DataError::NotSupported(feature.into())
    }

    pub fn invalid_stock_code<S: Into<String>>(code: S) -> Self {
        DataError::InvalidStockCode(code.into())
    }

    pub fn rate_limited<S: Into<String>>(source: S) -> Self {
        DataError::RateLimitExceeded(source.into())
    }

    pub fn invalid_date<S: Into<String>>(message: S) -> Self {
        DataError::InvalidDate(message.into())
    }

    /// Returns true if should try next source.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            DataError::Network(_)
                | DataError::SourceUnavailable(_)
                | DataError::RateLimitExceeded(_)
                | DataError::NotSupported(_)
                | DataError::Parse(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = DataError::custom("test error");
        assert_eq!(err.to_string(), "test error");

        let err = DataError::invalid_stock_code("INVALID");
        assert_eq!(err.to_string(), "Invalid stock code: INVALID");
    }

    #[test]
    fn test_recoverable_errors() {
        let err = DataError::source_unavailable("test");
        assert!(err.is_recoverable());

        let err = DataError::NoDataAvailable;
        assert!(!err.is_recoverable());
    }
}
