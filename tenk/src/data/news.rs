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
    /// 产经资讯 - Industry news
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
            NewsCategory::Industry => "115",
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
            "115" => Some(NewsCategory::Industry),
            _ => None,
        }
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
