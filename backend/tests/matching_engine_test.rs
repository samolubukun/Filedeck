//! Integration tests for FileDeck's taxonomy matching engine.
//!
//! Verifies matching algorithms:
//! - MATCH_NONE (0)
//! - MATCH_ANY (1)
//! - MATCH_ALL (2)
//! - MATCH_LITERAL (3)
//! - MATCH_REGEX (4)
//! - MATCH_FUZZY (5)
//! - MATCH_AUTO (6)

use filedeck_backend::matching::{
    matches, Assignment, Predicted, MATCH_ALL, MATCH_ANY, MATCH_AUTO, MATCH_FUZZY, MATCH_LITERAL,
    MATCH_NONE, MATCH_REGEX,
};

#[test]
fn test_match_none_algorithm() {
    let corpus = "Acme Corp Quarterly Financial Statement 2024-Q3";
    assert!(!matches(MATCH_NONE, "Acme", true, corpus));
    assert!(!matches(MATCH_NONE, "Financial", false, corpus));
    assert!(!matches(MATCH_NONE, "2024", true, corpus));
    assert!(!matches(MATCH_NONE, "", true, corpus));
}

#[test]
fn test_match_any_single_and_multi_word() {
    let invoice = "Invoice #98234 from Amazon Web Services EMEA SARL for Cloud Hosting";
    
    // Single word hit
    assert!(matches(MATCH_ANY, "Amazon", true, invoice));
    assert!(matches(MATCH_ANY, "amazon", true, invoice));
    assert!(!matches(MATCH_ANY, "amazon", false, invoice)); // case sensitive check
    assert!(matches(MATCH_ANY, "Amazon", false, invoice));

    // Multi-word pattern where only one matches
    assert!(matches(MATCH_ANY, "Microsoft Google Amazon Azure", true, invoice));
    assert!(matches(MATCH_ANY, "Cloud Database Analytics", true, invoice));
    assert!(!matches(MATCH_ANY, "Oracle Salesforce Snowflake", true, invoice));

    // Word boundary checks (should not match partial words)
    assert!(!matches(MATCH_ANY, "Amaz", true, invoice));
    assert!(!matches(MATCH_ANY, "Host", true, invoice));
    assert!(matches(MATCH_ANY, "Hosting", true, invoice));
}

#[test]
fn test_match_any_quoted_phrases() {
    let receipt = "Total amount paid to Deutsche Telekom AG for Mobile Broadband Plan";

    // Quoted phrase matching
    assert!(matches(MATCH_ANY, "\"Deutsche Telekom\" Vodafone", true, receipt));
    assert!(matches(MATCH_ANY, "'Mobile Broadband' Cable", true, receipt));
    
    // Quoted phrase where phrase words are in different order should NOT match
    assert!(!matches(MATCH_ANY, "\"Telekom Deutsche\"", true, receipt));
    assert!(!matches(MATCH_ANY, "\"Broadband Mobile\"", true, receipt));
}

#[test]
fn test_match_all_algorithm() {
    let statement = "Confidential Payroll summary for John Doe at Acme Industries March 2024";

    // All tokens present
    assert!(matches(MATCH_ALL, "Payroll Acme March", true, statement));
    assert!(matches(MATCH_ALL, "John Doe Confidential", true, statement));
    
    // One token missing -> must fail
    assert!(!matches(MATCH_ALL, "Payroll Acme April", true, statement));
    assert!(!matches(MATCH_ALL, "John Doe Bonus", true, statement));

    // Case sensitivity in MATCH_ALL
    assert!(matches(MATCH_ALL, "payroll acme", true, statement));
    assert!(!matches(MATCH_ALL, "payroll acme", false, statement));
    assert!(matches(MATCH_ALL, "Payroll Acme", false, statement));
}

#[test]
fn test_match_all_with_quotes() {
    let doc = "Medical report issued by St. Jude Children's Research Hospital regarding pediatric care";

    assert!(matches(MATCH_ALL, "\"St. Jude\" pediatric", true, doc));
    assert!(matches(MATCH_ALL, "\"Research Hospital\" \"pediatric care\"", true, doc));
    assert!(!matches(MATCH_ALL, "\"St. Jude\" neurology", true, doc));
}

#[test]
fn test_match_literal_algorithm() {
    let contract = "Non-Disclosure Agreement entered into by and between Alpha Corp and Beta LLC";

    // Exact continuous substring with word boundary
    assert!(matches(MATCH_LITERAL, "Non-Disclosure Agreement", true, contract));
    assert!(matches(MATCH_LITERAL, "Alpha Corp", true, contract));
    assert!(matches(MATCH_LITERAL, "Beta LLC", true, contract));

    // Reversed or scattered words must fail for MATCH_LITERAL
    assert!(!matches(MATCH_LITERAL, "Agreement Non-Disclosure", true, contract));
    assert!(!matches(MATCH_LITERAL, "Alpha Beta", true, contract));

    // Case sensitivity
    assert!(matches(MATCH_LITERAL, "non-disclosure agreement", true, contract));
    assert!(!matches(MATCH_LITERAL, "non-disclosure agreement", false, contract));
}

