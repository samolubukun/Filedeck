//! Comprehensive unit and domain integration tests for FileDeck IDP extraction engine.

use filedeck_backend::extraction::exporter::{
    compute_batch_summary, export_json_lines, export_markdown_table, ExportRow,
};
use filedeck_backend::extraction::normalizer::{
    normalize_boolean, normalize_date, normalize_integer, normalize_monetary, normalize_phone,
    normalize_string, validate_and_format_iban,
};
use filedeck_backend::extraction::validator::{validate_cell_value, validate_record_row};
use filedeck_backend::extraction::{
    ColumnDataType, ColumnDefinition, ExtractedCellData, SchemaDefinition, ValidationRule,
};
use std::collections::HashMap;

#[test]
fn test_column_data_types_and_defaults() {
    assert_eq!(ColumnDataType::from_str_loose("string"), ColumnDataType::String);
    assert_eq!(ColumnDataType::from_str_loose("date"), ColumnDataType::Date);
    assert_eq!(ColumnDataType::from_str_loose("monetary"), ColumnDataType::Monetary);
    assert_eq!(ColumnDataType::from_str_loose("money"), ColumnDataType::Monetary);
    assert_eq!(ColumnDataType::from_str_loose("amount"), ColumnDataType::Monetary);
    assert_eq!(ColumnDataType::from_str_loose("integer"), ColumnDataType::Integer);
    assert_eq!(ColumnDataType::from_str_loose("int"), ColumnDataType::Integer);
    assert_eq!(ColumnDataType::from_str_loose("bool"), ColumnDataType::Boolean);
    assert_eq!(ColumnDataType::from_str_loose("boolean"), ColumnDataType::Boolean);
    assert_eq!(ColumnDataType::from_str_loose("unknown"), ColumnDataType::String);

    assert_eq!(ColumnDataType::String.as_str(), "string");
    assert_eq!(ColumnDataType::Date.as_str(), "date");
    assert_eq!(ColumnDataType::Monetary.as_str(), "monetary");
    assert_eq!(ColumnDataType::Integer.as_str(), "integer");
    assert_eq!(ColumnDataType::Boolean.as_str(), "boolean");
}

#[test]
fn test_schema_definition_builder() {
    let cols = vec![
        ColumnDefinition::new("inv_num", "Invoice Number", ColumnDataType::String)
            .with_required(true)
            .with_regex(r"^INV-\d{4}-\d{3}$")
            .with_hints(vec!["Invoice #".into(), "Bill No".into()]),
        ColumnDefinition::new("inv_date", "Invoice Date", ColumnDataType::Date)
            .with_required(true),
        ColumnDefinition::new("total", "Total Amount", ColumnDataType::Monetary)
            .with_required(true),
        ColumnDefinition::new("notes", "Notes", ColumnDataType::String),
    ];

    let schema = SchemaDefinition::new("Vendor Invoices", cols);
    assert_eq!(schema.name, "Vendor Invoices");
    assert_eq!(schema.columns.len(), 4);

    let req_keys = schema.required_keys();
    assert_eq!(req_keys, vec!["inv_num", "inv_date", "total"]);

    let found_col = schema.get_column("inv_num");
    assert!(found_col.is_some());
    assert_eq!(found_col.unwrap().label, "Invoice Number");
    assert!(found_col.unwrap().required);
    assert!(schema.get_column("non_existent").is_none());
}

#[test]
fn test_monetary_normalization_edge_cases() {
    // US notation
    assert_eq!(normalize_monetary("$1,234.56"), Some("1234.56".to_string()));
    assert_eq!(normalize_monetary("  10,000.00 USD  "), Some("10000.00".to_string()));
    assert_eq!(normalize_monetary("0.99"), Some("0.99".to_string()));
    assert_eq!(normalize_monetary(".50"), Some("0.50".to_string()));

    // European notation
    assert_eq!(normalize_monetary("1.234,56 €"), Some("1234.56".to_string()));
    assert_eq!(normalize_monetary("EUR 5.000,00"), Some("5000.00".to_string()));
    assert_eq!(normalize_monetary("250,50"), Some("250.50".to_string()));

    // Accounting parentheses for negative values
    assert_eq!(normalize_monetary("(1,500.00)"), Some("-1500.00".to_string()));
    assert_eq!(normalize_monetary("($250.75)"), Some("-250.75".to_string()));
    assert_eq!(normalize_monetary("-99.95"), Some("-99.95".to_string()));

    // Empty or non-numeric
    assert_eq!(normalize_monetary(""), None);
    assert_eq!(normalize_monetary("   "), None);
    assert_eq!(normalize_monetary("N/A"), None);
    assert_eq!(normalize_monetary("Pending"), None);
}

