use ratatui::crossterm::style::{available_color_count, Colored};
use ratatui::style::palette::tailwind::{AMBER, EMERALD, RED, SKY, SLATE};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders};

#[derive(Clone, Copy)]
pub struct UiTheme {
    text: Style,
    dim: Style,
    label: Style,
    focus: Style,
    border: Style,
    section: Style,
    up: Style,
    down: Style,
    highlight: Style,
    status: Style,
    search_border_editing: Style,
    chart_bull: Color,
    chart_bear: Color,
    chart_wick: Color,
    chart_avg: Color,
}

impl UiTheme {
    pub fn detect() -> Self {
        if supports_true_color() {
            Self::colorful()
        } else {
            Self::basic()
        }
    }

    pub fn text(self) -> Style {
        self.text
    }

    pub fn dim(self) -> Style {
        self.dim
    }

    pub fn label(self) -> Style {
        self.label
    }

    pub fn focus(self) -> Style {
        self.focus
    }

    pub fn border(self) -> Style {
        self.border
    }

    pub fn section(self) -> Style {
        self.section
    }

    pub fn highlight(self) -> Style {
        self.highlight
    }

    pub fn list_selected(self) -> Style {
        self.highlight
    }

    pub fn status(self) -> Style {
        self.status
    }

    pub fn search_border_editing(self) -> Style {
        self.search_border_editing
    }

    pub fn change_style(self, pct: f64) -> Style {
        if pct > 0.0 {
            self.up
        } else if pct < 0.0 {
            self.down
        } else {
            self.text
        }
    }

    pub fn chart_colors(self) -> (Color, Color, Color) {
        (self.chart_bull, self.chart_bear, self.chart_wick)
    }

    pub fn chart_avg(self) -> Color {
        self.chart_avg
    }

    pub fn panel_block(self, title: &str, focused: bool) -> Block<'static> {
        let style = if focused { self.focus() } else { self.border() };
        Block::default()
            .borders(Borders::ALL)
            .border_style(style)
            .title(format!(" {title} "))
            .title_style(style)
    }
}

fn supports_true_color() -> bool {
    !Colored::ansi_color_disabled() && available_color_count() == u16::MAX
}

fn reset_style() -> Style {
    Style::reset().fg(Color::Reset)
}

impl UiTheme {
    fn basic() -> Self {
        Self {
            text: reset_style(),
            dim: reset_style().add_modifier(Modifier::DIM),
            label: reset_style().add_modifier(Modifier::DIM),
            focus: reset_style().add_modifier(Modifier::BOLD),
            border: reset_style(),
            section: reset_style().add_modifier(Modifier::BOLD),
            up: reset_style().fg(Color::Red).add_modifier(Modifier::BOLD),
            down: reset_style().fg(Color::Green).add_modifier(Modifier::BOLD),
            highlight: reset_style().add_modifier(Modifier::REVERSED | Modifier::BOLD),
            status: reset_style().add_modifier(Modifier::REVERSED),
            search_border_editing: reset_style().add_modifier(Modifier::BOLD),
            chart_bull: Color::Red,
            chart_bear: Color::Green,
            chart_wick: Color::Gray,
            chart_avg: Color::Yellow,
        }
    }

    fn colorful() -> Self {
        Self {
            text: Style::new().fg(SLATE.c200),
            dim: Style::new().fg(SLATE.c400),
            label: Style::new().fg(SLATE.c400),
            focus: Style::new().fg(SKY.c400).add_modifier(Modifier::BOLD),
            border: Style::new().fg(SLATE.c500),
            section: Style::new().fg(AMBER.c400).add_modifier(Modifier::BOLD),
            up: Style::new().fg(RED.c400).add_modifier(Modifier::BOLD),
            down: Style::new().fg(EMERALD.c400).add_modifier(Modifier::BOLD),
            highlight: Style::new()
                .fg(SLATE.c950)
                .bg(SKY.c400)
                .add_modifier(Modifier::BOLD),
            status: Style::new().fg(SLATE.c950).bg(SKY.c400),
            search_border_editing: Style::new().fg(AMBER.c400).add_modifier(Modifier::BOLD),
            chart_bull: RED.c400,
            chart_bear: EMERALD.c400,
            chart_wick: SLATE.c500,
            chart_avg: AMBER.c400,
        }
    }
}
