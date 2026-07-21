pub fn format_change_pct(pct: f64) -> String {
    if pct >= 0.0 {
        format!("+{:.2}%", pct)
    } else {
        format!("{:.2}%", pct)
    }
}

pub fn format_price(value: f64) -> String {
    format!("{:.2}", value)
}