#[test]
fn test_date_normalization_formats() {
    // Standard ISO
    assert_eq!(normalize_date("2026-10-25"), Some("2026-10-25".to_string()));
    assert_eq!(normalize_date("2026/10/25"), Some("2026-10-25".to_string()));
    assert_eq!(normalize_date("2026.10.25"), Some("2026-10-25".to_string()));

    // US standard MM/DD/YYYY
    assert_eq!(normalize_date("10/25/2026"), Some("2026-10-25".to_string()));
    assert_eq!(normalize_date("10-25-2026"), Some("2026-10-25".to_string()));

    // European standard DD.MM.YYYY
    assert_eq!(normalize_date("25.10.2026"), Some("2026-10-25".to_string()));
    assert_eq!(normalize_date("25-10-2026"), Some("2026-10-25".to_string()));
    assert_eq!(normalize_date("25/10/2026"), Some("2026-10-25".to_string()));

    // Month name variations
    assert_eq!(normalize_date("25 Oct 2026"), Some("2026-10-25".to_string()));
    assert_eq!(normalize_date("25 October 2026"), Some("2026-10-25".to_string()));
    assert_eq!(normalize_date("Oct 25, 2026"), Some("2026-10-25".to_string()));
    assert_eq!(normalize_date("October 25, 2026"), Some("2026-10-25".to_string()));
    assert_eq!(normalize_date("20261025"), Some("2026-10-25".to_string()));

    // Embedded in text
    assert_eq!(normalize_date("Issued on 2026-05-18 by vendor"), Some("2026-05-18".to_string()));

    // Invalid dates
    assert_eq!(normalize_date(""), None);
    assert_eq!(normalize_date("Not a date"), None);
}

#[test]
fn test_string_integer_and_phone_normalizers() {
    assert_eq!(normalize_string("  Hello   World \t \n "), "Hello World");
    assert_eq!(normalize_integer("  12,345 "), Some(12345));
    assert_eq!(normalize_integer("-42"), Some(-42));
    assert_eq!(normalize_integer("abc"), None);

    assert_eq!(normalize_boolean("yes"), Some(true));
    assert_eq!(normalize_boolean("TRUE"), Some(true));
    assert_eq!(normalize_boolean("0"), Some(false));
    assert_eq!(normalize_boolean("rejected"), Some(false));

    assert_eq!(normalize_phone("+1 (555) 123-4567"), "+15551234567");
    assert_eq!(normalize_phone("(555) 987-6543"), "5559876543");
}

#[test]
fn test_iban_mod97_checksum() {
    // Valid standard test IBAN (Germany sample)
    let valid_iban = "DE89370400440532013000";
    let formatted = validate_and_format_iban(valid_iban);
    assert!(formatted.is_some());
    assert_eq!(formatted.unwrap(), "DE89 3704 0044 0532 0130 00");

    // Invalid checksum IBAN
    let invalid_iban = "DE89370400440532013009";
    assert!(validate_and_format_iban(invalid_iban).is_none());

    // Too short
    assert!(validate_and_format_iban("DE123").is_none());
}

