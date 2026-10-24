//! Integration tests for FileDeck's workflow automation engine.
//!
//! Workflows evaluate document events (consumption, creation, modification),
//! match trigger conditions (sources, paths, filename globs, tag filters),
//! and dispatch configured actions (taxonomy assignments, tag removals, webhooks).

use filedeck_backend::workflows::{
    TRIGGER_CONSUMPTION, TRIGGER_DOCUMENT_ADDED, TRIGGER_DOCUMENT_UPDATED,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowTriggerSpec {
    pub id: i64,
    pub workflow_id: i64,
    pub trigger_type: i64,
    pub sources: Option<String>,
    pub filter_filename: Option<String>,
    pub filter_has_tags: Option<String>,
    pub filter_has_all_tags: Option<String>,
    pub filter_has_not_tags: Option<String>,
    pub filter_has_correspondent_id: Option<i64>,
    pub filter_has_document_type_id: Option<i64>,
    pub filter_has_storage_path_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowActionSpec {
    pub id: i64,
    pub workflow_id: i64,
    pub action_type: i64,
    pub assign_title: Option<String>,
    pub assign_correspondent_id: Option<i64>,
    pub assign_document_type_id: Option<i64>,
    pub assign_storage_path_id: Option<i64>,
    pub assign_tags: Option<String>,
    pub remove_tags: Option<String>,
    pub remove_all_tags: bool,
    pub remove_all_correspondents: bool,
    pub remove_all_document_types: bool,
    pub remove_all_storage_paths: bool,
    pub webhook_url: Option<String>,
}

#[test]
fn test_workflow_trigger_types_constants() {
    assert_eq!(TRIGGER_CONSUMPTION, 1);
    assert_eq!(TRIGGER_DOCUMENT_ADDED, 2);
    assert_eq!(TRIGGER_DOCUMENT_UPDATED, 3);
}

#[test]
fn test_workflow_trigger_serialization() {
    let spec = WorkflowTriggerSpec {
        id: 101,
        workflow_id: 1,
        trigger_type: TRIGGER_DOCUMENT_ADDED,
        sources: Some("api_upload,consume_folder".to_string()),
        filter_filename: Some("*.pdf".to_string()),
        filter_has_tags: Some("[1, 2, 3]".to_string()),
        filter_has_all_tags: None,
        filter_has_not_tags: Some("[99]".to_string()),
        filter_has_correspondent_id: Some(5),
        filter_has_document_type_id: Some(2),
        filter_has_storage_path_id: Some(1),
    };

    let serialized = serde_json::to_string(&spec).expect("serialization succeeds");
    let deserialized: WorkflowTriggerSpec =
        serde_json::from_str(&serialized).expect("deserialization succeeds");
    assert_eq!(spec, deserialized);
}

#[test]
fn test_workflow_action_serialization() {
    let action = WorkflowActionSpec {
        id: 201,
        workflow_id: 1,
        action_type: 1, // Assignment
        assign_title: Some("Tax Document {{ date }}".to_string()),
        assign_correspondent_id: Some(42),
        assign_document_type_id: Some(10),
        assign_storage_path_id: Some(3),
        assign_tags: Some("[7, 8]".to_string()),
        remove_tags: None,
        remove_all_tags: false,
        remove_all_correspondents: false,
        remove_all_document_types: false,
        remove_all_storage_paths: false,
        webhook_url: None,
    };

    let json_val = serde_json::to_value(&action).unwrap();
    assert_eq!(json_val["action_type"], 1);
    assert_eq!(json_val["assign_title"], "Tax Document {{ date }}");
    assert_eq!(json_val["assign_correspondent_id"], 42);
}

#[test]
fn test_workflow_webhook_action_payload() {
    let webhook_action = WorkflowActionSpec {
        id: 202,
        workflow_id: 2,
        action_type: 4, // Webhook
        assign_title: None,
        assign_correspondent_id: None,
        assign_document_type_id: None,
        assign_storage_path_id: None,
        assign_tags: None,
        remove_tags: None,
        remove_all_tags: false,
        remove_all_correspondents: false,
        remove_all_document_types: false,
        remove_all_storage_paths: false,
        webhook_url: Some("https://hooks.slack.com/services/XXX/YYY/ZZZ".to_string()),
    };

    let event_name = if TRIGGER_DOCUMENT_ADDED == 2 {
        "document_added"
    } else {
        "document_updated"
    };

    let payload = serde_json::json!({
        "document_id": 505,
        "workflow_id": webhook_action.workflow_id,
        "event": event_name,
        "timestamp": "2024-10-24T20:00:00Z"
    });

    assert_eq!(payload["document_id"], 505);
    assert_eq!(payload["workflow_id"], 2);
    assert_eq!(payload["event"], "document_added");
}

#[test]
fn test_workflow_filename_glob_matching() {
    let glob_to_regex = |pattern: &str| -> regex::Regex {
        let wildcard = pattern.replace('.', r"\.").replace('*', ".*");
        regex::RegexBuilder::new(&format!("^{wildcard}$"))
            .case_insensitive(true)
            .build()
            .unwrap()
    };

    let re_invoices = glob_to_regex("*invoice*.pdf");
    assert!(re_invoices.is_match("march_invoice_2024.pdf"));
    assert!(re_invoices.is_match("INVOICE_Acme.PDF"));
    assert!(!re_invoices.is_match("contract.pdf"));
    assert!(!re_invoices.is_match("invoice.docx"));

    let re_annual = glob_to_regex("annual_report_*.pdf");
    assert!(re_annual.is_match("annual_report_2023.pdf"));
    assert!(re_annual.is_match("annual_report_final.pdf"));
    assert!(!re_annual.is_match("quarterly_report_2023.pdf"));
}

#[test]
fn test_workflow_source_filtering() {
    let check_source = |allowed: Option<&str>, actual: &str| -> bool {
        let Some(s) = allowed else { return true; };
        let list: Vec<String> = s.split(',').map(|x| x.trim().to_ascii_lowercase()).collect();
        list.is_empty() || list.contains(&actual.to_ascii_lowercase())
    };

    assert!(check_source(None, "api_upload"));
    assert!(check_source(Some("api_upload,consume_folder"), "api_upload"));
    assert!(check_source(Some("api_upload,consume_folder"), "consume_folder"));
    assert!(check_source(Some("API_UPLOAD, CONSUME_FOLDER"), "api_upload"));
    assert!(!check_source(Some("consume_folder"), "api_upload"));
    assert!(!check_source(Some("mail_fetch"), "api_upload"));
}

#[test]
fn test_workflow_tag_logic_filters() {
    let doc_tags = vec![1, 2, 5, 8];

    // Filter: has ANY of tags [2, 9, 10] -> passes because 2 is present
    let has_tags = vec![2, 9, 10];
    assert!(has_tags.iter().any(|t| doc_tags.contains(t)));

    // Filter: has ALL of tags [1, 5] -> passes because both are present
    let has_all_tags = vec![1, 5];
    assert!(has_all_tags.iter().all(|t| doc_tags.contains(t)));

    // Filter: has ALL of tags [1, 3] -> fails because 3 is missing
    let has_all_tags_missing = vec![1, 3];
    assert!(!has_all_tags_missing.iter().all(|t| doc_tags.contains(t)));

    // Filter: has NONE of tags [99, 100] -> passes
    let has_not_tags = vec![99, 100];
    assert!(!has_not_tags.iter().any(|t| doc_tags.contains(t)));

    // Filter: has NONE of tags [5, 99] -> fails because 5 is present
    let has_not_tags_conflict = vec![5, 99];
    assert!(has_not_tags_conflict.iter().any(|t| doc_tags.contains(t)));
}
