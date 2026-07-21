//! News data structures.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// News category for filtering news articles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum NewsCategory {
    /// 财经要闻 - General finance news
    #[default]
    Finance,
    /// 上市公司 - Listed company news
    Company,
    /// 证券要闻 - Stock market news
    Stock,
    /// 美股 - US market news
    USMarket,
    /// 国际经济 - Global economy news
    Global,
    /// 国内经济 - Domestic economy news
    Domestic,
    /// 产经资讯 - China sector and macro flash news (kuaixun column 110, dq_zg)
    Industry,
}

impl NewsCategory {
    /// Get the column code for EastMoney API.
    pub fn to_column_code(&self) -> &'static str {
        match self {
            NewsCategory::Finance => "102",
            NewsCategory::Company => "103",
            NewsCategory::Stock => "104",
            NewsCategory::USMarket => "105",
            NewsCategory::Global => "111",
            NewsCategory::Domestic => "106",
            NewsCategory::Industry => "110",
        }
    }

    /// Parse from column code.
    pub fn from_column_code(code: &str) -> Option<Self> {
        match code {
            "102" => Some(NewsCategory::Finance),
            "103" => Some(NewsCategory::Company),
            "104" => Some(NewsCategory::Stock),
            "105" => Some(NewsCategory::USMarket),
            "111" => Some(NewsCategory::Global),
            "106" => Some(NewsCategory::Domestic),
            "110" => Some(NewsCategory::Industry),
            _ => None,
        }
    }

    /// Parses from CLI/MCP name or column code (defaults to finance).
    pub fn from_name(name: &str) -> Self {
        Self::from_column_code(name).unwrap_or_else(|| {
            match name.to_lowercase().as_str() {
                "finance" => NewsCategory::Finance,
                "company" => NewsCategory::Company,
                "stock" => NewsCategory::Stock,
                "us" | "usmarket" => NewsCategory::USMarket,
                "global" => NewsCategory::Global,
                "domestic" => NewsCategory::Domestic,
                "industry" => NewsCategory::Industry,
                _ => NewsCategory::Finance,
            }
        })
    }
}

impl std::fmt::Display for NewsCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            NewsCategory::Finance => "Finance",
            NewsCategory::Company => "Company",
            NewsCategory::Stock => "Stock",
            NewsCategory::USMarket => "US Market",
            NewsCategory::Global => "Global",
            NewsCategory::Domestic => "Domestic",
            NewsCategory::Industry => "Industry",
        };
        write!(f, "{}", name)
    }
}

/// News article.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsArticle {
    /// Unique news ID
    pub id: String,
    /// News title
    pub title: String,
    /// News summary/digest
    pub digest: String,
    /// Web URL
    pub url: String,
    /// Mobile URL
    pub url_mobile: Option<String>,
    /// Source/media name
    pub source: String,
    /// Publish time
    pub publish_time: DateTime<Utc>,
    /// Category
    pub category: NewsCategory,
    /// Comment count
    pub comment_count: u32,
    /// Whether the news has an image
    pub has_image: bool,
    /// Image URL
    pub image_url: Option<String>,
}

pub fn sort_news_by_time_desc(articles: &mut Vec<NewsArticle>) {
    articles.sort_by(|left, right| right.publish_time.cmp(&left.publish_time));
}

fn strip_html_tags(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output
}

pub fn news_matches_symbol(article: &NewsArticle, symbol: &crate::data::StockCode) -> bool {
    let title = strip_html_tags(&article.title);
    let digest = strip_html_tags(&article.digest);
    let haystacks = [title.as_str(), digest.as_str()];

    if !symbol.short_name.is_empty() {
        let needle = symbol.short_name.as_str();
        if haystacks.iter().any(|text| text.contains(needle)) {
            return true;
        }
    }

    let code = symbol.stock_code.as_str();
    if haystacks.iter().any(|text| text.contains(code)) {
        return true;
    }

    let trimmed = code.trim_start_matches('0');
    if !trimmed.is_empty()
        && trimmed != code
        && haystacks.iter().any(|text| text.contains(trimmed))
    {
        return true;
    }

    if symbol.exchange != crate::data::Exchange::Unknown {
        let tagged = format!("{}.{}", code, symbol.exchange);
        if haystacks.iter().any(|text| text.contains(tagged.as_str())) {
            return true;
        }
    }

    false
}

pub fn filter_news_for_symbol(
    articles: &[NewsArticle],
    symbol: &crate::data::StockCode,
) -> Vec<NewsArticle> {
    articles
        .iter()
        .filter(|article| news_matches_symbol(article, symbol))
        .cloned()
        .collect()
}

