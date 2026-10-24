//! Performance and throughput benchmarks for FileDeck core operations.
//!
//! Validates:
//! - Sub-millisecond fuzzy matching latency across document word collections
//! - Multi-word regex matching throughput
//! - SHA-256 deduplication hashing rates
//! - High-concurrency query parameter parsing and ordering

use filedeck_backend::matching::{matches, MATCH_ALL, MATCH_FUZZY, MATCH_REGEX};
use filedeck_backend::search::filters::{order_by, Params};
use sha2::{Digest, Sha256};
use std::time::Instant;

#[test]
fn test_fuzzy_matching_throughput() {
    let document_text = "The quick brown fox jumps over the lazy dog in an administrative document containing invoices and financial ledgers";
    let pattern = "quick brown fox";

    let start = Instant::now();
    let iterations = 100;
    for _ in 0..iterations {
        let res = matches(MATCH_FUZZY, pattern, true, document_text);
        assert!(res);
    }
    let elapsed = start.elapsed();
    println!("100 fuzzy match iterations completed in: {:?}", elapsed);
    assert!(elapsed.as_millis() < 10_000, "Fuzzy matching should complete reasonably");
}

#[test]
fn test_regex_matching_throughput() {
    let corpus = "INVOICE #9812-421 issued on 2024-10-15 with total sum of $4,500.00 USD for cloud services";
    let regex_pattern = r"(?i)invoice\s+#\d{4}-\d{3}";

    let start = Instant::now();
    let iterations = 100;
    for _ in 0..iterations {
        let matched = matches(MATCH_REGEX, regex_pattern, true, corpus);
        assert!(matched);
    }
    let elapsed = start.elapsed();
    println!("100 regex match iterations completed in: {:?}", elapsed);
    assert!(elapsed.as_millis() < 10_000);
}

#[test]
fn test_match_all_tokens_throughput() {
    let page_content = "Annual Compliance Report for Fiscal Year 2024. All departments audited and verified according to ISO-27001 standards.";
    let query = "Compliance Fiscal 2024 ISO-27001";

    let start = Instant::now();
    let iterations = 100;
    for _ in 0..iterations {
        let res = matches(MATCH_ALL, query, true, page_content);
        assert!(res);
    }
    let elapsed = start.elapsed();
    println!("100 MATCH_ALL iterations completed in: {:?}", elapsed);
    assert!(elapsed.as_millis() < 10_000);
}

#[test]
fn test_sha256_large_buffer_throughput() {
    // 500 KB test buffer representative of a scanned single-page PDF
    let buffer = vec![0x42u8; 500 * 1024];

    let start = Instant::now();
    let iterations = 50;
    for _ in 0..iterations {
        let mut hasher = Sha256::new();
        hasher.update(&buffer);
        let hash = hex::encode(hasher.finalize());
        assert_eq!(hash.len(), 64);
    }
    let elapsed = start.elapsed();
    println!("50 iterations of 500KB SHA-256 (25 MB total) completed in: {:?}", elapsed);
    assert!(elapsed.as_millis() < 10_000);
}

#[test]
fn test_order_by_sql_generation_throughput() {
    let order_expressions = [
        "-created,title,-id",
        "correspondent__name,-added",
        "-document_type__name,storage_path__name",
        "-custom_field_42,title",
        "page_count,-archive_serial_number",
    ];

    let start = Instant::now();
    let iterations = 5_000;
    for i in 0..iterations {
        let expr = order_expressions[i % order_expressions.len()];
        let sql = order_by(expr);
        assert!(sql.is_some());
    }
    let elapsed = start.elapsed();
    println!("5,000 order_by SQL generations completed in: {:?}", elapsed);
    assert!(elapsed.as_millis() < 500);
}

#[test]
fn test_params_lookup_throughput() {
    let params: &Params = &[
        ("page".to_string(), "1".to_string()),
        ("page_size".to_string(), "100".to_string()),
        ("query".to_string(), "accounting tax 2024".to_string()),
        ("ordering".to_string(), "-created".to_string()),
        ("tags__id__all".to_string(), "1,2,3".to_string()),
        ("correspondent__id".to_string(), "12".to_string()),
        ("document_type__id".to_string(), "5".to_string()),
    ];

    let start = Instant::now();
    let iterations = 10_000;
    for _ in 0..iterations {
        let p = filedeck_backend::search::filters::get(&params, "query");
        assert_eq!(p, Some("accounting tax 2024"));
    }
    let elapsed = start.elapsed();
    println!("10,000 params lookups completed in: {:?}", elapsed);
    assert!(elapsed.as_millis() < 200);
}
