//! Normalizers and sanitizers for extracted document fields.
//!
//! Provides deterministic cleansing, ISO 8601 date parsing, currency extraction,
//! financial format conversion, and IBAN/phone normalization.

use chrono::NaiveDate;
use regex::Regex;
use std::sync::OnceLock;

static WHITESPACE_RE: OnceLock<Regex> = OnceLock::new();
static DIGITS_ONLY_RE: OnceLock<Regex> = OnceLock::new();

fn whitespace_regex() -> &'static Regex {
    WHITESPACE_RE.get_or_init(|| Regex::new(r"\s+").unwrap())
}

fn digits_only_regex() -> &'static Regex {
    DIGITS_ONLY_RE.get_or_init(|| Regex::new(r"\D").unwrap())
}

/// Normalizes a monetary or currency string into a standardized decimal format `1234.56`.
/// Handles multiple symbols ($, €, £, ¥), thousands separators (commas, spaces, periods),
/// and parenthetical accounting negatives e.g. `(150.00)` -> `-150.00`.
pub fn normalize_monetary(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let is_negative = (trimmed.starts_with('(') && trimmed.ends_with(')'))
        || trimmed.starts_with('-')
        || trimmed.ends_with('-');

    let cleaned = trimmed
        .replace(['(', ')', '$', '€', '£', '¥', '₹'], "")
        .replace("USD", "")
        .replace("EUR", "")
        .replace("GBP", "");

    let trimmed_cleaned = cleaned.trim();

    // European format vs US format detection:
    // "1.234,56" -> "1234.56"
    // "1,234.56" -> "1234.56"
    let has_comma = trimmed_cleaned.contains(',');
    let has_dot = trimmed_cleaned.contains('.');

    let parsed_str;

    if has_comma && has_dot {
        let comma_idx = trimmed_cleaned.rfind(',').unwrap();
        let dot_idx = trimmed_cleaned.rfind('.').unwrap();
        if comma_idx > dot_idx {
            // European style: 1.250,50
            let without_thousands = trimmed_cleaned.replace('.', "");
            parsed_str = without_thousands.replace(',', ".");
        } else {
            // Standard US style: 1,250.50
            let without_thousands = trimmed_cleaned.replace(',', "");
            parsed_str = without_thousands;
        }
    } else if has_comma && !has_dot {
        // Could be decimal comma "1250,50" or thousands "1,250"
        let parts: Vec<&str> = trimmed_cleaned.split(',').collect();
        if parts.len() == 2 && parts[1].len() == 2 {
            parsed_str = trimmed_cleaned.replace(',', ".");
        } else {
            parsed_str = trimmed_cleaned.replace(',', "");
        }
    } else {
        // Standard dot or pure digits
        parsed_str = trimmed_cleaned.replace(' ', "");
    }

    // Filter to valid numeric characters
    let valid_chars: String = parsed_str
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();

    if let Ok(num) = valid_chars.parse::<f64>() {
        let final_num = if is_negative && num > 0.0 { -num } else { num };
        Some(format!("{:.2}", final_num))
    } else {
        None
    }
}

/// Normalizes various date string formats into ISO 8601 `YYYY-MM-DD`.
/// Supports:
/// - `YYYY-MM-DD`, `YYYY/MM/DD`, `YYYY.MM.DD`
/// - `MM/DD/YYYY`, `MM-DD-YYYY`
/// - `DD.MM.YYYY`, `DD-MM-YYYY`, `DD/MM/YYYY`
/// - `DD-MMM-YYYY` (e.g. `15-Oct-2026`)
/// - `MMM DD, YYYY` (e.g. `Oct 15, 2026` or `October 15, 2026`)
pub fn normalize_date(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    let formats = [
        "%Y-%m-%d",
        "%Y/%m/%d",
        "%Y.%m.%d",
        "%m/%d/%Y",
        "%m-%d-%Y",
        "%d.%m.%Y",
        "%d-%m-%Y",
        "%d/%m/%Y",
        "%d %b %Y",
        "%d %B %Y",
        "%d-%b-%Y",
        "%d-%B-%Y",
        "%b %d, %Y",
        "%B %d, %Y",
        "%b %d %Y",
        "%B %d %Y",
        "%Y%m%d",
    ];

    for fmt in &formats {
        if let Ok(d) = NaiveDate::parse_from_str(s, fmt) {
            return Some(d.format("%Y-%m-%d").to_string());
        }
    }

    // Fallback: search for embedded YYYY-MM-DD pattern
    let iso_re = Regex::new(r"(\d{4})[-/.](\d{1,2})[-/.](\d{1,2})").unwrap();
    if let Some(caps) = iso_re.captures(s) {
        if let (Ok(y), Ok(m), Ok(d)) = (
            caps[1].parse::<i32>(),
            caps[2].parse::<u32>(),
            caps[3].parse::<u32>(),
        ) {
            if let Some(date) = NaiveDate::from_ymd_opt(y, m, d) {
                return Some(date.format("%Y-%m-%d").to_string());
            }
        }
    }

    None
}

