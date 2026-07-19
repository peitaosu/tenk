//! Error types for tenk.

use thiserror::Error;

/// Error type for data operations.
#[derive(Error, Debug)]
pub enum DataError {
    /// Network request failed
    #[error("Network Error: {0}")]
    Network(#[from] reqwest::Error),

    /// JSON parsing failed
    #[error("Parse Error: {0}")]
    Parse(#[from] serde_json::Error),

    /// Data source is unavailable
    #[error("Source Unavailable: {0}")]
    SourceUnavailable(String),

    /// Feature not supported by source
    #[error("Not Supported: {0}")]
    NotSupported(String),

    /// No data available from any source
    #[error("No Data Available")]
    NoDataAvailable,

    /// Invalid stock code format
    #[error("Invalid Stock Code: {0}")]
    InvalidStockCode(String),

    /// Rate limit exceeded
    #[error("Rate Limit Exceeded: {0}")]
    RateLimitExceeded(String),

    /// Invalid date format or range
    #[error("Invalid Date: {0}")]
    InvalidDate(String),

    /// Configuration error
    #[error("Configuration Error: {0}")]
    Config(String),

    /// IO operation failed
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    /// Custom error message
    #[error("{0}")]
    Custom(String),
}

/// Result type alias for data operations.
pub type DataResult<T> = Result<T, DataError>;

impl DataError {
    /// Creates a custom error with a message.
    pub fn custom<S: Into<String>>(message: S) -> Self {
        DataError::Custom(message.into())
    }

    /// Creates a source unavailable error.
    pub fn source_unavailable<S: Into<String>>(source: S) -> Self {
        DataError::SourceUnavailable(source.into())
    }

    /// Creates a not supported error.
    pub fn not_supported<S: Into<String>>(feature: S) -> Self {
        DataError::NotSupported(feature.into())
    }

    /// Creates an invalid stock code error.
    pub fn invalid_stock_code<S: Into<String>>(code: S) -> Self {
        DataError::InvalidStockCode(code.into())
    }

    /// Creates a rate limited error.
    pub fn rate_limited<S: Into<String>>(source: S) -> Self {
        DataError::RateLimitExceeded(source.into())
    }

    /// Creates an invalid date error.
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
        assert_eq!(err.to_string(), "Invalid Stock Code: INVALID");
    }

    #[test]
    fn test_recoverable_errors() {
        let err = DataError::source_unavailable("test");
        assert!(err.is_recoverable());

        let err = DataError::NoDataAvailable;
        assert!(!err.is_recoverable());

        let err = DataError::not_supported("feature");
        assert!(err.is_recoverable());

        let err = DataError::invalid_stock_code("000");
        assert!(!err.is_recoverable());

        let err = DataError::invalid_date("bad date");
        assert!(!err.is_recoverable());
    }

    #[test]
    fn test_error_helpers() {
        assert_eq!(
            DataError::rate_limited("sina").to_string(),
            "Rate Limit Exceeded: sina"
        );
        assert_eq!(
            DataError::not_supported("ticks").to_string(),
            "Not Supported: ticks"
        );
        assert_eq!(
            DataError::invalid_date("range").to_string(),
            "Invalid Date: range"
        );
    }
}
