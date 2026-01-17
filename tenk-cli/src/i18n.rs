//! Internationalization (i18n) support for tenk-cli.

use rust_i18n::t;
use unicode_width::UnicodeWidthStr;

/// Supported languages
pub const SUPPORTED_LANGUAGES: &[&str] = &["en", "zh-CN"];

/// Pad a string to a given display width 
pub fn pad_display_width(s: &str, width: usize) -> String {
    let display_width = UnicodeWidthStr::width(s);
    if display_width >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - display_width))
    }
}

/// Initialize i18n with the specified language.
pub fn init(cli_lang: &str) {
    let lang = if cli_lang != "en" {
        cli_lang.to_string()
    } else {
        std::env::var("TENK_LANG").unwrap_or_else(|_| "en".to_string())
    };

    let lang = if SUPPORTED_LANGUAGES.contains(&lang.as_str()) {
        lang
    } else {
        "en".to_string()
    };

    rust_i18n::set_locale(&lang);
}

/// Get current locale
#[allow(dead_code)]
pub fn current_locale() -> String {
    rust_i18n::locale().to_string()
}

/// Format volume with localized unit suffix.
pub fn format_volume_i18n(vol: u64) -> String {
    if vol >= 100_000_000 {
        format!(
            "{} ({:.2}{})",
            vol,
            vol as f64 / 100_000_000.0,
            t!("units.yi")
        )
    } else if vol >= 10_000 {
        format!("{} ({:.2}{})", vol, vol as f64 / 10_000.0, t!("units.wan"))
    } else {
        format!("{}", vol)
    }
}

/// Format amount with localized unit suffix.
pub fn format_amount_i18n(amount: f64) -> String {
    if amount >= 100_000_000.0 {
        format!(
            "{:.0} ({:.2}{})",
            amount,
            amount / 100_000_000.0,
            t!("units.yi")
        )
    } else if amount >= 10_000.0 {
        format!(
            "{:.0} ({:.2}{})",
            amount,
            amount / 10_000.0,
            t!("units.wan")
        )
    } else {
        format!("{:.2}", amount)
    }
}
