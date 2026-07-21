//! HTTP request manager with retry and proxy support.

use reqwest::header::HeaderMap;
use reqwest::{Client, Response};
use std::sync::{Arc, Once};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

use crate::error::{DataError, DataResult};

static TLS_PROVIDER: Once = Once::new();

fn ensure_tls_provider() {
    TLS_PROVIDER.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

/// HTTP request configuration.
#[derive(Debug, Clone)]
pub struct RequestConfig {
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Wait time between retries (ms)
    pub retry_wait_ms: u64,
    /// Wait time between requests (ms)
    pub request_wait_ms: Option<u64>,
    /// Request timeout
    pub timeout: Duration,
    /// Proxy URL
    pub proxy: Option<String>,
    /// User agent string
    pub user_agent: String,
    /// Custom headers
    pub headers: Option<HeaderMap>,
}

impl Default for RequestConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            retry_wait_ms: 1500,
            request_wait_ms: None,
            timeout: Duration::from_secs(30),
            proxy: None,
            user_agent: format!("tenk/{}", env!("CARGO_PKG_VERSION")),
            headers: None,
        }
    }
}

impl RequestConfig {
    /// Creates a new default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum retry count.
    pub fn with_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Sets the wait time between retries.
    pub fn with_retry_wait(mut self, wait_ms: u64) -> Self {
        self.retry_wait_ms = wait_ms;
        self
    }

    /// Sets the wait time between requests.
    pub fn with_request_wait(mut self, wait_ms: u64) -> Self {
        self.request_wait_ms = Some(wait_ms);
        self
    }

    /// Sets the request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the proxy URL.
    pub fn with_proxy<S: Into<String>>(mut self, proxy_url: S) -> Self {
        self.proxy = Some(proxy_url.into());
        self
    }

    /// Sets the user agent string.
    pub fn with_user_agent<S: Into<String>>(mut self, user_agent: S) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Sets default headers.
    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = Some(headers);
        self
    }

    pub fn with_proxy_opt(mut self, proxy: Option<&str>) -> Self {
        if let Some(proxy_url) = proxy {
            self.proxy = Some(proxy_url.to_string());
        }
        self
    }
}

/// HTTP client with retry logic.
#[derive(Clone)]
pub struct RequestManager {
    /// HTTP client instance
    client: Client,
    /// Request configuration
    config: Arc<RequestConfig>,
}

