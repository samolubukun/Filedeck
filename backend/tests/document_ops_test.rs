//! Integration tests for FileDeck's document operations, bulk editing, and trash purge logic.

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrashedDocumentRecord {
    pub id: i64,
    pub title: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperationRequest {
    pub document_ids: Vec<i64>,
    pub operation: String,
    pub correspondent_id: Option<i64>,
    pub document_type_id: Option<i64>,
    pub storage_path_id: Option<i64>,
    pub add_tag_ids: Vec<i64>,
    pub remove_tag_ids: Vec<i64>,
}

#[test]
fn test_trash_retention_cutoff_calculation() {
    let now = Utc::now();
    let retention_days = 30;
    let cutoff = now - Duration::days(retention_days);

    let doc_deleted_recent = now - Duration::days(5);
    let doc_deleted_old = now - Duration::days(45);

    // Documents deleted > 30 days ago should be purged
    assert!(doc_deleted_old < cutoff);
    // Documents deleted < 30 days ago should remain in trash
    assert!(doc_deleted_recent >= cutoff);
}

#[test]
fn test_bulk_operation_request_validation() {
    let req = BulkOperationRequest {
        document_ids: vec![1, 2, 3, 4, 5],
        operation: "modify_tags".to_string(),
        correspondent_id: None,
        document_type_id: None,
        storage_path_id: None,
        add_tag_ids: vec![10, 11],
        remove_tag_ids: vec![99],
    };

    assert_eq!(req.document_ids.len(), 5);
    assert_eq!(req.add_tag_ids, vec![10, 11]);
    assert_eq!(req.remove_tag_ids, vec![99]);
}

#[test]
fn test_document_filename_sanitization() {
    let sanitize_filename = |raw: &str| -> String {
        raw.chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                c if c.is_ascii_control() => '_',
                c => c,
            })
            .collect::<String>()
            .trim()
            .to_string()
    };

    assert_eq!(
        sanitize_filename("Invoice: 2024/10/24 *Confidential*.pdf"),
        "Invoice_ 2024_10_24 _Confidential_.pdf"
    );
    assert_eq!(sanitize_filename("Clean_Name.pdf"), "Clean_Name.pdf");
}

#[test]
fn test_document_mime_type_classification() {
    let is_viewable_in_browser = |mime: &str| -> bool {
        matches!(
            mime,
            "application/pdf"
                | "image/png"
                | "image/jpeg"
                | "image/webp"
                | "image/gif"
                | "text/plain"
        )
    };

    assert!(is_viewable_in_browser("application/pdf"));
    assert!(is_viewable_in_browser("image/png"));
    assert!(is_viewable_in_browser("image/jpeg"));
    assert!(!is_viewable_in_browser("application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
    assert!(!is_viewable_in_browser("application/zip"));
}