#[test]
fn test_cell_validation_logic() {
    let col_str = ColumnDefinition::new("vendor", "Vendor Name", ColumnDataType::String)
        .with_required(true)
        .with_regex(r"^[A-Za-z\s.,]+$");

    let err_empty = validate_cell_value(&col_str, None, None, 0.0);
    assert!(err_empty.is_some());
    assert!(err_empty.unwrap().contains("Required"));

    let err_regex = validate_cell_value(&col_str, Some("12345!"), Some("12345!"), 0.9);
    assert!(err_regex.is_some());
    assert!(err_regex.unwrap().contains("required format pattern"));

    let ok = validate_cell_value(&col_str, Some("Acme Corp."), Some("Acme Corp."), 0.95);
    assert!(ok.is_none());

    let col_money = ColumnDefinition::new("amount", "Amount", ColumnDataType::Monetary);
    assert!(validate_cell_value(&col_money, Some("invalid"), Some("invalid"), 0.9).is_some());
    assert!(validate_cell_value(&col_money, Some("$450.00"), Some("450.00"), 0.9).is_none());

    let col_date = ColumnDefinition::new("due_date", "Due Date", ColumnDataType::Date);
    assert!(validate_cell_value(&col_date, Some("not-a-date"), Some("not-a-date"), 0.9).is_some());
    assert!(validate_cell_value(&col_date, Some("2026-11-30"), Some("2026-11-30"), 0.9).is_none());
}

#[test]
fn test_cross_field_validation_rules() {
    let cols = vec![
        ColumnDefinition::new("invoice_date", "Invoice Date", ColumnDataType::Date),
        ColumnDefinition::new("due_date", "Due Date", ColumnDataType::Date),
        ColumnDefinition::new("subtotal", "Subtotal", ColumnDataType::Monetary),
        ColumnDefinition::new("tax", "Tax", ColumnDataType::Monetary),
        ColumnDefinition::new("total", "Total", ColumnDataType::Monetary),
    ];

    let rules = vec![
        ValidationRule {
            rule: "sum".to_string(),
            target: "total".to_string(),
            components: vec!["subtotal".to_string(), "tax".to_string()],
            tolerance: Some(0.01),
            description: Some("Subtotal + Tax must equal Total".to_string()),
        },
        ValidationRule {
            rule: "date_order".to_string(),
            target: "due_date".to_string(),
            components: vec!["invoice_date".to_string()],
            tolerance: None,
            description: Some("Invoice Date must be before or on Due Date".to_string()),
        },
    ];

    // Case 1: All valid
    let cells_valid = vec![
        ExtractedCellData {
            id: Some(1),
            column_key: "invoice_date".to_string(),
            raw_value: Some("2026-10-01".to_string()),
            normalized_value: Some("2026-10-01".to_string()),
            confidence: 0.95,
            is_reviewed: true,
            validation_error: None,
            bbox: None,
            page_number: 1,
        },
        ExtractedCellData {
            id: Some(2),
            column_key: "due_date".to_string(),
            raw_value: Some("2026-10-31".to_string()),
            normalized_value: Some("2026-10-31".to_string()),
            confidence: 0.95,
            is_reviewed: true,
            validation_error: None,
            bbox: None,
            page_number: 1,
        },
        ExtractedCellData {
            id: Some(3),
            column_key: "subtotal".to_string(),
            raw_value: Some("100.00".to_string()),
            normalized_value: Some("100.00".to_string()),
            confidence: 0.95,
            is_reviewed: true,
            validation_error: None,
            bbox: None,
            page_number: 1,
        },
        ExtractedCellData {
            id: Some(4),
            column_key: "tax".to_string(),
            raw_value: Some("15.00".to_string()),
            normalized_value: Some("15.00".to_string()),
            confidence: 0.95,
            is_reviewed: true,
            validation_error: None,
            bbox: None,
            page_number: 1,
        },
        ExtractedCellData {
            id: Some(5),
            column_key: "total".to_string(),
            raw_value: Some("115.00".to_string()),
            normalized_value: Some("115.00".to_string()),
            confidence: 0.95,
            is_reviewed: true,
            validation_error: None,
            bbox: None,
            page_number: 1,
        },
    ];

    let errors = validate_record_row(&cols, &rules, &cells_valid);
    assert!(errors.is_empty(), "Expected 0 errors but got: {:?}", errors);

    // Case 2: Math sum mismatch
    let mut cells_mismatch = cells_valid.clone();
    cells_mismatch[4].normalized_value = Some("120.00".to_string());
    let errors_math = validate_record_row(&cols, &rules, &cells_mismatch);
    assert_eq!(errors_math.len(), 1);
    assert!(errors_math.contains_key("total"));
    assert!(errors_math["total"].contains("Calculated 115.00 vs Expected 120.00"));

    // Case 3: Due date before invoice date
    let mut cells_date_err = cells_valid.clone();
    cells_date_err[1].normalized_value = Some("2026-09-01".to_string());
    let errors_date = validate_record_row(&cols, &rules, &cells_date_err);
    assert_eq!(errors_date.len(), 1);
    assert!(errors_date.contains_key("due_date"));
    assert!(errors_date["due_date"].contains("cannot be earlier than start date"));
}

