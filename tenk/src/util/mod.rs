//! Shared utilities.

mod board;
mod date;
mod eastmoney;
mod html;
mod jsonp;
mod secid;
mod sina;
mod ths;

pub use board::{
    extract_ths_news_content, extract_ths_news_title, normalize_board_name,
    parse_ths_concept_board_section, parse_ths_industry_board_links,
};
pub use date::{normalize_date_bound, parse_trade_date};
pub use eastmoney::{
    cn_market_date, format_cn_market_time, parse_cn_market_time, parse_order_book_from_fields,
    parse_tick_details,
};
pub use html::{extract_balanced_div, html_to_text};
pub use jsonp::parse_jsonp;
pub use secid::{
    eastmoney_secid_for_board, eastmoney_secid_for_hk, eastmoney_secid_for_index,
    eastmoney_secid_for_us, is_us_symbol, sina_hq_symbol, sina_index_hq_symbol,
};
pub use sina::{
    decode_gb18030, kline_scale, parse_kline_records, parse_minute_records,
    parse_order_book_from_parts, parse_ticks_from_trans_list, SinaKLineRecord, SinaMinuteResponse,
};
pub use ths::{is_board_antibot_page, kline_period_code, parse_board_html};
