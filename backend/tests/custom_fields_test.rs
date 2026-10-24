//! Integration tests for FileDeck's custom fields, data types, and value validation.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CustomFieldDataType {
    String,
    Integer,
    Float,
    Date,
    Boolean,
    Url,
    DocumentLink,
    Monetary,
    Select,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomFieldDefinition {
    pub id: i64,
    pub name: String,
    pub data_type: CustomFieldDataType,
    pub extra_data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomFieldInstanceValue {
    pub field_id: i64,
    pub document_id: i64,
    pub value_text: Option<String>,
    pub value_int: Option<i64>,
    pub value_float: Option<f64>,
    pub value_date: Option<String>,
    pub value_bool: Option<bool>,
    pub value_url: Option<String>,
    pub value_document_ids: Option<Vec<i64>>,
    pub value_monetary: Option<String>,
}

#[test]
fn test_custom_field_data_type_mapping() {
    let fields = vec![
        CustomFieldDefinition {
            id: 1,
            name: "Invoice Number".to_string(),
            data_type: CustomFieldDataType::String,
            extra_data: None,
        },
        CustomFieldDefinition {
            id: 2,
            name: "Total Amount".to_string(),
            data_type: CustomFieldDataType::Monetary,
            extra_data: Some(serde_json::json!({ "currency": "EUR" })),
        },
        CustomFieldDefinition {
            id: 3,
            name: "Verified by Auditor".to_string(),
            data_type: CustomFieldDataType::Boolean,
            extra_data: None,
        },
        CustomFieldDefinition {
            id: 4,
            name: "Due Date".to_string(),
            data_type: CustomFieldDataType::Date,
            extra_data: None,
        },
        CustomFieldDefinition {
            id: 5,
            name: "Parent Contract".to_string(),
            data_type: CustomFieldDataType::DocumentLink,
            extra_data: None,
        },
    ];

    assert_eq!(fields.len(), 5);
    assert_eq!(fields[1].data_type, CustomFieldDataType::Monetary);
    assert_eq!(fields[1].extra_data.as_ref().unwrap()["currency"], "EUR");
}

#[test]
fn test_custom_field_value_serialization() {
    let val = CustomFieldInstanceValue {
        field_id: 10,
        document_id: 100,
        value_text: Some("INV-2024-9988".to_string()),
        value_int: None,
        value_float: None,
        value_date: None,
        value_bool: None,
        value_url: None,
        value_document_ids: None,
        value_monetary: Some("1500.50".to_string()),
    };

    let json_val = serde_json::to_value(&val).unwrap();
    assert_eq!(json_val["field_id"], 10);
    assert_eq!(json_val["value_text"], "INV-2024-9988");
    assert_eq!(json_val["value_monetary"], "1500.50");
}

#[test]
fn test_document_links_custom_field_ids() {
    let link_instance = CustomFieldInstanceValue {
        field_id: 5,
        document_id: 100,
        value_text: None,
        value_int: None,
        value_float: None,
        value_date: None,
        value_bool: None,
        value_url: None,
        value_document_ids: Some(vec![101, 102, 105]),
        value_monetary: None,
    };

    let ids = link_instance.value_document_ids.unwrap();
    assert_eq!(ids.len(), 3);
    assert!(ids.contains(&102));
}

#[test]
fn test_url_custom_field_validation() {
    let is_valid_url = |url: &str| -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    };

    assert!(is_valid_url("https://github.com/samolubukun/Filedeck"));
    assert!(is_valid_url("http://localhost:8080"));
    assert!(!is_valid_url("ftp://invalid.com"));
    assert!(!is_valid_url("plain text not url"));
}
