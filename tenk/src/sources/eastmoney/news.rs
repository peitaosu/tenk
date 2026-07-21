use super::*;
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use serde::Deserialize;
use tracing::debug;

use crate::data::{
    Exchange, NewsArticle, NewsCategory, NewsContent, StockCode, filter_news_for_symbol,
    sort_news_by_time_desc,
};
use crate::error::DataResult;
use crate::traits::NewsSource;

#[async_trait]
impl NewsSource for EastMoneySource {
    /// Fetches news articles by category.
    async fn get_news(
        &self,
        category: NewsCategory,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<NewsArticle>> {
        let column = category.to_column_code();
        let url = "https://newsapi.eastmoney.com/kuaixun/v2/api/list";

        let params = [
            ("column", column),
            ("limit", &limit.to_string()),
            ("p", &page.to_string()),
        ];

        debug!(
            "Fetching news from East Money: category={:?}, page={}",
            category, page
        );

        let response: NewsResponse = self.request.get_json_with_params(url, &params).await?;

        if response.rc != 1 {
            return Ok(Vec::new());
        }

        let items = response.news.unwrap_or_default();
        let mut articles = Vec::with_capacity(items.len());

        for item in items {
            let publish_time =
                chrono::NaiveDateTime::parse_from_str(&item.showtime, "%Y-%m-%d %H:%M:%S")
                    .ok()
                    .and_then(|dt| beijing_tz().from_local_datetime(&dt).single())
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now);

            let comment_count: u32 = item.commentnum.parse().unwrap_or(0);
            let has_image = !item.image.is_empty() || !item.image_url.is_empty();
            let image_url = if !item.image_url.is_empty() {
                Some(item.image_url)
            } else if !item.image.is_empty() {
                Some(item.image)
            } else {
                None
            };

            articles.push(NewsArticle {
                id: item.id,
                title: item.title,
                digest: item.digest,
                url: item.url_w,
                url_mobile: if item.url_m.is_empty() {
                    None
                } else {
                    Some(item.url_m)
                },
                source: item.media_name,
                publish_time,
                category,
                comment_count,
                has_image,
                image_url,
            });
        }

        Ok(articles)
    }

