//! Integration tests for FileDeck's taxonomy management: tags, correspondents, document types, and storage paths.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TagModel {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub is_inbox_tag: bool,
    pub match_pattern: Option<String>,
    pub matching_algorithm: i64,
    pub is_insensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CorrespondentModel {
    pub id: i64,
    pub name: String,
    pub match_pattern: Option<String>,
    pub matching_algorithm: i64,
    pub is_insensitive: bool,
    pub document_count: i64,
    pub last_correspondence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentTypeModel {
    pub id: i64,
    pub name: String,
    pub match_pattern: Option<String>,
    pub matching_algorithm: i64,
    pub is_insensitive: bool,
    pub document_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoragePathModel {
    pub id: i64,
    pub name: String,
    pub path_format: String,
    pub match_pattern: Option<String>,
    pub matching_algorithm: i64,
    pub is_insensitive: bool,
    pub document_count: i64,
}

#[test]
fn test_tag_color_validation() {
    let is_valid_hex = |color: &str| -> bool {
        let trimmed = color.trim();
        (trimmed.len() == 7 && trimmed.starts_with('#') && trimmed[1..].chars().all(|c| c.is_ascii_hexdigit()))
            || (trimmed.len() == 4 && trimmed.starts_with('#') && trimmed[1..].chars().all(|c| c.is_ascii_hexdigit()))
    };

    assert!(is_valid_hex("#ff0000"));
    assert!(is_valid_hex("#2563EB"));
    assert!(is_valid_hex("#000"));
    assert!(is_valid_hex("#fff"));

    assert!(!is_valid_hex("red"));
    assert!(!is_valid_hex("#ggg"));
    assert!(!is_valid_hex("#12345678"));
}

#[test]
fn test_storage_path_placeholder_formatting() {
    let format_storage_path = |template: &str, year: &str, correspondent: &str, title: &str| -> String {
        template
            .replace("{created_year}", year)
            .replace("{correspondent}", correspondent)
            .replace("{title}", title)
    };

    let tmpl = "{created_year}/{correspondent}/{title}.pdf";
    let formatted = format_storage_path(tmpl, "2024", "Acme_Corp", "Q3_Financial_Summary");
    assert_eq!(formatted, "2024/Acme_Corp/Q3_Financial_Summary.pdf");

    let tmpl_flat = "Archive/{created_year}_{title}.pdf";
    let formatted_flat = format_storage_path(tmpl_flat, "2023", "Any", "Invoice_101");
    assert_eq!(formatted_flat, "Archive/2023_Invoice_101.pdf");
}

#[test]
fn test_taxonomy_serialization_roundtrip() {
    let tag = TagModel {
        id: 1,
        name: "Tax Deductible".to_string(),
        color: "#10B981".to_string(),
        is_inbox_tag: false,
        match_pattern: Some("tax deductible".to_string()),
        matching_algorithm: 1,
        is_insensitive: true,
    };

    let json_str = serde_json::to_string(&tag).unwrap();
    let parsed: TagModel = serde_json::from_str(&json_str).unwrap();
    assert_eq!(tag, parsed);

    let corr = CorrespondentModel {
        id: 5,
        name: "Stadtwerke München".to_string(),
        match_pattern: Some("Stadtwerke München".to_string()),
        matching_algorithm: 3,
        is_insensitive: true,
        document_count: 24,
        last_correspondence: Some("2024-09-18".to_string()),
    };

    let corr_json = serde_json::to_string(&corr).unwrap();
    let corr_parsed: CorrespondentModel = serde_json::from_str(&corr_json).unwrap();
    assert_eq!(corr, corr_parsed);
}
