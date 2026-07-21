use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use serde_json::Value;
use tracing::debug;

use crate::data::{NewsArticle, NewsCategory, NewsContent, StockCode};
use crate::error::{DataError, DataResult};
use crate::traits::NewsSource;

use super::TradingViewSource;
use super::convert::to_us_tv_symbol;
use super::symbol::to_tv_symbol;

const HEADLINES_URL: &str = "https://news-headlines.tradingview.com/headlines/";
const STORY_URL: &str = "https://news-headlines.tradingview.com/v2/story";

impl TradingViewSource {
    fn news_category(category: NewsCategory) -> &'static str {
        match category {
            NewsCategory::USMarket => "stock",
            NewsCategory::Global | NewsCategory::Domestic | NewsCategory::Finance => "stock",
            NewsCategory::Industry => "economic",
            NewsCategory::Company | NewsCategory::Stock => "stock",
        }
    }

    fn news_lang(category: NewsCategory, symbol: Option<&str>) -> &'static str {
        if let Some(symbol) = symbol {
            if symbol.starts_with("SSE:")
                || symbol.starts_with("SZSE:")
                || symbol.starts_with("BSE:")
            {
                return "zh";
            }
            return "en";
        }
        match category {
            NewsCategory::USMarket | NewsCategory::Global => "en",
            _ => "zh",
        }
    }

    fn parse_headline(item: &Value, category: NewsCategory) -> Option<NewsArticle> {
        let id = item.get("id")?.as_str()?.to_string();
        let title = item.get("title")?.as_str()?.to_string();
        let provider = item
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("TradingView")
            .to_string();
        let published = item
            .get("published")
            .and_then(Value::as_i64)
            .and_then(|ts| Utc.timestamp_opt(ts, 0).single())
            .unwrap_or_else(Utc::now);
        let digest = ast_to_text(item.get("astDescription").unwrap_or(&Value::Null));
        Some(NewsArticle {
            id: format!("tv:{id}"),
            title,
            digest,
            url: String::new(),
            url_mobile: None,
            source: provider,
            publish_time: published,
            category,
            comment_count: 0,
            has_image: false,
            image_url: None,
        })
    }

    async fn fetch_headlines(
        &self,
        category: NewsCategory,
        page: u32,
        limit: u32,
        symbol: Option<&str>,
    ) -> DataResult<Vec<NewsArticle>> {
        let mut params = vec![
            ("category", Self::news_category(category).to_string()),
            ("lang", Self::news_lang(category, symbol).to_string()),
        ];
        if let Some(symbol) = symbol.filter(|value| !value.is_empty()) {
            params.push(("symbol", symbol.to_string()));
        }
        let _ = page;
        let data: Value = self
            .rest
            .http
            .get_with_headers(HEADLINES_URL, &params, Some(self.rest.tv_headers()))
            .await?;
        let items = data.as_array().cloned().unwrap_or_default();
        Ok(items
            .into_iter()
            .filter_map(|item| Self::parse_headline(&item, category))
            .take(limit as usize)
            .collect())
    }
}

pub(crate) fn ast_to_text(node: &Value) -> String {
    match node {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .map(ast_to_text)
            .collect::<Vec<_>>()
            .join(""),
        Value::Object(map) => {
            if let Some(children) = map.get("children") {
                return ast_to_text(children);
            }
            String::new()
        }
        _ => String::new(),
    }
}