    /// Fetches full news content by ID.
    async fn get_news_content(&self, news_id: &str) -> DataResult<NewsContent> {
        let url = "https://newsinfo.eastmoney.com/kuaixun/v2/api/content";
        let params = [("newsid", news_id)];

        debug!("Fetching news content from East Money: id={}", news_id);

        #[derive(Deserialize)]
        struct RelatedStock {
            #[serde(rename = "Code", default)]
            code: String,
        }

        #[derive(Deserialize)]
        struct ContentResponse {
            newsid: String,
            title: String,
            #[serde(default)]
            description: String,
            #[serde(default)]
            body: String,
            #[serde(default)]
            source: String,
            #[serde(default)]
            author: String,
            #[serde(default)]
            showtime: String,
            #[serde(default)]
            relatedstocks: Vec<RelatedStock>,
            #[serde(default, deserialize_with = "deserialize_images")]
            images: Vec<String>,
        }

        fn deserialize_images<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            use serde::de::{SeqAccess, Visitor};
            use std::fmt;

            struct ImagesVisitor;

            impl<'de> Visitor<'de> for ImagesVisitor {
                type Value = Vec<String>;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a sequence of strings or image objects")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    let mut images = Vec::new();
                    while let Some(value) = seq.next_element::<serde_json::Value>()? {
                        match value {
                            serde_json::Value::String(s) => images.push(s),
                            serde_json::Value::Object(obj) => {
                                if let Some(serde_json::Value::String(src)) = obj.get("src") {
                                    images.push(src.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                    Ok(images)
                }
            }

            deserializer.deserialize_seq(ImagesVisitor)
        }

        let response: ContentResponse = self.request.get_json_with_params(url, &params).await?;

        let publish_time =
            chrono::NaiveDateTime::parse_from_str(&response.showtime, "%Y-%m-%d %H:%M:%S")
                .ok()
                .and_then(|dt| beijing_tz().from_local_datetime(&dt).single())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

        let body_text = html2text::from_read(response.body.as_bytes(), usize::MAX)
            .unwrap_or_default()
            .trim()
            .to_string();

        let related_stocks: Vec<String> = response
            .relatedstocks
            .into_iter()
            .flat_map(|rs| rs.code.split(',').map(String::from).collect::<Vec<_>>())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(NewsContent {
            id: response.newsid,
            title: response.title,
            description: response.description,
            body_html: response.body,
            body_text,
            source: response.source,
            author: if response.author.is_empty() {
                None
            } else {
                Some(response.author)
            },
            publish_time,
            related_stocks,
            images: response.images,
        })
    }

    /// Searches news articles by keyword.
    async fn search_news(
        &self,
        keyword: &str,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<NewsArticle>> {
        let url = "https://search-api-web.eastmoney.com/search/jsonp";

        let param = serde_json::json!({
            "uid": "",
            "keyword": keyword,
            "type": ["cmsArticleWebOld"],
            "client": "web",
            "clientType": "web",
            "clientVersion": "curr",
            "param": {
                "cmsArticleWebOld": {
                    "searchScope": "default",
                    "sort": "time",
                    "pageIndex": page,
                    "pageSize": limit,
                    "preTag": "",
                    "postTag": ""
                }
            }
        });

        let callback = format!("jQuery_{}", chrono::Utc::now().timestamp_millis());
        let params = [("cb", callback.as_str()), ("param", &param.to_string())];

        debug!(
            "Searching news from East Money: keyword={}, page={}",
            keyword, page
        );

        let response = self.request.get_with_params(url, &params).await?;
        let response_text = response.text().await.map_err(|e| {
            crate::error::DataError::custom(format!("Failed to read response: {}", e))
        })?;

        let prefix = format!("{}(", callback);
        let json_str = response_text
            .trim()
            .strip_prefix(&prefix)
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or(&response_text);

        #[derive(Deserialize)]
        struct SearchResponse {
            #[serde(default)]
            result: Option<SearchResult>,
        }

        #[derive(Deserialize)]
        struct SearchResult {
            #[serde(rename = "cmsArticleWebOld", default)]
            articles: Vec<SearchArticle>,
        }

        #[derive(Deserialize)]
        struct SearchArticle {
            #[serde(default)]
            code: String,
            #[serde(default)]
            title: String,
            #[serde(default)]
            content: String,
            #[serde(default)]
            url: String,
            #[serde(rename = "mediaName", default)]
            media_name: String,
            #[serde(default)]
            date: String,
            #[serde(rename = "imgUrl", default)]
            img_url: String,
        }

        let search_response: SearchResponse = serde_json::from_str(json_str).map_err(|e| {
            crate::error::DataError::custom(format!("Failed to parse search response: {}", e))
        })?;

        let items = search_response
            .result
            .map(|r| r.articles)
            .unwrap_or_default();

        let mut articles = Vec::with_capacity(items.len());

        for item in items {
            let publish_time =
                chrono::NaiveDateTime::parse_from_str(&item.date, "%Y-%m-%d %H:%M:%S")
                    .or_else(|_| chrono::NaiveDateTime::parse_from_str(&item.date, "%Y-%m-%d"))
                    .ok()
                    .and_then(|dt| beijing_tz().from_local_datetime(&dt).single())
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now);

            let has_image = !item.img_url.is_empty();

            articles.push(NewsArticle {
                id: item.code,
                title: item.title,
                digest: item.content,
                url: item.url,
                url_mobile: None,
                source: item.media_name,
                publish_time,
                category: NewsCategory::Finance,
                comment_count: 0,
                has_image,
                image_url: if has_image { Some(item.img_url) } else { None },
            });
        }

        Ok(articles)
    }

    async fn search_news_for_symbol(
        &self,
        symbol: &StockCode,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<NewsArticle>> {
        use std::collections::HashSet;

        let mut articles = Vec::new();
        let mut seen = HashSet::new();

        if !symbol.short_name.is_empty() {
            let batch = self.search_news(&symbol.short_name, page, limit).await?;
            for article in batch {
                if seen.insert(article.id.clone()) {
                    articles.push(article);
                }
            }
        }

        if articles.len() < limit as usize && !symbol.stock_code.is_empty() {
            let fetch_limit = limit.saturating_mul(3).max(limit).min(100);
            let batch = self
                .search_news(&symbol.stock_code, page, fetch_limit)
                .await?;
            for article in filter_news_for_symbol(&batch, symbol) {
                if seen.insert(article.id.clone()) {
                    articles.push(article);
                }
            }
        }

        if articles.len() < limit as usize
            && symbol.exchange != Exchange::Unknown
            && !symbol.stock_code.is_empty()
        {
            let tagged = format!("{}.{}", symbol.stock_code, symbol.exchange);
            let batch = self.search_news(&tagged, page, limit).await?;
            for article in batch {
                if seen.insert(article.id.clone()) {
                    articles.push(article);
                }
            }
        }

        articles.truncate(limit as usize);
        sort_news_by_time_desc(&mut articles);
        Ok(articles)
    }
}