/// News search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsSearchResult {
    /// Search keyword
    pub keyword: String,
    /// Total count of results
    pub total_count: u32,
    /// Current page
    pub page: u32,
    /// Page size
    pub page_size: u32,
    /// News articles
    pub articles: Vec<NewsArticle>,
}

/// News list result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsListResult {
    /// Category
    pub category: NewsCategory,
    /// Current page
    pub page: u32,
    /// Total pages
    pub total_pages: u32,
    /// News articles
    pub articles: Vec<NewsArticle>,
}

/// Full news content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsContent {
    /// News ID
    pub id: String,
    /// News title
    pub title: String,
    /// News description/summary
    pub description: String,
    /// Full content (HTML)
    pub body_html: String,
    /// Plain text content (Text)
    pub body_text: String,
    /// Source/media name
    pub source: String,
    /// Author
    pub author: Option<String>,
    /// Publish time
    pub publish_time: DateTime<Utc>,
    /// Related stock codes
    pub related_stocks: Vec<String>,
    /// Images
    pub images: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    use crate::data::{Exchange, StockCode};

    #[test]
    fn test_to_column_code() {
        assert_eq!(NewsCategory::Finance.to_column_code(), "102");
        assert_eq!(NewsCategory::Industry.to_column_code(), "110");
    }

    #[test]
    fn test_from_column_code() {
        assert_eq!(
            NewsCategory::from_column_code("104"),
            Some(NewsCategory::Stock)
        );
        assert_eq!(NewsCategory::from_column_code("999"), None);
    }

    #[test]
    fn test_from_name_by_keyword() {
        assert_eq!(NewsCategory::from_name("company"), NewsCategory::Company);
        assert_eq!(NewsCategory::from_name("US"), NewsCategory::USMarket);
        assert_eq!(NewsCategory::from_name("usmarket"), NewsCategory::USMarket);
        assert_eq!(NewsCategory::from_name("unknown"), NewsCategory::Finance);
    }

    #[test]
    fn test_from_name_by_column_code() {
        assert_eq!(NewsCategory::from_name("110"), NewsCategory::Industry);
    }

    #[test]
    fn test_display() {
        assert_eq!(NewsCategory::USMarket.to_string(), "US Market");
        assert_eq!(NewsCategory::Finance.to_string(), "Finance");
    }

    #[test]
    fn sort_news_by_time_desc_orders_newest_first() {
        let mut articles = vec![
            NewsArticle {
                id: "1".into(),
                title: "older".into(),
                digest: String::new(),
                url: String::new(),
                url_mobile: None,
                source: String::new(),
                publish_time: Utc.with_ymd_and_hms(2026, 7, 20, 10, 0, 0).unwrap(),
                category: NewsCategory::Finance,
                comment_count: 0,
                has_image: false,
                image_url: None,
            },
            NewsArticle {
                id: "2".into(),
                title: "newer".into(),
                digest: String::new(),
                url: String::new(),
                url_mobile: None,
                source: String::new(),
                publish_time: Utc.with_ymd_and_hms(2026, 7, 20, 12, 0, 0).unwrap(),
                category: NewsCategory::Finance,
                comment_count: 0,
                has_image: false,
                image_url: None,
            },
        ];
        sort_news_by_time_desc(&mut articles);
        assert_eq!(articles[0].title, "newer");
        assert_eq!(articles[1].title, "older");
    }

    #[test]
    fn news_matches_symbol_by_name_and_code() {
        let symbol = StockCode::new(
            "600519".into(),
            "贵州茅台".into(),
            Exchange::SH,
        );
        let matched = NewsArticle {
            id: "1".into(),
            title: "贵州茅台发布业绩预告".into(),
            digest: String::new(),
            url: String::new(),
            url_mobile: None,
            source: String::new(),
            publish_time: Utc.with_ymd_and_hms(2026, 7, 20, 12, 0, 0).unwrap(),
            category: NewsCategory::Finance,
            comment_count: 0,
            has_image: false,
            image_url: None,
        };
        let unrelated = NewsArticle {
            id: "2".into(),
            title: "A股尾盘反抽，三大股指涨跌互现".into(),
            digest: String::new(),
            url: String::new(),
            url_mobile: None,
            source: String::new(),
            publish_time: Utc.with_ymd_and_hms(2026, 7, 20, 11, 0, 0).unwrap(),
            category: NewsCategory::Finance,
            comment_count: 0,
            has_image: false,
            image_url: None,
        };
        assert!(news_matches_symbol(&matched, &symbol));
        assert!(!news_matches_symbol(&unrelated, &symbol));
    }
}
