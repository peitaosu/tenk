use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;
use tracing::debug;

use crate::data::{NewsArticle, NewsCategory, NewsContent};
use crate::error::{DataError, DataResult};
use crate::traits::NewsSource;
use crate::util::{extract_balanced_div, html_to_text};

use super::SinaSource;

const ROLL_URL: &str = "https://feed.mix.sina.com.cn/api/roll/get";
const ZHIBO_URL: &str = "https://zhibo.sina.com.cn/api/zhibo/feed";
const ZHIBO_ID: &str = "152";

impl SinaSource {
    fn roll_lid(category: NewsCategory) -> &'static str {
        match category {
            NewsCategory::Finance | NewsCategory::Stock | NewsCategory::Domestic => "2509",
            NewsCategory::Industry => "2515",
            NewsCategory::USMarket | NewsCategory::Global => "2510",
            NewsCategory::Company => "2509",
        }
    }

    fn parse_roll_time(ctime: &str, mtime: &str) -> chrono::DateTime<Utc> {
        fn parse_one(value: &str) -> Option<chrono::DateTime<Utc>> {
            if value.is_empty() {
                return None;
            }
            if let Ok(timestamp) = value.parse::<i64>() {
                return chrono::DateTime::from_timestamp(timestamp, 0);
            }
            chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                .ok()
                .and_then(|dt| {
                    dt.and_local_timezone(chrono::FixedOffset::east_opt(8 * 3600).unwrap())
                        .single()
                })
                .map(|dt| dt.with_timezone(&Utc))
        }

        parse_one(ctime)
            .or_else(|| parse_one(mtime))
            .unwrap_or_else(Utc::now)
    }

    async fn fetch_roll(
        &self,
        lid: &str,
        page: u32,
        limit: u32,
        keyword: Option<&str>,
    ) -> DataResult<Vec<NewsArticle>> {
        let mut params = vec![
            ("pageid", "153".to_string()),
            ("lid", lid.to_string()),
            ("num", limit.max(1).to_string()),
            ("page", page.max(1).to_string()),
        ];
        if let Some(keyword) = keyword.filter(|value| !value.is_empty()) {
            params.push(("k", keyword.to_string()));
        }

        #[derive(Deserialize)]
        struct RollResponse {
            result: Option<RollResult>,
        }

        #[derive(Deserialize)]
        struct RollResult {
            data: Option<Vec<RollItem>>,
        }

        #[derive(Deserialize)]
        struct RollItem {
            #[serde(default)]
            docid: String,
            #[serde(default)]
            title: String,
            #[serde(default)]
            intro: String,
            #[serde(default)]
            summary: String,
            #[serde(default)]
            url: String,
            #[serde(default)]
            wapurl: String,
            #[serde(default)]
            ctime: String,
            #[serde(default)]
            mtime: String,
            #[serde(default)]
            media_name: String,
            #[serde(default)]
            img: Value,
            #[serde(default)]
            images: Vec<Value>,
        }

        let response: RollResponse = self
            .finance_request()
            .get_json_with_params(ROLL_URL, &params)
            .await?;
        let items = response.result.and_then(|result| result.data).unwrap_or_default();
        Ok(items
            .into_iter()
            .filter_map(|item| {
                if item.title.is_empty() {
                    return None;
                }
                let wapurl = item.wapurl.clone();
                let url = if !item.url.is_empty() {
                    item.url
                } else if !wapurl.is_empty() {
                    wapurl.clone()
                } else {
                    return None;
                };
                let digest = if !item.intro.is_empty() {
                    item.intro
                } else {
                    item.summary
                };
                let image_url = roll_image_url(&item.img).or_else(|| {
                    item.images
                        .iter()
                        .find_map(|value| value.get("u").and_then(Value::as_str))
                        .map(str::to_string)
                });
                let id = if item.docid.is_empty() {
                    url.clone()
                } else {
                    format!("sina:roll:{}", item.docid)
                };
                Some(NewsArticle {
                    id,
                    title: item.title,
                    digest,
                    url,
                    url_mobile: if wapurl.is_empty() {
                        None
                    } else {
                        Some(wapurl)
                    },
                    source: if item.media_name.is_empty() {
                        "Sina".to_string()
                    } else {
                        item.media_name
                    },
                    publish_time: Self::parse_roll_time(&item.ctime, &item.mtime),
                    category: NewsCategory::Finance,
                    comment_count: 0,
                    has_image: image_url.is_some(),
                    image_url,
                })
            })
            .collect())
    }

    async fn resolve_roll_url(&self, docid: &str) -> DataResult<String> {
        for page in 1..=5 {
            let articles = self.fetch_roll("2509", page, 50, None).await?;
            for article in articles {
                if article.id == format!("sina:roll:{docid}") {
                    return Ok(article.url);
                }
            }
        }
        Err(DataError::custom("Sina news article not found"))
    }

    fn parse_sina_article_html(html: &str) -> (String, String, String, String) {
        let title = extract_meta_content(html, "og:title")
            .or_else(|| extract_tag_text(html, "h1"))
            .unwrap_or_default();
        let description = extract_meta_content(html, "og:description")
            .or_else(|| extract_meta_content(html, "description"))
            .unwrap_or_default();
        let body_html = extract_article_body_html(html).unwrap_or_default();
        let body_text = html_to_text(&body_html);
        (title, description, body_html, body_text)
    }
}

fn roll_image_url(value: &Value) -> Option<String> {
    match value {
        Value::String(url) if !url.is_empty() => Some(url.clone()),
        Value::Object(map) => map
            .get("u")
            .and_then(Value::as_str)
            .filter(|url| !url.is_empty())
            .map(str::to_string),
        _ => None,
    }
}