impl RequestManager {
    /// Creates a new request manager with configuration.
    pub fn new(config: RequestConfig) -> DataResult<Self> {
        ensure_tls_provider();
        let mut client_builder = Client::builder()
            .timeout(config.timeout)
            .user_agent(&config.user_agent)
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90));

        if let Some(proxy_url) = &config.proxy {
            let proxy = reqwest::Proxy::all(proxy_url)
                .map_err(|e| DataError::Config(format!("Invalid proxy URL '{proxy_url}': {e}")))?;
            client_builder = client_builder.proxy(proxy);
        } else {
            client_builder = client_builder.no_proxy();
        }

        if let Some(headers) = &config.headers {
            client_builder = client_builder.default_headers(headers.clone());
        }

        let client = client_builder
            .build()
            .map_err(|e| DataError::Config(format!("Failed to build HTTP client: {e}")))?;

        Ok(Self {
            client,
            config: Arc::new(config),
        })
    }

    /// Creates a request manager with default configuration.
    pub fn default_manager() -> DataResult<Self> {
        Self::new(RequestConfig::default())
    }

    /// Returns the configuration.
    pub fn config(&self) -> &RequestConfig {
        &self.config
    }

    /// Performs a GET request.
    pub async fn get(&self, url: &str) -> DataResult<Response> {
        self.request_with_retry(|| self.client.get(url)).await
    }

    /// Performs a GET request with query parameters.
    pub async fn get_with_params<T: serde::Serialize + ?Sized>(
        &self,
        url: &str,
        params: &T,
    ) -> DataResult<Response> {
        self.request_with_retry(|| self.client.get(url).query(params))
            .await
    }

    /// Performs a POST request with JSON body.
    pub async fn post_json<T: serde::Serialize + ?Sized>(
        &self,
        url: &str,
        body: &T,
    ) -> DataResult<Response> {
        self.request_with_retry(|| self.client.post(url).json(body))
            .await
    }

    /// Performs a GET request and deserializes JSON response.
    pub async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> DataResult<T> {
        let response = self.get(url).await?;
        let json = response.json::<T>().await?;
        Ok(json)
    }

    /// Performs a GET request with params and deserializes JSON response.
    pub async fn get_json_with_params<T, P>(&self, url: &str, params: &P) -> DataResult<T>
    where
        T: serde::de::DeserializeOwned,
        P: serde::Serialize + ?Sized,
    {
        let response = self.get_with_params(url, params).await?;
        let json = response.json::<T>().await?;
        Ok(json)
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub async fn get_with_headers<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        params: &[(&str, String)],
        headers: Option<HeaderMap>,
    ) -> DataResult<T> {
        let response = self
            .request_with_retry(|| {
                let mut builder = self.client.get(url).query(params);
                if let Some(headers) = &headers {
                    builder = builder.headers(headers.clone());
                }
                builder
            })
            .await?;
        Ok(response.json::<T>().await?)
    }

    pub async fn post_json_with_headers<T, B>(
        &self,
        url: &str,
        body: &B,
        headers: Option<HeaderMap>,
    ) -> DataResult<T>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize + ?Sized,
    {
        let response = self
            .request_with_retry(|| {
                let mut builder = self.client.post(url).json(body);
                if let Some(headers) = &headers {
                    builder = builder.headers(headers.clone());
                }
                builder
            })
            .await?;
        Ok(response.json::<T>().await?)
    }

    pub async fn get_text_with_headers(
        &self,
        url: &str,
        params: &[(&str, String)],
        headers: Option<HeaderMap>,
    ) -> DataResult<String> {
        let response = self
            .request_with_retry(|| {
                let mut builder = self.client.get(url).query(params);
                if let Some(headers) = &headers {
                    builder = builder.headers(headers.clone());
                }
                builder
            })
            .await?;
        Ok(response.text().await?)
    }

    /// Performs a request with retry logic.
    async fn request_with_retry<F>(&self, builder: F) -> DataResult<Response>
    where
        F: Fn() -> reqwest::RequestBuilder,
    {
        let mut last_error = None;

        for attempt in 0..self.config.max_retries {
            if let Some(wait_ms) = self.config.request_wait_ms {
                sleep(Duration::from_millis(wait_ms)).await;
            }

            debug!(
                "Request attempt {} of {}",
                attempt + 1,
                self.config.max_retries
            );

            match builder().send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() || status == reqwest::StatusCode::NOT_FOUND {
                        return Ok(response);
                    }

                    if is_retryable_status(status) {
                        warn!("Retryable HTTP status {}, waiting before retry", status);
                        sleep(Duration::from_millis(self.config.retry_wait_ms * 2)).await;
                        last_error = Some(DataError::custom(format!("HTTP error: {status}")));
                        continue;
                    }

                    warn!("Request failed with status: {}", status);
                    last_error = Some(DataError::custom(format!("HTTP error: {status}")));
                }
                Err(e) => {
                    warn!("Request error: {}", e);
                    let retryable = is_retryable_error(&e);
                    last_error = Some(DataError::Network(e));
                    if !retryable {
                        break;
                    }
                }
            }

            if attempt < self.config.max_retries - 1 {
                sleep(Duration::from_millis(self.config.retry_wait_ms)).await;
            }
        }

        Err(last_error.unwrap_or_else(|| DataError::custom("Request failed after all retries")))
    }
}

fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    matches!(
        status,
        reqwest::StatusCode::TOO_MANY_REQUESTS
            | reqwest::StatusCode::INTERNAL_SERVER_ERROR
            | reqwest::StatusCode::BAD_GATEWAY
            | reqwest::StatusCode::SERVICE_UNAVAILABLE
            | reqwest::StatusCode::GATEWAY_TIMEOUT
            | reqwest::StatusCode::REQUEST_TIMEOUT
    )
}

fn is_retryable_error(error: &reqwest::Error) -> bool {
    error.is_connect() || error.is_timeout() || error.is_request()
}

impl std::fmt::Debug for RequestManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestManager")
            .field("config", &self.config)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = RequestConfig::new()
            .with_retries(5)
            .with_retry_wait(2000)
            .with_timeout(Duration::from_secs(60));

        assert_eq!(config.max_retries, 5);
        assert_eq!(config.retry_wait_ms, 2000);
        assert_eq!(config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_config_with_proxy_opt() {
        let with_proxy = RequestConfig::default().with_proxy_opt(Some("http://127.0.0.1:7890"));
        assert_eq!(with_proxy.proxy.as_deref(), Some("http://127.0.0.1:7890"));

        let without_proxy = RequestConfig::default().with_proxy_opt(None);
        assert!(without_proxy.proxy.is_none());
    }

    #[test]
    fn test_is_retryable_status() {
        assert!(is_retryable_status(reqwest::StatusCode::BAD_GATEWAY));
        assert!(is_retryable_status(reqwest::StatusCode::SERVICE_UNAVAILABLE));
        assert!(!is_retryable_status(reqwest::StatusCode::NOT_FOUND));
        assert!(!is_retryable_status(reqwest::StatusCode::FORBIDDEN));
    }

    #[tokio::test]
    async fn test_request_manager_creation() {
        let manager = RequestManager::default_manager();
        assert!(manager.is_ok());
    }
}