#[test]
fn test_match_regex_patterns() {
    let log_doc = "ERROR [2024-10-24 18:32:01] Service payment-gateway failed with code 503 Service Unavailable";

    // Standard regex match
    assert!(matches(MATCH_REGEX, r"code\s+\d{3}", true, log_doc));
    assert!(matches(MATCH_REGEX, r"\[\d{4}-\d{2}-\d{2}", true, log_doc));
    assert!(matches(MATCH_REGEX, r"payment-(?:gateway|processor)", true, log_doc));

    // Negative regex match
    assert!(!matches(MATCH_REGEX, r"code\s+200", true, log_doc));
    assert!(!matches(MATCH_REGEX, r"WARN|DEBUG", false, log_doc));

    // Case sensitivity with regex
    assert!(matches(MATCH_REGEX, r"error", true, log_doc));
    assert!(!matches(MATCH_REGEX, r"error", false, log_doc));
    assert!(matches(MATCH_REGEX, r"ERROR", false, log_doc));

    // Malformed regex should gracefully return false without panicking
    assert!(!matches(MATCH_REGEX, r"[unclosed-bracket", true, log_doc));
    assert!(!matches(MATCH_REGEX, r"(unclosed-group", true, log_doc));
    assert!(!matches(MATCH_REGEX, r"*invalid-start", true, log_doc));
}

#[test]
fn test_match_fuzzy_algorithm() {
    let contract = "Agreement with Siemens Aktiengesellschaft for engineering consultation services";

    // Exact match passes fuzzy
    assert!(matches(MATCH_FUZZY, "Siemens", true, contract));

    // Minor typo on long word (>= 90% Levenshtein ratio required by fuzzy engine)
    assert!(matches(MATCH_FUZZY, "Aktiengesellschaf", true, contract));
    assert!(matches(MATCH_FUZZY, "consultaton", true, contract));
    assert!(matches(MATCH_FUZZY, "engineering", true, contract));
    assert!(matches(MATCH_FUZZY, "engeneering", true, contract));

    // Multi-word fuzzy match (all pattern tokens must have a close match)
    assert!(matches(MATCH_FUZZY, "Siemens consultaton", true, contract));
    assert!(!matches(MATCH_FUZZY, "Siemens aerospace", true, contract));

    // Complete mismatch
    assert!(!matches(MATCH_FUZZY, "Volkswagen", true, contract));
    assert!(!matches(MATCH_FUZZY, "completely unrelated document text", true, contract));
}

#[test]
fn test_match_auto_algorithm() {
    // AUTO relies strictly on classifier predictions, not raw text matching
    let text = "Quarterly Tax Return from Internal Revenue Service";
    assert!(!matches(MATCH_AUTO, "Internal Revenue Service", true, text));
    assert!(!matches(MATCH_AUTO, "Tax", true, text));
}

#[test]
fn test_empty_patterns_and_empty_text() {
    assert!(!matches(MATCH_ANY, "", true, "Some text"));
    assert!(!matches(MATCH_ANY, "   ", true, "Some text"));
    assert!(!matches(MATCH_ALL, "", true, "Some text"));
    assert!(!matches(MATCH_LITERAL, "", true, "Some text"));
    assert!(!matches(MATCH_REGEX, "", true, "Some text"));
    assert!(!matches(MATCH_FUZZY, "", true, "Some text"));

    assert!(!matches(MATCH_ANY, "needle", true, ""));
    assert!(!matches(MATCH_ALL, "needle", true, ""));
    assert!(!matches(MATCH_LITERAL, "needle", true, ""));
    assert!(!matches(MATCH_REGEX, "needle", true, ""));
    assert!(!matches(MATCH_FUZZY, "needle", true, ""));
}

#[test]
fn test_special_characters_and_punctuation() {
    let text = "Contract #A-9019-X (Confidential!) @ $1,500.00 / month; 100% SLA guaranteed.";

    assert!(matches(MATCH_ANY, "1,500.00", true, text));
    assert!(matches(MATCH_ANY, "Confidential", true, text));
    assert!(matches(MATCH_LITERAL, "SLA guaranteed", true, text));
    assert!(matches(MATCH_REGEX, r"\$1,500\.00", true, text));
    assert!(matches(MATCH_ALL, "Confidential SLA month", true, text));
}

#[test]
fn test_assignment_and_predicted_defaults() {
    let assignment = Assignment::default();
    assert_eq!(assignment.correspondent, None);
    assert_eq!(assignment.document_type, None);
    assert_eq!(assignment.storage_path, None);
    assert!(assignment.tags.is_empty());

    let mut predicted = Predicted::default();
    assert!(predicted.correspondents.is_empty());
    assert!(predicted.document_types.is_empty());
    assert!(predicted.tags.is_empty());

    predicted.correspondents.push(42);
    predicted.document_types.push(10);
    predicted.tags.push(1);
    predicted.tags.push(2);

    assert_eq!(predicted.correspondents, vec![42]);
    assert_eq!(predicted.document_types, vec![10]);
    assert_eq!(predicted.tags.len(), 2);
}
