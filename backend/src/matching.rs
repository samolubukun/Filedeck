//! Auto-assignment of taxonomy (tags, correspondent, document type, storage path).
//!
//! FileDeck employs an intelligent multi-strategy rule matching engine that automatically
//! categorizes incoming documents during ingestion. Rules can match based on:
//!
//! | Algorithm | Constant | Description | Example |
//! | :--- | :--- | :--- | :--- |
//! | **None** | `MATCH_NONE` (0) | Disabled; never matches automatically | N/A |
//! | **Any Word** | `MATCH_ANY` (1) | Matches if ANY pattern word is found | `"invoice bill receipt"` |
//! | **All Words** | `MATCH_ALL` (2) | Matches if ALL pattern words exist | `"Amazon EMEA AWS"` |
//! | **Exact / Literal** | `MATCH_LITERAL` (3) | Matches exact continuous sequence | `"Deutsche Telekom AG"` |
//! | **Regular Expression** | `MATCH_REGEX` (4) | Evaluates PCRE/Rust regex pattern | `r"Invoice\s+#\d{4}"` |
//! | **Fuzzy Matching** | `MATCH_FUZZY` (5) | Levenshtein distance ratio >= 90% | `"Siemens"` matches `"Siemons"` |
//! | **Automatic** | `MATCH_AUTO` (6) | Driven by ML classifier prediction | Supervised model |
//!
//! ### Word Grouping and Quoted Expressions
//! The engine supports quoted phrases in `MATCH_ANY` and `MATCH_ALL` modes using `"` or `'`.
//! For instance, `"\"St. Jude\" pediatric"` requires the exact phrase "St. Jude" alongside
//! the individual word "pediatric".

use regex::RegexBuilder;
use sqlx::{Row, SqlitePool};

/// Algorithm 0: Do not match automatically.
pub const MATCH_NONE: i64 = 0;
/// Algorithm 1: Match if any whitespace-separated word in pattern appears in text.
pub const MATCH_ANY: i64 = 1;
/// Algorithm 2: Match only if all whitespace-separated words in pattern appear in text.
pub const MATCH_ALL: i64 = 2;
/// Algorithm 3: Match exact contiguous phrase with word boundaries.
pub const MATCH_LITERAL: i64 = 3;
/// Algorithm 4: Evaluate regular expression pattern.
pub const MATCH_REGEX: i64 = 4;
/// Algorithm 5: Levenshtein distance ratio >= 90% with length variance <= 2.
pub const MATCH_FUZZY: i64 = 5;
/// Algorithm 6: Defer to ML classifier prediction vector.
pub const MATCH_AUTO: i64 = 6;

/// Split like python's `shlex`-ish matching: whitespace separated, quotes group words.
fn split_words(pattern: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for c in pattern.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => {
                quote = None;
                if !cur.is_empty() {
                    words.push(std::mem::take(&mut cur));
                }
            }
            (Some(_), c) => cur.push(c),
            (None, '"') | (None, '\'') => quote = Some(c),
            (None, c) if c.is_whitespace() => {
                if !cur.is_empty() {
                    words.push(std::mem::take(&mut cur));
                }
            }
            (None, c) => cur.push(c),
        }
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    words
}

fn word_regex(word: &str, insensitive: bool) -> Option<regex::Regex> {
    // Words are joined by flexible whitespace; \b boundaries.
    let escaped = word
        .split_whitespace()
        .map(regex::escape)
        .collect::<Vec<_>>()
        .join(r"\s+");
    RegexBuilder::new(&format!(r"\b{escaped}\b"))
        .case_insensitive(insensitive)
        .build()
        .ok()
}

fn levenshtein(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

fn similar(a: &str, b: &str) -> f64 {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let max = a.len().max(b.len());
    if max == 0 {
        return 100.0;
    }
    100.0 * (1.0 - levenshtein(&a, &b) as f64 / max as f64)
}

/// Fuzzy match: every pattern word must have a text word within the similarity threshold (90).
fn fuzzy(pattern: &str, text: &str) -> bool {
    let norm = |s: &str| -> Vec<String> {
        s.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(String::from)
            .collect()
    };
    let pw = norm(pattern);
    if pw.is_empty() {
        return false;
    }
    let mut tw = norm(text);
    tw.sort();
    tw.dedup();
    pw.iter().all(|p| {
        tw.iter()
            .any(|t| (t.len() as i64 - p.len() as i64).abs() <= 2 && similar(p, t) >= 90.0)
    })
}

/// Does `pattern` (with `algorithm`) match `text`? `Auto`/`None` never match here.
pub fn matches(algorithm: i64, pattern: &str, insensitive: bool, text: &str) -> bool {
    if pattern.trim().is_empty() {
        return false;
    }
    match algorithm {
        MATCH_ANY => split_words(pattern)
            .iter()
            .filter_map(|w| word_regex(w, insensitive))
            .any(|re| re.is_match(text)),
        MATCH_ALL => {
            let words = split_words(pattern);
            !words.is_empty()
                && words.iter().all(|w| {
                    word_regex(w, insensitive)
                        .map(|re| re.is_match(text))
                        .unwrap_or(false)
                })
        }
        MATCH_LITERAL => word_regex(pattern.trim(), insensitive)
            .map(|re| re.is_match(text))
            .unwrap_or(false),
        MATCH_REGEX => RegexBuilder::new(pattern)
            .case_insensitive(insensitive)
            .build()
            .map(|re| re.is_match(text))
            .unwrap_or(false),
        MATCH_FUZZY => fuzzy(pattern, text),
        _ => false,
    }
}

#[derive(Default, Debug)]
pub struct Assignment {
    pub correspondent: Option<i64>,
    pub document_type: Option<i64>,
    pub storage_path: Option<i64>,
    pub tags: Vec<i64>,
}

struct Candidate {
    id: i64,
    pattern: String,
    algorithm: i64,
    insensitive: bool,
}

async fn candidates(db: &SqlitePool, table: &str) -> Result<Vec<Candidate>, sqlx::Error> {
    let rows = sqlx::query(&format!(
        "SELECT id, match, matching_algorithm, is_insensitive FROM {table}"
    ))
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Candidate {
            id: r.get("id"),
            pattern: r.get("match"),
            algorithm: r.get("matching_algorithm"),
            insensitive: r.get("is_insensitive"),
        })
        .collect())
}