fn extract_meta_content(html: &str, key: &str) -> Option<String> {
    for pattern in [format!(r#"property="{key}""#), format!(r#"name="{key}""#)] {
        let Some(start) = html.find(&pattern) else {
            continue;
        };
        let fragment = &html[start..];
        let content_key = "content=\"";
        let content_start = fragment.find(content_key)? + content_key.len();
        let content_end = fragment[content_start..].find('"')? + content_start;
        return Some(html[content_start..content_end].to_string());
    }
    None
}

fn extract_tag_text(html: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let start = html.find(&open)?;
    let tag_end = html[start..].find('>')? + start;
    let close_start = tag_end + 1;
    let close = format!("</{tag}>");
    let end = html[close_start..].find(&close)? + close_start;
    let text = html[close_start..end]
        .replace("<![CDATA[", "")
        .replace("]]>", "");
    Some(strip_tags(&text))
}

fn extract_article_body_html(html: &str) -> Option<String> {
    for marker in [r#"id="artibody""#, r#"class="article""#, r#"id="article""#] {
        if let Some(body) = extract_balanced_div(html, marker) {
            if !body.trim().is_empty() {
                return Some(body);
            }
        }
    }
    None
}

fn strip_tags(input: &str) -> String {
    let mut output = String::new();
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output.trim().to_string()
}

#[async_trait]
impl NewsSource for SinaSource {
    async fn get_news(
        &self,
        category: NewsCategory,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<NewsArticle>> {
        debug!(
            "Fetching Sina news: category={:?}, page={}, limit={}",
            category, page, limit
        );
        self.fetch_roll(Self::roll_lid(category), page, limit, None)
            .await
    }

    async fn get_news_content(&self, news_id: &str) -> DataResult<NewsContent> {
        if news_id.starts_with("sina:zhibo:") {
            let feed_id = news_id.trim_start_matches("sina:zhibo:");
            for page in 1..=5 {
                let params = [
                    ("zhibo_id", ZHIBO_ID),
                    ("page", &page.to_string()),
                    ("page_size", "50"),
                    ("tag_id", "0"),
                ];
                #[derive(Deserialize)]
                struct ZhiboResponse {
                    result: Option<ZhiboResult>,
                }
                #[derive(Deserialize)]
                struct ZhiboResult {
                    data: Option<ZhiboData>,
                }
                #[derive(Deserialize)]
                struct ZhiboData {
                    feed: Option<ZhiboFeed>,
                }
                #[derive(Deserialize)]
                struct ZhiboFeed {
                    list: Option<Vec<ZhiboItem>>,
                }
                #[derive(Deserialize)]
                struct ZhiboItem {
                    id: i64,
                    rich_text: String,
                    create_time: String,
                }
                let response: ZhiboResponse = self
                    .finance_request()
                    .get_json_with_params(ZHIBO_URL, &params)
                    .await?;
                let items = response
                    .result
                    .and_then(|result| result.data)
                    .and_then(|data| data.feed)
                    .and_then(|feed| feed.list)
                    .unwrap_or_default();
                if let Some(item) = items.into_iter().find(|item| item.id.to_string() == feed_id) {
                    let publish_time = chrono::NaiveDateTime::parse_from_str(
                        &item.create_time,
                        "%Y-%m-%d %H:%M:%S",
                    )
                    .ok()
                    .and_then(|dt| {
                        dt.and_local_timezone(chrono::FixedOffset::east_opt(8 * 3600).unwrap())
                            .single()
                    })
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now);
                    return Ok(NewsContent {
                        id: news_id.to_string(),
                        title: item.rich_text.clone(),
                        description: item.rich_text.clone(),
                        body_html: format!("<p>{}</p>", item.rich_text),
                        body_text: item.rich_text,
                        source: "Sina".to_string(),
                        author: None,
                        publish_time,
                        related_stocks: Vec::new(),
                        images: Vec::new(),
                    });
                }
            }
            return Err(DataError::custom("Sina flash news not found"));
        }

        let article_url = if news_id.starts_with("sina:roll:") {
            self.resolve_roll_url(news_id.trim_start_matches("sina:roll:"))
                .await?
        } else if news_id.starts_with("http://") || news_id.starts_with("https://") {
            news_id.to_string()
        } else if news_id.starts_with("sina:") {
            return Err(DataError::not_supported("get_news_content"));
        } else {
            return Err(DataError::not_supported("get_news_content"));
        };

        debug!("Fetching Sina news content: {}", article_url);
        let response = self.finance_request().get(&article_url).await?;
        let html = response.text().await.map_err(DataError::Network)?;
        let (title, description, body_html, body_text) = Self::parse_sina_article_html(&html);
        Ok(NewsContent {
            id: news_id.to_string(),
            title,
            description,
            body_html,
            body_text,
            source: "Sina".to_string(),
            author: None,
            publish_time: Utc::now(),
            related_stocks: Vec::new(),
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
        debug!("Searching Sina news: keyword={}, page={}", keyword, page);
        self.fetch_roll("2509", page, limit, Some(keyword)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_strip_tags() {
        assert_eq!(strip_tags("<p>hello</p>"), "hello");
    }

    #[test]
    fn parse_roll_time_accepts_unix_timestamp() {
        let parsed = SinaSource::parse_roll_time("1784557081", "");
        let expected = chrono::FixedOffset::east_opt(8 * 3600)
            .unwrap()
            .with_ymd_and_hms(2026, 7, 20, 22, 18, 1)
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_roll_time_accepts_datetime_string() {
        let parsed = SinaSource::parse_roll_time("2026-07-20 21:44:18", "");
        let expected = chrono::FixedOffset::east_opt(8 * 3600)
            .unwrap()
            .with_ymd_and_hms(2026, 7, 20, 21, 44, 18)
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(parsed, expected);
    }
}