/// Normalizes a clean string by stripping control characters and collapsing whitespace.
pub fn normalize_string(raw: &str) -> String {
    let collapsed = whitespace_regex().replace_all(raw.trim(), " ");
    collapsed
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect::<String>()
        .trim()
        .to_string()
}

/// Normalizes an integer string.
pub fn normalize_integer(raw: &str) -> Option<i64> {
    let digits: String = raw
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    digits.parse::<i64>().ok()
}

/// Normalizes a boolean value from affirmative/negative text.
pub fn normalize_boolean(raw: &str) -> Option<bool> {
    let lower = raw.trim().to_lowercase();
    match lower.as_str() {
        "true" | "yes" | "y" | "1" | "t" | "approved" | "confirmed" => Some(true),
        "false" | "no" | "n" | "0" | "f" | "rejected" | "denied" => Some(false),
        _ => None,
    }
}

/// Checks and formats an International Bank Account Number (IBAN) using the Mod-97 checksum.
pub fn validate_and_format_iban(raw: &str) -> Option<String> {
    let clean: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect();

    if clean.len() < 15 || clean.len() > 34 {
        return None;
    }

    // Country code must be two letters
    if !clean[..2].chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }

    // Rearrange: move first 4 characters to the end
    let rearranged = format!("{}{}", &clean[4..], &clean[..4]);

    // Convert letters to two-digit numbers (A=10, B=11, ... Z=35)
    let mut numeric_str = String::new();
    for c in rearranged.chars() {
        if c.is_ascii_digit() {
            numeric_str.push(c);
        } else {
            let val = (c as u32) - ('A' as u32) + 10;
            numeric_str.push_str(&val.to_string());
        }
    }

    // Compute mod-97 in pieces to avoid 64-bit integer overflow
    let mut remainder = 0u64;
    for chunk in numeric_str.as_bytes().chunks(7) {
        let s = std::str::from_utf8(chunk).unwrap_or("");
        let piece = format!("{}{}", remainder, s);
        if let Ok(n) = piece.parse::<u64>() {
            remainder = n % 97;
        } else {
            return None;
        }
    }

    if remainder == 1 {
        // Format with space every 4 characters
        let formatted = clean
            .as_bytes()
            .chunks(4)
            .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
            .collect::<Vec<&str>>()
            .join(" ");
        Some(formatted)
    } else {
        None
    }
}

/// Cleans phone numbers into a consistent international or standard format.
pub fn normalize_phone(raw: &str) -> String {
    let has_plus = raw.trim().starts_with('+');
    let digits: String = digits_only_regex().replace_all(raw, "").into_owned();
    if has_plus {
        format!("+{}", digits)
    } else {
        digits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monetary_normalization() {
        assert_eq!(normalize_monetary("$1,234.56"), Some("1234.56".to_string()));
        assert_eq!(normalize_monetary("€ 1.234,56"), Some("1234.56".to_string()));
        assert_eq!(normalize_monetary("(500.00)"), Some("-500.00".to_string()));
        assert_eq!(normalize_monetary("-250.75"), Some("-250.75".to_string()));
        assert_eq!(normalize_monetary("99"), Some("99.00".to_string()));
        assert_eq!(normalize_monetary("  $ 4,500.99 USD "), Some("4500.99".to_string()));
    }

    #[test]
    fn test_date_normalization() {
        assert_eq!(normalize_date("2026-10-25"), Some("2026-10-25".to_string()));
        assert_eq!(normalize_date("10/25/2026"), Some("2026-10-25".to_string()));
        assert_eq!(normalize_date("25.10.2026"), Some("2026-10-25".to_string()));
        assert_eq!(normalize_date("25-Oct-2026"), Some("2026-10-25".to_string()));
        assert_eq!(normalize_date("October 25, 2026"), Some("2026-10-25".to_string()));
    }

    #[test]
    fn test_boolean_normalization() {
        assert_eq!(normalize_boolean("yes"), Some(true));
        assert_eq!(normalize_boolean("Approved"), Some(true));
        assert_eq!(normalize_boolean("0"), Some(false));
        assert_eq!(normalize_boolean("no"), Some(false));
        assert_eq!(normalize_boolean("invalid_text"), None);
    }
}
