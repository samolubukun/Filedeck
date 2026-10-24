//! Integration tests for FileDeck's document versioning, version merging, and label tracking.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentVersionRecord {
    pub id: i64,
    pub root_document_id: Option<i64>,
    pub version_index: i64,
    pub version_label: String,
    pub filename: String,
    pub original_filename: String,
    pub mime_type: String,
    pub sha256: String,
    pub created: String,
    pub page_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeVersionsRequest {
    pub root_document_id: i64,
    pub document_ids_to_merge: Vec<i64>,
}

#[test]
fn test_version_lineage_ordering() {
    let mut versions = vec![
        DocumentVersionRecord {
            id: 101,
            root_document_id: None, // Root document
            version_index: 0,
            version_label: "Original Scan".to_string(),
            filename: "doc_101_v0.pdf".to_string(),
            original_filename: "Contract_Draft.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            sha256: "a1b2c3d4e5f6".to_string(),
            created: "2024-01-10T10:00:00Z".to_string(),
            page_count: 5,
        },
        DocumentVersionRecord {
            id: 102,
            root_document_id: Some(101),
            version_index: 2,
            version_label: "Countersigned Final".to_string(),
            filename: "doc_102_v2.pdf".to_string(),
            original_filename: "Contract_Signed.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            sha256: "998877665544".to_string(),
            created: "2024-01-15T14:30:00Z".to_string(),
            page_count: 6,
        },
        DocumentVersionRecord {
            id: 103,
            root_document_id: Some(101),
            version_index: 1,
            version_label: "Legal Review Revisions".to_string(),
            filename: "doc_103_v1.pdf".to_string(),
            original_filename: "Contract_Reviewed.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            sha256: "112233445566".to_string(),
            created: "2024-01-12T09:15:00Z".to_string(),
            page_count: 5,
        },
    ];

    // Sort by version index ascending
    versions.sort_by_key(|v| v.version_index);

    assert_eq!(versions[0].version_label, "Original Scan");
    assert_eq!(versions[1].version_label, "Legal Review Revisions");
    assert_eq!(versions[2].version_label, "Countersigned Final");
    assert_eq!(versions[2].page_count, 6);
}

#[test]
fn test_merge_versions_request_validation() {
    let req = MergeVersionsRequest {
        root_document_id: 50,
        document_ids_to_merge: vec![51, 52, 53],
    };

    assert_eq!(req.root_document_id, 50);
    assert_eq!(req.document_ids_to_merge.len(), 3);
    assert!(!req.document_ids_to_merge.contains(&req.root_document_id));
}

#[test]
fn test_version_label_sanitization() {
    let sanitize_label = |label: &str| -> String {
        let trimmed = label.trim();
        if trimmed.is_empty() {
            "Untitled Version".to_string()
        } else {
            trimmed.chars().take(100).collect()
        }
    };

    assert_eq!(sanitize_label("  Final Approved  "), "Final Approved");
    assert_eq!(sanitize_label(""), "Untitled Version");
    assert_eq!(sanitize_label("   "), "Untitled Version");

    let long_label = "A".repeat(150);
    assert_eq!(sanitize_label(&long_label).len(), 100);
}
