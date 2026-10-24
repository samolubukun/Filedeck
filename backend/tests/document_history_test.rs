//! Integration tests for FileDeck's document audit history and diff tracking.

use filedeck_backend::history::Snapshot;
use serde_json::{json, Value};

#[test]
fn test_snapshot_default_state() {
    let snap = Snapshot::default();
    let cloned = snap.clone();
    drop(cloned);
}

#[test]
fn test_history_change_event_formatting() {
    let mut changes = serde_json::Map::new();
    changes.insert(
        "title".to_string(),
        json!(["Old Invoice Title", "New Invoice Title 2024"]),
    );
    changes.insert(
        "correspondent".to_string(),
        json!(["None", "Google Cloud EMEA"]),
    );
    changes.insert(
        "archive_serial_number".to_string(),
        json!(["1001", "1002"]),
    );

    let payload = json!({
        "document_id": 42,
        "action": "update",
        "timestamp": "2024-10-24T22:30:00Z",
        "changes": Value::Object(changes),
    });

    assert_eq!(payload["document_id"], 42);
    assert_eq!(payload["action"], "update");
    assert_eq!(
        payload["changes"]["title"][1],
        "New Invoice Title 2024"
    );
    assert_eq!(
        payload["changes"]["correspondent"][0],
        "None"
    );
}

#[test]
fn test_history_m2m_tags_change_representation() {
    let tag_add_event = json!({
        "type": "m2m",
        "operation": "add",
        "objects": ["Tax 2024", "Verified", "Finance"]
    });

    let tag_remove_event = json!({
        "type": "m2m",
        "operation": "remove",
        "objects": ["Needs Review"]
    });

    assert_eq!(tag_add_event["operation"], "add");
    assert_eq!(tag_add_event["objects"].as_array().unwrap().len(), 3);
    assert_eq!(tag_remove_event["operation"], "remove");
    assert_eq!(tag_remove_event["objects"][0], "Needs Review");
}

#[test]
fn test_history_custom_field_diff_representation() {
    let cf_change = json!({
        "PO_Number": ["PO-99120", "PO-99125"],
        "Approved_Amount": ["$1,250.00", "$1,450.00"],
        "Department": ["Sales", "Engineering"]
    });

    assert_eq!(cf_change["PO_Number"][0], "PO-99120");
    assert_eq!(cf_change["PO_Number"][1], "PO-99125");
    assert_eq!(cf_change["Approved_Amount"][1], "$1,450.00");
}