/// Ids whose pattern matches the text, plus those the classifier proposed for AUTO objects.
fn select(cands: &[Candidate], text: &str, predicted: &[i64]) -> Vec<i64> {
    cands
        .iter()
        .filter(|c| match c.algorithm {
            MATCH_NONE => false,
            MATCH_AUTO => predicted.contains(&c.id),
            algo => matches(algo, &c.pattern, c.insensitive, text),
        })
        .map(|c| c.id)
        .collect()
}

/// Compute taxonomy for a document. Single-valued fields are only assigned when exactly one
/// object matches (opts for "none" when the match is ambiguous).
pub async fn assign(
    db: &SqlitePool,
    content: &str,
    predicted: &Predicted,
) -> Result<Assignment, sqlx::Error> {
    let mut out = Assignment::default();
    if content.trim().is_empty() && predicted.is_empty() {
        return Ok(out);
    }

    let c = select(
        &candidates(db, "correspondents").await?,
        content,
        &predicted.correspondents,
    );
    if c.len() == 1 {
        out.correspondent = Some(c[0]);
    }
    let d = select(
        &candidates(db, "document_types").await?,
        content,
        &predicted.document_types,
    );
    if d.len() == 1 {
        out.document_type = Some(d[0]);
    }
    let s = select(&candidates(db, "storage_paths").await?, content, &[]);
    if s.len() == 1 {
        out.storage_path = Some(s[0]);
    }
    out.tags = select(&candidates(db, "tags").await?, content, &predicted.tags);
    Ok(out)
}

/// Classifier predictions (only honoured for objects whose algorithm is AUTO).
#[derive(Default)]
pub struct Predicted {
    pub correspondents: Vec<i64>,
    pub document_types: Vec<i64>,
    pub tags: Vec<i64>,
}

impl Predicted {
    fn is_empty(&self) -> bool {
        self.correspondents.is_empty() && self.document_types.is_empty() && self.tags.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_all_literal() {
        let t = "Invoice from Acme Corporation for widgets";
        assert!(matches(MATCH_ANY, "acme foo", true, t));
        assert!(!matches(MATCH_ANY, "foo bar", true, t));
        assert!(matches(MATCH_ALL, "acme widgets", true, t));
        assert!(!matches(MATCH_ALL, "acme gadgets", true, t));
        assert!(matches(MATCH_LITERAL, "acme corporation", true, t));
        assert!(!matches(MATCH_LITERAL, "corporation acme", true, t));
        assert!(!matches(MATCH_LITERAL, "acm", true, t), "word boundaries");
    }

    #[test]
    fn case_sensitivity() {
        assert!(!matches(MATCH_ANY, "acme", false, "ACME"));
        assert!(matches(MATCH_ANY, "acme", true, "ACME"));
    }

    #[test]
    fn regex_and_fuzzy() {
        assert!(matches(MATCH_REGEX, r"inv\w+ce", true, "An Invoice"));
        assert!(!matches(MATCH_REGEX, "(", true, "bad regex does not panic"));
        assert!(matches(
            MATCH_FUZZY,
            "acme corporation",
            true,
            "from Acme Corporaton Ltd"
        ));
        assert!(!matches(
            MATCH_FUZZY,
            "acme corporation",
            true,
            "something else entirely"
        ));
    }

    #[test]
    fn quoted_words() {
        assert!(matches(
            MATCH_ANY,
            "\"bank statement\" zzz",
            true,
            "my bank statement 2024"
        ));
        assert!(!matches(
            MATCH_ALL,
            "\"bank statement\" zzz",
            true,
            "my bank statement 2024"
        ));
    }

    #[test]
    fn empty_pattern_never_matches() {
        assert!(!matches(MATCH_ANY, "", true, "anything"));
    }
}
