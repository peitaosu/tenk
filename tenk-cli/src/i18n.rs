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

#[cfg(test)]
mod tests {
    rust_i18n::i18n!("../locales", fallback = "en");

    use super::*;

    #[test]
    fn test_pad_display_width_ascii() {
        assert_eq!(pad_display_width("abc", 6), "abc   ");
        assert_eq!(pad_display_width("abcdef", 4), "abcdef");
    }

    #[test]
    fn test_pad_display_width_cjk() {
        let padded = pad_display_width("中文", 6);
        assert!(padded.starts_with("中文"));
        assert!(padded.len() > "中文".len());
    }

    #[test]
    fn test_format_volume_i18n() {
        rust_i18n::set_locale("en");
        assert_eq!(format_volume_i18n(500), "500");
        assert!(format_volume_i18n(50_000).contains("5.00"));
        assert!(format_volume_i18n(200_000_000).contains("2.00"));
    }

    #[test]
    fn test_format_amount_i18n() {
        rust_i18n::set_locale("en");
        assert_eq!(format_amount_i18n(123.45), "123.45");
        assert!(format_amount_i18n(50_000.0).contains("5.00"));
        assert!(format_amount_i18n(200_000_000.0).contains("2.00"));
    }

    #[test]
    fn test_init_locale_selection() {
        init("zh-CN");
        assert_eq!(rust_i18n::locale().to_string(), "zh-CN");

        init("fr");
        assert_eq!(rust_i18n::locale().to_string(), "en");

        unsafe {
            std::env::set_var("TENK_LANG", "en");
        }
        init("en");
        assert_eq!(rust_i18n::locale().to_string(), "en");
    }
}
