//! Best-effort "created" date extraction from document text.
//!
//! Recognises ISO (2024-03-05), day-first numeric (05.03.2024, 5/3/2024, 05-03-2024) and
//! textual (5 March 2024, March 5, 2024) dates. The first plausible date wins.

use chrono::{Datelike, NaiveDate};
use regex::Regex;
use std::sync::OnceLock;

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

fn month_number(name: &str) -> Option<u32> {
    let n = name.trim_end_matches('.').to_ascii_lowercase();
    if n == "sept" {
        return Some(9);
    }
    if n.len() < 3 {
        return None;
    }
    MONTHS
        .iter()
        .position(|m| m.starts_with(&n))
        .map(|i| i as u32 + 1)
}

fn plausible(d: NaiveDate) -> bool {
    let today = chrono::Local::now().date_naive();
    d.year() >= 1970 && d <= today
}

struct Patterns {
    iso: Regex,
    numeric: Regex,
    day_month_text: Regex,
    month_day_text: Regex,
}

fn patterns() -> &'static Patterns {
    static P: OnceLock<Patterns> = OnceLock::new();
    P.get_or_init(|| Patterns {
        iso: Regex::new(r"\b(\d{4})-(\d{1,2})-(\d{1,2})\b").unwrap(),
        numeric: Regex::new(r"\b(\d{1,2})[./-](\d{1,2})[./-](\d{4})\b").unwrap(),
        day_month_text: Regex::new(
            r"(?i)\b(\d{1,2})(?:st|nd|rd|th)?\s+([A-Za-z]{3,9})\.?,?\s+(\d{4})\b",
        )
        .unwrap(),
        month_day_text: Regex::new(
            r"(?i)\b([A-Za-z]{3,9})\.?\s+(\d{1,2})(?:st|nd|rd|th)?,?\s+(\d{4})\b",
        )
        .unwrap(),
    })
}

/// Returns the earliest-positioned plausible date as `YYYY-MM-DD`.
pub fn find_date(text: &str) -> Option<String> {
    let p = patterns();
    let mut found: Vec<(usize, NaiveDate)> = Vec::new();

    for c in p.iso.captures_iter(text) {
        if let Some(d) =
            NaiveDate::from_ymd_opt(c[1].parse().ok()?, c[2].parse().ok()?, c[3].parse().ok()?)
        {
            found.push((c.get(0)?.start(), d));
        }
    }
    for c in p.numeric.captures_iter(text) {
        let (day, month, year): (u32, u32, i32) =
            (c[1].parse().ok()?, c[2].parse().ok()?, c[3].parse().ok()?);
        if let Some(d) = NaiveDate::from_ymd_opt(year, month, day) {
            found.push((c.get(0)?.start(), d));
        }
    }
    for c in p.day_month_text.captures_iter(text) {
        if let Some(m) = month_number(&c[2]) {
            if let Some(d) = NaiveDate::from_ymd_opt(c[3].parse().ok()?, m, c[1].parse().ok()?) {
                found.push((c.get(0)?.start(), d));
            }
        }
    }
    for c in p.month_day_text.captures_iter(text) {
        if let Some(m) = month_number(&c[1]) {
            if let Some(d) = NaiveDate::from_ymd_opt(c[3].parse().ok()?, m, c[2].parse().ok()?) {
                found.push((c.get(0)?.start(), d));
            }
        }
    }

    found.retain(|(_, d)| plausible(*d));
    found.sort_by_key(|(pos, _)| *pos);
    found.first().map(|(_, d)| d.format("%Y-%m-%d").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_and_numeric() {
        assert_eq!(
            find_date("Invoice date 2024-03-05 due later").as_deref(),
            Some("2024-03-05")
        );
        assert_eq!(
            find_date("Datum: 05.03.2024").as_deref(),
            Some("2024-03-05")
        );
        assert_eq!(find_date("on 5/3/2023.").as_deref(), Some("2023-03-05"));
    }

    #[test]
    fn textual() {
        assert_eq!(
            find_date("Issued 5 March 2024").as_deref(),
            Some("2024-03-05")
        );
        assert_eq!(
            find_date("Issued March 5, 2024").as_deref(),
            Some("2024-03-05")
        );
        assert_eq!(
            find_date("Issued Sept 3rd, 2022").as_deref(),
            Some("2022-09-03")
        );
    }

    #[test]
    fn rejects_future_and_garbage() {
        assert_eq!(find_date("due 2999-01-01"), None);
        assert_eq!(find_date("no dates here"), None);
        assert_eq!(find_date("version 31.13.2024"), None);
    }

    #[test]
    fn earliest_position_wins() {
        assert_eq!(
            find_date("2023-01-02 and later 2024-05-06").as_deref(),
            Some("2023-01-02")
        );
    }
}
