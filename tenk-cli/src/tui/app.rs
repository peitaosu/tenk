use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rust_i18n::t;
use tenk::{
    CurrentMarketData, DataClient, Exchange, KLineType, MarketData, MinuteData, NewsContent,
    ResearchReportData, SourceKind, StockCode, StockSearchHit, StockValuation,
};
use tokio::sync::mpsc;

use super::config;
use super::feed::{feed_kinds, FeedData, FeedKind, FeedPanel, FeedScope};
use super::fetch::{
    FetchMsg, spawn_feed, spawn_kline_refresh, spawn_market_feeds, spawn_news_content,
    spawn_stock_search, spawn_symbol, spawn_symbol_feeds, spawn_watchlist,
};
use super::kline::{KlineView, TimelineScope};
use super::theme::UiTheme;

const POLL_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Watchlist,
    Quote,
    Kline,
    Feed(usize),
}

pub enum Overlay {
    None,
    Command(String),
    Help,
}

pub enum Dialog {
    News {
        loading: bool,
        scroll: usize,
        content: Option<NewsContent>,
    },
    Research(ResearchReportData),
}

pub struct StockSearch {
    pub input: String,
    pub selected: usize,
    pub results: Vec<StockSearchHit>,
    pub loading: bool,
    fetch_id: u64,
}

pub struct App {
    pub source: SourceKind,
    pub client: Arc<DataClient>,
    pub tx: mpsc::UnboundedSender<FetchMsg>,
    pub watchlist: Vec<StockCode>,
    pub selected: usize,
    pub focus: Focus,
    pub overlay: Overlay,
    pub dialog: Option<Dialog>,
    pub stock_search: Option<StockSearch>,
    pub quotes: HashMap<String, CurrentMarketData>,
    pub quote: Option<CurrentMarketData>,
    pub valuation: Option<StockValuation>,
    pub kline: Vec<MarketData>,
    pub intraday: Vec<MinuteData>,
    pub kline_type: KLineType,
    pub kline_view: KlineView,
    pub timeline_scope: TimelineScope,
    pub timeline_day: usize,
    pub kline_scroll: usize,
    pub quote_scroll: usize,
    pub feeds: Vec<FeedPanel>,
    pub status: String,
    pub loading: bool,
    pub theme: UiTheme,
    pub quit: bool,
    fetch_id: u64,
    pending: u8,
    last_poll: Instant,
}

impl App {
    pub fn new(client: DataClient, source: SourceKind, tx: mpsc::UnboundedSender<FetchMsg>) -> Self {
        let watchlist = config::load_watchlist();
        let feeds = feed_kinds(source)
            .iter()
            .copied()
            .map(FeedPanel::new)
            .collect();
        Self {
            source,
            client: Arc::new(client),
            tx,
            watchlist,
            selected: 0,
            focus: Focus::Watchlist,
            overlay: Overlay::None,
            dialog: None,
            stock_search: None,
            quotes: HashMap::new(),
            quote: None,
            valuation: None,
            kline: vec![],
            intraday: vec![],
            kline_type: KLineType::Daily,
            kline_view: KlineView::Timeline,
            timeline_scope: TimelineScope::Day,
            timeline_day: 0,
            kline_scroll: 0,
            quote_scroll: 0,
            feeds,
            status: String::new(),
            loading: false,
            theme: UiTheme::detect(),
            quit: false,
            fetch_id: 0,
            pending: 0,
            last_poll: Instant::now() - POLL_INTERVAL,
        }
    }

    fn focus_order(&self) -> Vec<Focus> {
        let mut order = vec![Focus::Watchlist, Focus::Quote, Focus::Kline];
        for index in 0..self.feeds.len() {
            order.push(Focus::Feed(index));
        }
        order
    }

    fn focus_next(&self) -> Focus {
        let order = self.focus_order();
        let pos = order.iter().position(|&item| item == self.focus).unwrap_or(0);
        order[(pos + 1) % order.len()]
    }

    fn focus_prev(&self) -> Focus {
        let order = self.focus_order();
        let pos = order.iter().position(|&item| item == self.focus).unwrap_or(0);
        order[(pos + order.len() - 1) % order.len()]
    }