#[test]
fn test_batch_summary_and_export_formats() {
    let cols = vec![
        ColumnDefinition::new("vendor", "Vendor Name", ColumnDataType::String),
        ColumnDefinition::new("total", "Total Amount", ColumnDataType::Monetary),
    ];

    let mut cells1 = HashMap::new();
    cells1.insert(
        "vendor".to_string(),
        ExtractedCellData {
            id: Some(1),
            column_key: "vendor".to_string(),
            raw_value: Some("Apex Logistics".to_string()),
            normalized_value: Some("Apex Logistics".to_string()),
            confidence: 0.95,
            is_reviewed: true,
            validation_error: None,
            bbox: None,
            page_number: 1,
        },
    );
    cells1.insert(
        "total".to_string(),
        ExtractedCellData {
            id: Some(2),
            column_key: "total".to_string(),
            raw_value: Some("500.00".to_string()),
            normalized_value: Some("500.00".to_string()),
            confidence: 0.98,
            is_reviewed: true,
            validation_error: None,
            bbox: None,
            page_number: 1,
        },
    );

    let mut cells2 = HashMap::new();
    cells2.insert(
        "vendor".to_string(),
        ExtractedCellData {
            id: Some(3),
            column_key: "vendor".to_string(),
            raw_value: Some("Zenith Technologies".to_string()),
            normalized_value: Some("Zenith Technologies".to_string()),
            confidence: 0.92,
            is_reviewed: false,
            validation_error: None,
            bbox: None,
            page_number: 1,
        },
    );
    cells2.insert(
        "total".to_string(),
        ExtractedCellData {
            id: Some(4),
            column_key: "total".to_string(),
            raw_value: Some("1500.00".to_string()),
            normalized_value: Some("1500.00".to_string()),
            confidence: 0.90,
            is_reviewed: false,
            validation_error: None,
            bbox: None,
            page_number: 1,
        },
    );

    let rows = vec![
        ExportRow {
            record_id: 1,
            filename: "apex.pdf".to_string(),
            status: "approved".to_string(),
            overall_confidence: 0.965,
            cells: cells1,
        },
        ExportRow {
            record_id: 2,
            filename: "zenith.pdf".to_string(),
            status: "extracted".to_string(),
            overall_confidence: 0.91,
            cells: cells2,
        },
    ];

    let summary = compute_batch_summary(&cols, &rows);
    assert_eq!(summary.total_documents, 2);
    assert_eq!(summary.approved_documents, 1);
    assert_eq!(summary.flagged_fields_count, 0);
    assert!(summary.overall_confidence > 0.93);

    let total_agg = &summary.field_aggregations["total"];
    assert_eq!(total_agg.count, 2);
    assert_eq!(total_agg.populated_count, 2);
    assert_eq!(total_agg.sum, Some(2000.00));
    assert_eq!(total_agg.min_numeric, Some(500.00));
    assert_eq!(total_agg.max_numeric, Some(1500.00));

    // Test JSON lines
    let jsonl = export_json_lines(&cols, &rows);
    assert_eq!(jsonl.lines().count(), 2);
    assert!(jsonl.contains("\"filename\":\"apex.pdf\""));
    assert!(jsonl.contains("\"fields\":{\"total\":\"500.00\",\"vendor\":\"Apex Logistics\"}"));

    // Test Markdown table
    let md = export_markdown_table(&cols, &rows);
    assert!(md.contains("| ID | Document | Vendor Name | Total Amount | Status |"));
    assert!(md.contains("| 1 | apex.pdf | Apex Logistics | 500.00 | approved |"));
    assert!(md.contains("| 2 | zenith.pdf | Zenith Technologies | 1500.00 | extracted |"));
}