pub(crate) fn ast_to_html(node: &Value) -> String {
    match node {
        Value::String(text) => text.to_string(),
        Value::Array(items) => items
            .iter()
            .map(ast_to_html)
            .collect::<Vec<_>>()
            .join(""),
        Value::Object(map) => {
            let node_type = map.get("type").and_then(Value::as_str).unwrap_or("");
            let inner = ast_to_html(map.get("children").unwrap_or(&Value::Null));
            match node_type {
                "p" | "root" => format!("<p>{inner}</p>"),
                "a" => {
                    let href = map
                        .get("href")
                        .and_then(Value::as_str)
                        .unwrap_or("#");
                    format!(r#"<a href="{href}">{inner}</a>"#)
                }
                _ => inner,
            }
        }
        _ => String::new(),
    }
}

#[async_trait]
impl NewsSource for TradingViewSource {
    async fn get_news(
        &self,
        category: NewsCategory,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<NewsArticle>> {
        debug!(
            "Fetching TradingView news: category={:?}, page={}, limit={}",
            category, page, limit
        );
        self.fetch_headlines(category, page, limit, None).await
    }

    async fn get_news_content(&self, news_id: &str) -> DataResult<NewsContent> {
        let id = news_id
            .strip_prefix("tv:")
            .ok_or_else(|| DataError::not_supported("get_news_content"))?;
        let params = [("lang", "en".to_string()), ("id", id.to_string())];
        debug!("Fetching TradingView news content: id={}", id);
        let story: Value = self
            .rest
            .http
            .get_with_headers(STORY_URL, &params, Some(self.rest.tv_headers()))
            .await?;
        let title = story
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let description = story
            .get("shortDescription")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let body_node = story.get("astDescription").cloned().unwrap_or(Value::Null);
        let body_html = ast_to_html(&body_node);
        let body_text = ast_to_text(&body_node);
        let source = story
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("TradingView")
            .to_string();
        let publish_time = story
            .get("published")
            .and_then(Value::as_i64)
            .and_then(|ts| Utc.timestamp_opt(ts, 0).single())
            .unwrap_or_else(Utc::now);
        let related_stocks = story
            .get("relatedSymbols")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.get("symbol").and_then(Value::as_str))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        Ok(NewsContent {
            id: news_id.to_string(),
            title,
            description,
            body_html,
            body_text,
            source,
            author: None,
            publish_time,
            related_stocks,
            images: Vec::new(),
        })
    }

    async fn search_news(
        &self,
        keyword: &str,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<NewsArticle>> {
        let keyword = keyword.trim();
        if keyword.is_empty() {
            return Ok(Vec::new());
        }
        debug!("Searching TradingView news: keyword={}, page={}", keyword, page);

        if let Some(tv_symbol) = resolve_news_symbol(keyword) {
            let articles = self
                .fetch_headlines(NewsCategory::Stock, page, limit, Some(&tv_symbol))
                .await?;
            if !articles.is_empty() {
                return Ok(articles);
            }
        }

        let category = if keyword.is_ascii() {
            NewsCategory::USMarket
        } else {
            NewsCategory::Finance
        };
        let scan_limit = 200u32;
        let batch = self
            .fetch_headlines(category, 1, scan_limit, None)
            .await?;
        let matched: Vec<_> = batch
            .into_iter()
            .filter(|article| news_matches(article, keyword))
            .collect();
        let start = ((page.saturating_sub(1)) as usize) * limit as usize;
        let end = start + limit as usize;
        Ok(matched.into_iter().skip(start).take(end).collect())
    }

    async fn search_news_for_symbol(
        &self,
        symbol: &StockCode,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<NewsArticle>> {
        let tv_symbol = symbol.tv_symbol();
        let articles = self
            .fetch_headlines(NewsCategory::Stock, page, limit, Some(&tv_symbol))
            .await?;
        if !articles.is_empty() {
            return Ok(articles);
        }
        if !symbol.short_name.is_empty() {
            return self.search_news(&symbol.short_name, page, limit).await;
        }
        Ok(Vec::new())
    }
}

fn resolve_news_symbol(keyword: &str) -> Option<String> {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return None;
    }
    if keyword.contains(':') {
        return Some(keyword.to_string());
    }
    if keyword.chars().all(|ch| ch.is_ascii_digit()) && keyword.len() == 6 {
        return Some(to_tv_symbol(keyword));
    }
    if keyword.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-')
        && !keyword.chars().any(char::is_whitespace)
        && keyword.len() <= 16
    {
        let upper = keyword.to_ascii_uppercase();
        if upper.chars().all(|ch| ch.is_ascii_alphabetic()) && !upper.is_empty() {
            return Some(to_us_tv_symbol(&upper));
        }
    }
    None
}

fn news_matches(article: &NewsArticle, keyword: &str) -> bool {
    if keyword.is_ascii() {
        let needle = keyword.to_ascii_lowercase();
        article.title.to_ascii_lowercase().contains(&needle)
            || article.digest.to_ascii_lowercase().contains(&needle)
    } else {
        article.title.contains(keyword) || article.digest.contains(keyword)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_ast_to_text() {
        let node = json!({
            "type": "root",
            "children": [
                {"type": "p", "children": ["Hello ", "world"]}
            ]
        });
        assert_eq!(ast_to_text(&node), "Hello world");
    }
}