    fn focus_from_digit(&self, digit: usize) -> Option<Focus> {
        match digit {
            1 => Some(Focus::Watchlist),
            2 => Some(Focus::Quote),
            3 => Some(Focus::Kline),
            n if n >= 4 && n <= 3 + self.feeds.len() => Some(Focus::Feed(n - 4)),
            _ => None,
        }
    }

    pub fn source_label(&self) -> &'static str {
        match self.source {
            SourceKind::Eastmoney => "EM",
            SourceKind::Sina => "Sina",
            SourceKind::Ths => "THS",
            SourceKind::Tradingview => "TV",
        }
    }

    fn symbol_feed_indices(&self) -> Vec<(usize, FeedKind, u32)> {
        self.feeds
            .iter()
            .enumerate()
            .filter(|(_, panel)| panel.kind.scope() == FeedScope::Symbol)
            .map(|(index, panel)| (index, panel.kind, panel.page))
            .collect()
    }

    fn market_feed_indices(&self) -> Vec<(usize, FeedKind, u32)> {
        self.feeds
            .iter()
            .enumerate()
            .filter(|(_, panel)| panel.kind.scope() == FeedScope::Market)
            .map(|(index, panel)| (index, panel.kind, panel.page))
            .collect()
    }

    fn start_symbol_fetch(&mut self) {
        let Some(symbol) = self.current_symbol().cloned() else {
            return;
        };
        self.fetch_id += 1;
        let fetch_id = self.fetch_id;
        let symbol_feed_count = self.symbol_feed_indices().len() as u8;
        self.pending = 3 + symbol_feed_count;
        self.loading = true;
        self.kline.clear();
        self.intraday.clear();
        self.timeline_day = 0;
        self.timeline_scope = TimelineScope::Day;
        for panel in &mut self.feeds {
            if panel.kind.scope() == FeedScope::Symbol {
                panel.reset_page();
                panel.clear();
                panel.loading = true;
            }
        }
        self.quote_scroll = 0;
        self.status = t!("tui.status.loading").to_string();
        spawn_symbol(
            self.client.clone(),
            symbol.clone(),
            self.kline_type,
            fetch_id,
            self.tx.clone(),
        );
        spawn_symbol_feeds(
            self.client.clone(),
            symbol,
            fetch_id,
            self.symbol_feed_indices(),
            self.tx.clone(),
        );
    }

    fn finish_fetch(&mut self, fetch_id: u64) {
        if fetch_id != self.fetch_id {
            return;
        }
        self.pending = self.pending.saturating_sub(1);
        if self.pending == 0 {
            self.loading = false;
            self.status = t!("tui.status.live").to_string();
        }
    }

    pub fn current_symbol(&self) -> Option<&StockCode> {
        self.watchlist.get(self.selected)
    }

    pub fn current_code(&self) -> Option<&str> {
        self.current_symbol().map(|symbol| symbol.stock_code.as_str())
    }

    fn sync_watchlist_name(&mut self, quote: &CurrentMarketData) {
        let Some(entry) = self.watchlist.get_mut(self.selected) else {
            return;
        };
        if entry.stock_code != quote.stock_code {
            return;
        }
        if entry.short_name.is_empty() && !quote.short_name.is_empty() {
            entry.short_name = quote.short_name.clone();
            let _ = config::save_watchlist(&self.watchlist);
        }
    }

    pub fn poll_if_due(&mut self) {
        if self.stock_search.is_some() {
            return;
        }
        if self.last_poll.elapsed() >= POLL_INTERVAL {
            self.refresh_watchlist();
            self.refresh_kline_poll();
            self.last_poll = Instant::now();
        }
    }

    fn refresh_kline_poll(&mut self) {
        let Some(symbol) = self.current_symbol().cloned() else {
            return;
        };
        spawn_kline_refresh(
            self.client.clone(),
            symbol,
            self.kline_view,
            self.kline_type,
            self.timeline_scope,
            self.tx.clone(),
        );
    }

    fn kline_scroll_max(&self) -> usize {
        self.kline_scroll_len().saturating_sub(1)
    }

    fn kline_scroll_len(&self) -> usize {
        match self.kline_view {
            KlineView::Timeline => self.visible_intraday().len(),
            _ => self.kline.len(),
        }
    }

    pub fn visible_intraday(&self) -> Vec<MinuteData> {
        super::kline::filter_intraday(&self.intraday, self.timeline_scope, self.timeline_day)
    }

    fn clamp_timeline_day(&mut self) {
        let max = super::kline::intraday_dates(&self.intraday)
            .len()
            .saturating_sub(1);
        if self.timeline_day > max {
            self.timeline_day = max;
        }
    }

    fn merge_kline_poll(&mut self, kline: Vec<MarketData>) {
        let update_scroll = matches!(self.kline_view, KlineView::Chart | KlineView::Table);
        let at_end = update_scroll && self.kline_scroll + 1 >= self.kline.len().max(1);
        self.kline = kline;
        if update_scroll && at_end {
            self.kline_scroll = self.kline.len().saturating_sub(1);
        }
    }

    fn merge_intraday_poll(&mut self, data: Vec<MinuteData>) {
        let update_scroll = self.kline_view == KlineView::Timeline;
        let at_end = if update_scroll {
            let visible_len = super::kline::filter_intraday(
                &data,
                self.timeline_scope,
                self.timeline_day,
            )
            .len();
            self.kline_scroll + 1 >= visible_len.max(1)
        } else {
            false
        };
        self.intraday = data;
        self.clamp_timeline_day();
        if update_scroll {
            let visible_len = self.visible_intraday().len();
            if at_end {
                self.kline_scroll = visible_len.saturating_sub(1);
            } else {
                self.kline_scroll = self.kline_scroll.min(visible_len.saturating_sub(1));
            }
        }
    }

    pub fn refresh_watchlist(&mut self) {
        spawn_watchlist(
            self.client.clone(),
            self.watchlist.clone(),
            self.tx.clone(),
        );
    }

    pub fn refresh_symbol(&mut self) {
        self.start_symbol_fetch();
    }

    pub fn refresh_kline(&mut self) {
        self.refresh_kline_poll();
    }

    fn refresh_market_feeds(&mut self) {
        let panels = self.market_feed_indices();
        for (index, _, _) in &panels {
            if let Some(panel) = self.feeds.get_mut(*index) {
                panel.loading = true;
            }
        }
        spawn_market_feeds(self.client.clone(), panels, self.tx.clone());
    }

    fn refresh_focused_feed(&mut self, index: usize) {
        let Some(kind) = self.feeds.get(index).map(|panel| panel.kind) else {
            return;
        };
        let page = self.feeds.get(index).map(|panel| panel.page).unwrap_or(1);
        if let Some(panel) = self.feeds.get_mut(index) {
            panel.loading = true;
            panel.clear();
        }
        if kind.scope() == FeedScope::Symbol {
            let Some(symbol) = self.current_symbol().cloned() else {
                return;
            };
            self.fetch_id += 1;
            let fetch_id = self.fetch_id;
            self.pending = self.pending.saturating_add(1);
            self.loading = true;
            self.status = t!("tui.status.loading").to_string();
            spawn_feed(
                self.client.clone(),
                index,
                kind,
                Some(symbol),
                page,
                fetch_id,
                self.tx.clone(),
            );
        } else {
            spawn_feed(
                self.client.clone(),
                index,
                kind,
                None,
                page,
                0,
                self.tx.clone(),
            );
        }
    }

    fn feed_page_prev(&mut self, index: usize) {
        let Some(panel) = self.feeds.get(index) else {
            return;
        };
        if !panel.supports_paging() || !panel.has_prev_page() {
            return;
        }
        self.feeds[index].page -= 1;
        self.refresh_focused_feed(index);
    }

    fn feed_page_next(&mut self, index: usize) {
        let Some(panel) = self.feeds.get(index) else {
            return;
        };
        if !panel.supports_paging() || !panel.has_next_page() {
            return;
        }
        self.feeds[index].page += 1;
        self.refresh_focused_feed(index);
    }

    fn apply_feed(&mut self, index: usize, fetch_id: u64, data: FeedData) {
        if fetch_id != 0 {
            if self.current_code().is_none() || fetch_id != self.fetch_id {
                return;
            }
        }
        let Some(panel) = self.feeds.get_mut(index) else {
            return;
        };
        panel.loading = false;
        panel.data = data;
        panel.selected = 0;
        panel.scroll = 0;
        if fetch_id != 0 {
            self.finish_fetch(fetch_id);
        }
    }

    fn scroll_feed_down(&mut self, index: usize) {
        let Some(panel) = self.feeds.get(index) else {
            return;
        };
        let len = panel.data.list_len();
        if panel.selected + 1 >= len {
            return;
        }
        let panel = &mut self.feeds[index];
        panel.selected += 1;
        if panel.selected >= panel.scroll + 1 {
            panel.scroll = panel.selected;
        }
    }

    fn scroll_feed_up(&mut self, index: usize) {
        let Some(panel) = self.feeds.get(index) else {
            return;
        };
        if panel.selected == 0 {
            return;
        }
        let panel = &mut self.feeds[index];
        panel.selected -= 1;
        if panel.selected < panel.scroll {
            panel.scroll = panel.selected;
        }
    }

    pub fn apply_fetch(&mut self, msg: FetchMsg) {
        match msg {
            FetchMsg::Watchlist(data) => {
                for q in data {
                    self.quotes.insert(q.stock_code.clone(), q.clone());
                }
                if let Some(sym) = self.current_code() {
                    if self.quote.is_none() || self.loading {
                        self.quote = self.quotes.get(sym).cloned();
                    }
                }
                if !self.loading {
                    self.status = t!("tui.status.live").to_string();
                }
            }
            FetchMsg::Detail {
                fetch_id,
                symbol,
                quote,
                valuation,
            } => {
                if self.current_code() != Some(symbol.as_str()) || fetch_id != self.fetch_id {
                    return;
                }
                if let Some(q) = quote.as_ref() {
                    self.quote = quote.clone();
                    self.sync_watchlist_name(q);
                }
                if valuation.is_some() {
                    self.valuation = valuation;
                }
                self.finish_fetch(fetch_id);
            }
            FetchMsg::Kline { fetch_id, symbol, kline } => {
                if self.current_code() != Some(symbol.as_str()) || fetch_id != self.fetch_id {
                    return;
                }
                self.kline = kline;
                self.kline_scroll = self.kline.len().saturating_sub(1);
                self.finish_fetch(fetch_id);
            }
            FetchMsg::Intraday { fetch_id, symbol, data } => {
                if self.current_code() != Some(symbol.as_str()) || fetch_id != self.fetch_id {
                    return;
                }
                self.intraday = data;
                self.clamp_timeline_day();
                self.kline_scroll = self.visible_intraday().len().saturating_sub(1);
                self.finish_fetch(fetch_id);
            }
            FetchMsg::KlinePoll { symbol, kline } => {
                if self.current_code() != Some(symbol.as_str()) {
                    return;
                }
                self.merge_kline_poll(kline);
            }
            FetchMsg::IntradayPoll { symbol, data } => {
                if self.current_code() != Some(symbol.as_str()) {
                    return;
                }
                self.merge_intraday_poll(data);
            }
            FetchMsg::Feed { index, fetch_id, data } => {
                self.apply_feed(index, fetch_id, data);
            }
            FetchMsg::NewsContent(content) => {
                if let Some(Dialog::News { content: slot, loading, .. }) = &mut self.dialog {
                    *slot = Some(content);
                    *loading = false;
                }
            }
            FetchMsg::StockSearch {
                fetch_id,
                query,
                results,
            } => {
                let Some(search) = &mut self.stock_search else {
                    return;
                };
                if search.fetch_id != fetch_id || search.input.trim() != query {
                    return;
                }
                search.results = results;
                search.loading = false;
                search.selected = search.selected.min(search.results.len().saturating_sub(1));
            }
            FetchMsg::Error { fetch_id, message } => {
                if fetch_id != 0 && fetch_id != self.fetch_id {
                    return;
                }
                if fetch_id != 0 {
                    self.finish_fetch(fetch_id);
                }
                if !message.is_empty() {
                    self.status = message;
                }
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return true;
        }

        if self.dialog.is_some() {
            return self.handle_dialog_key(key);
        }

        if self.stock_search.is_some() {
            return self.handle_stock_search_key(key);
        }

        match &mut self.overlay {
            Overlay::Help => {
                if matches!(key.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')) {
                    self.overlay = Overlay::None;
                }
                return false;
            }
            Overlay::Command(_) => return self.handle_command_key(key),
            Overlay::None => {}
        }

        match key.code {
            KeyCode::Char('q') => {
                self.quit = true;
                return true;
            }
            KeyCode::Char('?') => {
                self.overlay = Overlay::Help;
            }
            KeyCode::Char(':') => {
                self.overlay = Overlay::Command(String::new());
            }
            KeyCode::Tab => {
                self.focus = self.focus_next();
            }
            KeyCode::BackTab => {
                self.focus = self.focus_prev();
            }
            KeyCode::Char(c @ '1'..='9') => {
                let digit = (c as u8 - b'0') as usize;
                if let Some(focus) = self.focus_from_digit(digit) {
                    self.focus = focus;
                }
            }
            KeyCode::Char('r') => self.refresh_focused(),
            KeyCode::Char('/') => {
                self.start_stock_search();
            }
            KeyCode::Char('d') if self.focus == Focus::Watchlist => {
                self.remove_selected();
            }
            KeyCode::Char('t') if self.focus == Focus::Kline => {
                self.cycle_kline_period();
            }
            KeyCode::Char('v') if self.focus == Focus::Kline => {
                self.cycle_kline_mode(true);
            }
            KeyCode::Enter => self.activate(),
            KeyCode::Char('j') | KeyCode::Down => self.scroll_down(),
            KeyCode::Char('k') | KeyCode::Up => self.scroll_up(),
            KeyCode::Left => match self.focus {
                Focus::Kline => self.cycle_kline_mode(false),
                Focus::Feed(index) => self.feed_page_prev(index),
                _ => {}
            },
            KeyCode::Right => match self.focus {
                Focus::Kline => self.cycle_kline_mode(true),
                Focus::Feed(index) => self.feed_page_next(index),
                _ => {}
            },
            _ => {}
        }
        false
    }

    fn handle_stock_search_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => self.end_stock_search(),
            KeyCode::Enter => self.pick_stock_search(),
            KeyCode::Backspace => {
                if let Some(search) = &mut self.stock_search {
                    search.input.pop();
                    self.request_stock_search();
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(search) = &mut self.stock_search {
                    if search.selected + 1 < search.results.len() {
                        search.selected += 1;
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(search) = &mut self.stock_search {
                    search.selected = search.selected.saturating_sub(1);
                }
            }
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(search) = &mut self.stock_search {
                    search.input.push(ch);
                    self.request_stock_search();
                }
            }
            _ => {}
        }
        false
    }

    fn start_stock_search(&mut self) {
        self.focus = Focus::Watchlist;
        self.stock_search = Some(StockSearch {
            input: String::new(),
            selected: 0,
            results: Vec::new(),
            loading: false,
            fetch_id: 0,
        });
    }

    fn end_stock_search(&mut self) {
        self.stock_search = None;
    }

    fn request_stock_search(&mut self) {
        let Some(search) = &mut self.stock_search else {
            return;
        };
        search.fetch_id += 1;
        search.loading = !search.input.trim().is_empty();
        search.selected = 0;
        let fetch_id = search.fetch_id;
        let query = search.input.clone();
        spawn_stock_search(self.client.clone(), query, fetch_id, self.tx.clone());
    }

    fn pick_stock_search(&mut self) {
        let hit = self
            .stock_search
            .as_ref()
            .and_then(|search| search.results.get(search.selected).cloned());
        let Some(hit) = hit else {
            self.end_stock_search();
            return;
        };
        self.end_stock_search();
        self.add_symbol(hit.to_stock_code());
    }

    fn handle_dialog_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.dialog = None;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(Dialog::News { scroll, .. }) = &mut self.dialog {
                    *scroll = scroll.saturating_add(1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(Dialog::News { scroll, .. }) = &mut self.dialog {
                    *scroll = scroll.saturating_sub(1);
                }
            }
            KeyCode::PageDown => {
                if let Some(Dialog::News { scroll, .. }) = &mut self.dialog {
                    *scroll = scroll.saturating_add(10);
                }
            }
            KeyCode::PageUp => {
                if let Some(Dialog::News { scroll, .. }) = &mut self.dialog {
                    *scroll = scroll.saturating_sub(10);
                }
            }
            _ => {}
        }
        false
    }

    fn handle_command_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.overlay = Overlay::None;
            }
            KeyCode::Enter => {
                let input = match &self.overlay {
                    Overlay::Command(buf) => buf.trim().to_string(),
                    _ => return false,
                };
                self.overlay = Overlay::None;
                self.run_command(&input);
            }
            KeyCode::Backspace => {
                if let Overlay::Command(buf) = &mut self.overlay {
                    buf.pop();
                }
            }
            KeyCode::Char(c) => {
                if let Overlay::Command(buf) = &mut self.overlay {
                    buf.push(c);
                }
            }
            _ => {}
        }
        false
    }

    fn run_command(&mut self, input: &str) {
        let input = input.strip_prefix(':').unwrap_or(input).trim();
        let mut parts = input.split_whitespace();
        let Some(cmd) = parts.next() else {
            return;
        };
        match cmd.to_ascii_lowercase().as_str() {
            "quit" | "q" => self.quit = true,
            "refresh" | "r" => self.refresh_focused(),
            "add" => {
                if let Some(sym) = parts.next() {
                    self.add_symbol_input(sym);
                }
            }
            "remove" | "del" => {
                if let Some(sym) = parts.next() {
                    self.remove_symbol_input(sym);
                }
            }
            sym if sym.contains(':') => {
                self.add_symbol_input(sym);
            }
            _ => self.status = t!("tui.status.unknown_cmd").to_string(),
        }
    }

    fn refresh_focused(&mut self) {
        match self.focus {
            Focus::Watchlist => self.refresh_watchlist(),
            Focus::Kline => self.refresh_kline(),
            Focus::Feed(index) => self.refresh_focused_feed(index),
            _ => self.refresh_symbol(),
        }
    }

    fn activate(&mut self) {
        match self.focus {
            Focus::Watchlist => self.refresh_symbol(),
            Focus::Feed(index) => {
                let Some(kind) = self.feeds.get(index).map(|panel| panel.kind) else {
                    return;
                };
                if kind.opens_news_dialog() {
                    self.open_news_dialog(index);
                } else if kind.opens_report_dialog() {
                    self.open_report_dialog(index);
                }
            }
            _ => {}
        }
    }

    fn open_news_dialog(&mut self, index: usize) {
        let Some(article) = self.feeds.get(index).and_then(|panel| panel.selected_news()) else {
            return;
        };
        self.dialog = Some(Dialog::News {
            loading: true,
            scroll: 0,
            content: None,
        });
        spawn_news_content(
            self.client.clone(),
            article.id.clone(),
            self.tx.clone(),
        );
    }

    fn open_report_dialog(&mut self, index: usize) {
        let Some(report) = self
            .feeds
            .get(index)
            .and_then(|panel| panel.selected_report())
            .cloned()
        else {
            return;
        };
        self.dialog = Some(Dialog::Research(report));
    }

    fn scroll_down(&mut self) {
        match self.focus {
            Focus::Watchlist => {
                if self.selected + 1 < self.watchlist.len() {
                    self.selected += 1;
                    self.refresh_symbol();
                }
            }
            Focus::Kline => {
                let max = self.kline_scroll_max();
                if self.kline_scroll < max {
                    self.kline_scroll += 1;
                }
            }
            Focus::Quote => {
                self.quote_scroll = self.quote_scroll.saturating_add(1);
            }
            Focus::Feed(index) => self.scroll_feed_down(index),
        }
    }

    fn scroll_up(&mut self) {
        match self.focus {
            Focus::Watchlist => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.refresh_symbol();
                }
            }
            Focus::Kline => {
                self.kline_scroll = self.kline_scroll.saturating_sub(1);
            }
            Focus::Quote => {
                self.quote_scroll = self.quote_scroll.saturating_sub(1);
            }
            Focus::Feed(index) => self.scroll_feed_up(index),
        }
    }

    fn cycle_kline_period(&mut self) {
        if self.kline_view == KlineView::Timeline {
            if self.timeline_scope == TimelineScope::Day {
                self.timeline_scope = TimelineScope::FiveDays;
                self.timeline_day = 0;
                self.kline_scroll = self.visible_intraday().len().saturating_sub(1);
                self.refresh_kline_poll();
                return;
            }
            self.kline_view = KlineView::Chart;
            self.timeline_scope = TimelineScope::Day;
            self.kline_type = KLineType::Daily;
            self.enter_kline_history_view();
            return;
        }

        if matches!(self.kline_type, KLineType::Min60 | KLineType::Quarterly) {
            self.kline_view = KlineView::Timeline;
            self.timeline_scope = TimelineScope::Day;
            self.timeline_day = 0;
            self.kline_type = KLineType::Daily;
            self.kline_scroll = self.visible_intraday().len().saturating_sub(1);
            self.refresh_kline_poll();
            return;
        }

        self.cycle_kline_type();
    }

    fn cycle_kline_mode(&mut self, forward: bool) {
        self.kline_view = if forward {
            self.kline_view.toggle()
        } else {
            match self.kline_view {
                KlineView::Timeline => KlineView::Table,
                KlineView::Chart => KlineView::Timeline,
                KlineView::Table => KlineView::Chart,
            }
        };
        self.kline_scroll = self.kline_scroll_len().saturating_sub(1);
        match self.kline_view {
            KlineView::Timeline => self.refresh_kline_poll(),
            KlineView::Chart | KlineView::Table => self.enter_kline_history_view(),
        }
    }

    fn enter_kline_history_view(&mut self) {
        self.kline_scroll = self.kline.len().saturating_sub(1);
        self.refresh_kline_poll();
    }

    fn cycle_kline_type(&mut self) {
        self.kline_type = match self.kline_type {
            KLineType::Daily => KLineType::Weekly,
            KLineType::Weekly => KLineType::Monthly,
            KLineType::Monthly => KLineType::Min5,
            KLineType::Min5 => KLineType::Min15,
            KLineType::Min15 => KLineType::Min30,
            KLineType::Min30 => KLineType::Min60,
            KLineType::Min60 | KLineType::Quarterly => KLineType::Daily,
        };
        self.refresh_kline();
    }

    fn add_symbol(&mut self, symbol: StockCode) {
        if symbol.stock_code.is_empty() {
            return;
        }
        if self
            .watchlist
            .iter()
            .any(|entry| entry.stock_code == symbol.stock_code && entry.exchange == symbol.exchange)
        {
            return;
        }
        self.watchlist.push(symbol);
        self.selected = self.watchlist.len() - 1;
        let _ = config::save_watchlist(&self.watchlist);
        self.refresh_watchlist();
        self.refresh_symbol();
    }

    fn add_symbol_input(&mut self, input: &str) {
        let input = input.trim();
        if input.is_empty() {
            return;
        }
        let Some((code, exchange_text)) = input.split_once(':') else {
            self.status = t!("tui.status.symbol_format").to_string();
            return;
        };
        let Ok(exchange) = exchange_text.trim().parse::<Exchange>() else {
            self.status = t!("tui.status.symbol_format").to_string();
            return;
        };
        let code = code.trim();
        if code.is_empty() || exchange == Exchange::Unknown {
            self.status = t!("tui.status.symbol_format").to_string();
            return;
        }
        self.add_symbol(StockCode::new(
            code.to_string(),
            String::new(),
            exchange,
        ));
    }

    fn remove_symbol(&mut self, code: &str) {
        self.watchlist.retain(|entry| entry.stock_code != code);
        if self.watchlist.is_empty() {
            self.watchlist = config::load_watchlist();
        }
        self.selected = self.selected.min(self.watchlist.len().saturating_sub(1));
        let _ = config::save_watchlist(&self.watchlist);
        self.refresh_watchlist();
        self.refresh_symbol();
    }

    fn remove_symbol_input(&mut self, input: &str) {
        self.remove_symbol(input.trim());
    }

    fn remove_selected(&mut self) {
        if let Some(code) = self.current_code().map(str::to_string) {
            self.remove_symbol(&code);
        }
    }

    pub fn on_startup(&mut self) {
        for panel in &mut self.feeds {
            if panel.kind.scope() == FeedScope::Market {
                panel.loading = true;
            }
        }
        self.refresh_watchlist();
        self.refresh_symbol();
        self.refresh_market_feeds();
        self.status = t!("tui.status.loading").to_string();
    }

    pub fn save_on_exit(&self) {
        let _ = config::save_watchlist(&self.watchlist);
    }
}
