use unicode_width::UnicodeWidthChar;

pub fn display_width(text: &str) -> usize {
    text.chars()
        .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(0))
        .sum()
}

pub fn wrap_display(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        let mut current_w = 0;
        for ch in paragraph.chars() {
            let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
            if current_w + ch_w > width && !current.is_empty() {
                lines.push(current);
                current = String::new();
                current_w = 0;
            }
            current.push(ch);
            current_w += ch_w;
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

pub fn truncate_display_fit(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if display_width(s) <= width {
        return s.to_string();
    }
    let mut out = String::new();
    let mut used = 0;
    for ch in s.chars() {
        let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + ch_w + 1 > width {
            break;
        }
        out.push(ch);
        used += ch_w;
    }
    out.push('…');
    out
}

pub fn truncate_display_end(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if display_width(s) <= width {
        return pad_display_end(s, width);
    }
    let mut out = String::new();
    let mut used = 0;
    for ch in s.chars() {
        let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + ch_w + 1 > width {
            break;
        }
        out.push(ch);
        used += ch_w;
    }
    out.push('…');
    pad_display_end(&out, width)
}

pub fn pad_display_end(s: &str, width: usize) -> String {
    let w = display_width(s);
    if w >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - w))
    }
}

pub fn grid_columns(width: usize, gap: usize, min_pair_width: usize, max_columns: usize) -> usize {
    if width == 0 {
        return 1;
    }
    for columns in (1..=max_columns).rev() {
        let total_gap = gap.saturating_mul(columns.saturating_sub(1));
        let pair_width = width.saturating_sub(total_gap) / columns;
        if pair_width >= min_pair_width {
            return columns;
        }
    }
    1
}

pub fn grid_pair_width(width: usize, columns: usize, gap: usize) -> usize {
    if columns == 0 {
        return width;
    }
    width.saturating_sub(gap.saturating_mul(columns.saturating_sub(1))) / columns
}
