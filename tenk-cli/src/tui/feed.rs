use rust_i18n::t;
use tenk::{
    InstitutionalResearchData, NewsArticle, NewsCategory, ResearchReportData, SourceKind,
    TvAnalystData, TvAdvice, TvTechnicalAnalysis,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedScope {
    Symbol,
    Market,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedKind {
    SymbolNews,
    ResearchReports,
    MarketNews(NewsCategory),
    InstitutionalResearch,
    TechnicalAnalysis,
    AnalystConsensus,
}

impl FeedKind {
    pub fn scope(self) -> FeedScope {
        match self {
            Self::SymbolNews
            | Self::ResearchReports
            | Self::TechnicalAnalysis
            | Self::AnalystConsensus => FeedScope::Symbol,
            Self::MarketNews(_) | Self::InstitutionalResearch => FeedScope::Market,
        }
    }

    pub fn title(self) -> String {
        match self {
            Self::SymbolNews => t!("tui.feeds.symbol_news").to_string(),
            Self::ResearchReports => t!("tui.feeds.research").to_string(),
            Self::MarketNews(NewsCategory::Stock) => t!("tui.feeds.market_stock").to_string(),
            Self::MarketNews(NewsCategory::Finance) => t!("tui.feeds.market_finance").to_string(),
            Self::MarketNews(NewsCategory::Company) => t!("tui.feeds.market_company").to_string(),
            Self::MarketNews(NewsCategory::USMarket) => t!("tui.feeds.market_us").to_string(),
            Self::MarketNews(NewsCategory::Global) => t!("tui.feeds.market_global").to_string(),
            Self::MarketNews(NewsCategory::Domestic) => t!("tui.feeds.market_domestic").to_string(),
            Self::MarketNews(NewsCategory::Industry) => t!("tui.feeds.market_industry").to_string(),
            Self::InstitutionalResearch => t!("tui.feeds.institutional").to_string(),
            Self::TechnicalAnalysis => t!("tui.feeds.ta").to_string(),
            Self::AnalystConsensus => t!("tui.feeds.analyst").to_string(),
        }
    }

    pub fn opens_news_dialog(self) -> bool {
        matches!(self, Self::SymbolNews | Self::MarketNews(_))
    }

    pub fn opens_report_dialog(self) -> bool {
        self == Self::ResearchReports
    }
}

pub fn feed_kinds(source: SourceKind) -> &'static [FeedKind] {
    match source {
        SourceKind::Eastmoney => &[
            FeedKind::SymbolNews,
            FeedKind::ResearchReports,
            FeedKind::MarketNews(NewsCategory::Stock),
            FeedKind::MarketNews(NewsCategory::Finance),
            FeedKind::MarketNews(NewsCategory::USMarket),
            FeedKind::MarketNews(NewsCategory::Industry),
            FeedKind::InstitutionalResearch,
        ],
        SourceKind::Sina => &[
            FeedKind::SymbolNews,
            FeedKind::ResearchReports,
            FeedKind::MarketNews(NewsCategory::Stock),
            FeedKind::MarketNews(NewsCategory::USMarket),
            FeedKind::MarketNews(NewsCategory::Industry),
        ],
        SourceKind::Ths => &[
            FeedKind::SymbolNews,
            FeedKind::ResearchReports,
            FeedKind::MarketNews(NewsCategory::Stock),
        ],
        SourceKind::Tradingview => &[
            FeedKind::SymbolNews,
            FeedKind::MarketNews(NewsCategory::Stock),
            FeedKind::MarketNews(NewsCategory::USMarket),
            FeedKind::TechnicalAnalysis,
            FeedKind::AnalystConsensus,
        ],
    }
}

pub const FEED_PAGE_SIZE: u32 = 50;

#[derive(Debug, Clone)]
pub enum FeedData {
    Empty,
    Articles(Vec<NewsArticle>),
    Reports(Vec<ResearchReportData>),
    Institutional(Vec<InstitutionalResearchData>),
    TechnicalAnalysis(TvTechnicalAnalysis),
    Analyst(TvAnalystData),
}

impl FeedData {
    pub fn list_len(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Articles(items) => items.len(),
            Self::Reports(items) => items.len(),
            Self::Institutional(items) => items.len(),
            Self::TechnicalAnalysis(ta) => ta.periods.len().max(1),
            Self::Analyst(_) => 1,
        }
    }
}

pub struct FeedPanel {
    pub kind: FeedKind,
    pub data: FeedData,
    pub scroll: usize,
    pub selected: usize,
    pub page: u32,
    pub loading: bool,
}

impl FeedPanel {
    pub fn new(kind: FeedKind) -> Self {
        Self {
            kind,
            data: FeedData::Empty,
            scroll: 0,
            selected: 0,
            page: 1,
            loading: false,
        }
    }

    pub fn supports_paging(&self) -> bool {
        !matches!(
            self.kind,
            FeedKind::TechnicalAnalysis | FeedKind::AnalystConsensus
        )
    }

    pub fn page_title(&self) -> String {
        let base = self.kind.title();
        if self.supports_paging() {
            format!("{base} · {}", self.page)
        } else {
            base
        }
    }

    pub fn has_next_page(&self) -> bool {
        self.data.list_len() >= FEED_PAGE_SIZE as usize
    }

    pub fn has_prev_page(&self) -> bool {
        self.page > 1
    }

    pub fn clear(&mut self) {
        self.data = FeedData::Empty;
        self.scroll = 0;
        self.selected = 0;
    }

    pub fn reset_page(&mut self) {
        self.page = 1;
    }

    pub fn selected_news(&self) -> Option<&NewsArticle> {
        match &self.data {
            FeedData::Articles(items) => items.get(self.selected),
            _ => None,
        }
    }

    pub fn selected_report(&self) -> Option<&ResearchReportData> {
        match &self.data {
            FeedData::Reports(items) => items.get(self.selected),
            _ => None,
        }
    }
}

pub fn advice_label(advice: TvAdvice) -> &'static str {
    match advice {
        TvAdvice::StrongSell => "Strong Sell",
        TvAdvice::Sell => "Sell",
        TvAdvice::Neutral => "Neutral",
        TvAdvice::Buy => "Buy",
        TvAdvice::StrongBuy => "Strong Buy",
    }
}
