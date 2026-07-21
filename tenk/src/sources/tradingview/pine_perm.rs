use reqwest::header::{HeaderMap, HeaderValue, ORIGIN, REFERER};
use serde_json::Value;

use crate::data::TvPinePermUser;
use crate::error::{DataError, DataResult};

use super::rest::TvRestClient;

const TV_ORIGIN: &str = "https://www.tradingview.com";

pub struct TvPinePerm<'a> {
    client: &'a TvRestClient,
    pine_id: String,
}

impl<'a> TvPinePerm<'a> {
    pub fn new(client: &'a TvRestClient, pine_id: impl Into<String>) -> DataResult<Self> {
        let pine_id = pine_id.into();
        if pine_id.is_empty() {
            return Err(DataError::custom("pine id required"));
        }
        if client.session.is_empty() {
            return Err(DataError::custom("TradingView session required"));
        }
        Ok(Self { client, pine_id })
    }

    pub async fn list_users(&self, limit: u32) -> DataResult<Vec<TvPinePermUser>> {
        let url = format!(
            "https://www.tradingview.com/pine_perm/list_users/?limit={limit}&order_by=-created"
        );
        let body = format!("pine_id={}", encode_pine_id(&self.pine_id));
        let data: Value = self.post_form(&url, &body).await?;
        Ok(data
            .get("results")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(map_perm_user).collect())
            .unwrap_or_default())
    }

    pub async fn add_user(
        &self,
        username: &str,
        expiration: Option<&str>,
    ) -> DataResult<String> {
        let mut body = format!(
            "pine_id={}&username_recip={}",
            encode_pine_id(&self.pine_id),
            urlencoding_encode(username)
        );
        if let Some(expiration) = expiration.filter(|value| !value.is_empty()) {
            body.push_str("&expiration=");
            body.push_str(&urlencoding_encode(expiration));
        }
        let data: Value = self
            .post_form("https://www.tradingview.com/pine_perm/add/", &body)
            .await?;
        Ok(data
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string())
    }

    pub async fn modify_expiration(
        &self,
        username: &str,
        expiration: Option<&str>,
    ) -> DataResult<String> {
        let mut body = format!(
            "pine_id={}&username_recip={}",
            encode_pine_id(&self.pine_id),
            urlencoding_encode(username)
        );
        if let Some(expiration) = expiration.filter(|value| !value.is_empty()) {
            body.push_str("&expiration=");
            body.push_str(&urlencoding_encode(expiration));
        }
        let data: Value = self
            .post_form(
                "https://www.tradingview.com/pine_perm/modify_user_expiration/",
                &body,
            )
            .await?;
        Ok(data
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string())
    }

    pub async fn remove_user(&self, username: &str) -> DataResult<String> {
        let body = format!(
            "pine_id={}&username_recip={}",
            encode_pine_id(&self.pine_id),
            urlencoding_encode(username)
        );
        let data: Value = self
            .post_form("https://www.tradingview.com/pine_perm/remove/", &body)
            .await?;
        Ok(data
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string())
    }

    async fn post_form(&self, url: &str, body: &str) -> DataResult<Value> {
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, HeaderValue::from_static(TV_ORIGIN));
        headers.insert(REFERER, HeaderValue::from_static(TV_ORIGIN));
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        if let Some(cookie) = self.client.ws_cookie() {
            if let Ok(value) = HeaderValue::from_str(&cookie) {
                headers.insert(reqwest::header::COOKIE, value);
            }
        }
        let response = self
            .client
            .http
            .client()
            .post(url)
            .headers(headers)
            .body(body.to_string())
            .send()
            .await
            .map_err(DataError::Network)?;
        let status = response.status();
        let data: Value = response
            .json()
            .await
            .map_err(|error| DataError::custom(error.to_string()))?;
        if !status.is_success() {
            let detail = data
                .get("detail")
                .and_then(Value::as_str)
                .unwrap_or("pine permission request failed");
            return Err(DataError::custom(detail));
        }
        Ok(data)
    }
}

fn map_perm_user(item: &Value) -> Option<TvPinePermUser> {
    Some(TvPinePermUser {
        id: item.get("id").and_then(Value::as_i64).unwrap_or(0),
        username: item
            .get("username")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        expiration: item
            .get("expiration")
            .and_then(Value::as_str)
            .map(str::to_string),
        created: item
            .get("created")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn encode_pine_id(pine_id: &str) -> String {
    pine_id.replace(';', "%3B")
}

fn urlencoding_encode(input: &str) -> String {
    input
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}
