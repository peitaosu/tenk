use ratatui::style::Style;
use ratatui::text::{Line, Span};
use rust_i18n::t;
use tenk::{CurrentMarketData, StockValuation};

use super::style::{format_change_pct, format_price};
use super::theme::UiTheme;
use super::text::{display_width, grid_columns, grid_pair_width, pad_display_end, truncate_display_end};
use crate::i18n::{format_amount_compact, format_volume_compact};

pub struct QuoteRow {
    pub key: String,
    pub value: String,
    pub value_style: Option<Style>,
}

pub fn quote_rows(
    quote: &CurrentMarketData,
    valuation: Option<&StockValuation>,
    theme: UiTheme,
) -> Vec<QuoteRow> {
    let mut rows = vec![
        row(&t!("headers.code"), quote.stock_code.clone(), None, theme),
        row(&t!("headers.name"), quote.short_name.clone(), None, theme),
        row(
            &t!("headers.price"),
            format_price(quote.price),
            Some(theme.change_style(quote.change_pct)),
            theme,
        ),
        row(
            &t!("headers.change"),
            format_signed(quote.change),
            Some(theme.change_style(quote.change_pct)),
            theme,
        ),
        row(
            &t!("headers.change_pct"),
            format_change_pct(quote.change_pct),
            Some(theme.change_style(quote.change_pct)),
            theme,
        ),
        opt_row(&t!("headers.open"), quote.open, theme),
        opt_row(&t!("headers.high"), quote.high, theme),
        opt_row(&t!("headers.low"), quote.low, theme),
        opt_row(&t!("tui.quote.pre_close"), quote.pre_close, theme),
        row(
            &t!("headers.volume"),
            format_volume_compact(quote.volume),
            None,
            theme,
        ),
        row(
            &t!("headers.amount"),
            format_amount_compact(quote.amount),
            None,
            theme,
        ),
    ];

    let Some(v) = valuation else {
        return rows;
    };

    rows.extend([
        row(
            &t!("labels.pe_ttm"),
            opt_f64(v.pe_ttm),
            None,
            theme,
        ),
        row(&t!("labels.pe_static"), opt_f64(v.pe_static), None, theme),
        row(&t!("labels.pb"), opt_f64(v.pb), None, theme),
        row(&t!("labels.ps"), opt_f64(v.ps), None, theme),
        row(
            &t!("labels.market_cap"),
            format_amount_compact(v.market_cap),
            None,
            theme,
        ),
        row(
            &t!("labels.float_cap"),
            format_amount_compact(v.float_cap),
            None,
            theme,
        ),
        row(&t!("labels.eps"), opt_f64(v.eps), None, theme),
        row(&t!("labels.bps"), opt_f64(v.bps), None, theme),
        row(&t!("labels.roe"), opt_pct(v.roe), None, theme),
        row(
            &t!("labels.gross_margin"),
            opt_pct(v.gross_margin),
            None,
            theme,
        ),
        row(
            &t!("labels.net_margin"),
            opt_pct(v.net_margin),
            None,
            theme,
        ),
        row(
            &t!("labels.revenue"),
            opt_amount(v.revenue),
            None,
            theme,
        ),
        row(
            &t!("labels.net_profit"),
            opt_amount(v.net_profit),
            None,
            theme,
        ),
        row(
            &t!("labels.revenue_yoy"),
            opt_pct(v.revenue_yoy),
            None,
            theme,
        ),
        row(
            &t!("labels.profit_yoy"),
            opt_pct(v.profit_yoy),
            None,
            theme,
        ),
    ]);

    rows
}

fn opt_amount(value: Option<f64>) -> String {
    value
        .map(format_amount_compact)
        .unwrap_or_else(|| "-".into())
}

fn strip_label_colon(key: &str) -> String {
    key.trim_end_matches(':').trim_end().to_string()
}

pub fn quote_grid_lines(rows: &[QuoteRow], width: usize, theme: UiTheme) -> Vec<Line<'static>> {
    if width == 0 || rows.is_empty() {
        return Vec::new();
    }
    let gap = 2;
    let min_pair_width = 16;
    let columns = grid_columns(width, gap, min_pair_width, 4);
    let pair_width = grid_pair_width(width, columns, gap);
    let label_w = (pair_width / 2).clamp(6, 12);
    let value_w = pair_width.saturating_sub(label_w);

    rows.chunks(columns)
        .map(|chunk| {
            let mut spans = Vec::new();
            for (index, item) in chunk.iter().enumerate() {
                if index > 0 {
                    spans.push(Span::raw(" ".repeat(gap)));
                }
                let label = pad_display_end(&strip_label_colon(&item.key), label_w);
                let value = truncate_display_end(&item.value, value_w);
                spans.push(Span::styled(label, theme.label()));
                spans.push(Span::styled(
                    value.clone(),
                    item.value_style.unwrap_or_else(|| theme.text()),
                ));
                let cell_w = label_w + display_width(&value);
                let pad = pair_width.saturating_sub(cell_w);
                if pad > 0 {
                    spans.push(Span::raw(" ".repeat(pad)));
                }
            }
            Line::from(spans)
        })
        .collect()
}

fn row(key: &str, value: String, value_style: Option<Style>, _theme: UiTheme) -> QuoteRow {
    QuoteRow {
        key: key.to_string(),
        value,
        value_style,
    }
}

fn opt_row(key: &str, value: Option<f64>, theme: UiTheme) -> QuoteRow {
    row(
        key,
        value.map(format_price).unwrap_or_else(|| "-".into()),
        None,
        theme,
    )
}

fn format_signed(value: f64) -> String {
    if value >= 0.0 {
        format!("+{:.2}", value)
    } else {
        format!("{:.2}", value)
    }
}

fn opt_f64(value: Option<f64>) -> String {
    value
        .map(|n| format!("{:.2}", n))
        .unwrap_or_else(|| "-".into())
}

fn opt_pct(value: Option<f64>) -> String {
    value
        .map(|n| format!("{:.2}%", n))
        .unwrap_or_else(|| "-".into())
}
