//! Integration tests for IDP batch processing pipelines and database state machines.

use filedeck_backend::extraction::exporter::{export_delimited, ExportOptions, ExportRow};
use filedeck_backend::extraction::normalizer::{normalize_date, normalize_monetary};
use filedeck_backend::extraction::validator::validate_record_row;
use filedeck_backend::extraction::{
    ColumnDataType, ColumnDefinition, ExtractedCellData, SchemaDefinition, ValidationRule,
};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchStatus {
    Pending,
    Processing,
    NeedsReview,
    Approved,
}

impl BatchStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::NeedsReview => "needs_review",
            Self::Approved => "approved",
        }
    }
}

/// Simulated in-memory database record representing an extraction record
#[derive(Debug, Clone)]
pub struct MockRecord {
    pub id: i64,
    pub batch_id: i64,
    pub document_id: i64,
    pub filename: String,
    pub status: String,
    pub confidence_score: f64,
    pub cells: HashMap<String, ExtractedCellData>,
}

impl MockRecord {
    pub fn new(id: i64, batch_id: i64, doc_id: i64, filename: &str) -> Self {
        Self {
            id,
            batch_id,
            document_id: doc_id,
            filename: filename.to_string(),
            status: "extracted".to_string(),
            confidence_score: 0.95,
            cells: HashMap::new(),
        }
    }

    pub fn set_cell(&mut self, key: &str, raw: Option<&str>, norm: Option<&str>, conf: f64, reviewed: bool) {
        self.cells.insert(
            key.to_string(),
            ExtractedCellData {
                id: Some(self.cells.len() as i64 + 1),
                column_key: key.to_string(),
                raw_value: raw.map(str::to_string),
                normalized_value: norm.map(str::to_string),
                confidence: conf,
                is_reviewed: reviewed,
                validation_error: None,
                bbox: Some([0.1, 0.1, 0.3, 0.15]),
                page_number: 1,
            },
        );
    }

    pub fn approve(&mut self) {
        self.status = "approved".to_string();
        for cell in self.cells.values_mut() {
            cell.is_reviewed = true;
            cell.validation_error = None;
        }
    }
}

#[test]
fn test_batch_lifecycle_and_record_transitions() {
    let schema = SchemaDefinition::new(
        "Procurement Invoices",
        vec![
            ColumnDefinition::new("invoice_number", "Invoice #", ColumnDataType::String)
                .with_required(true),
            ColumnDefinition::new("invoice_date", "Date", ColumnDataType::Date)
                .with_required(true),
            ColumnDefinition::new("subtotal", "Subtotal", ColumnDataType::Monetary),
            ColumnDefinition::new("tax", "Tax", ColumnDataType::Monetary),
            ColumnDefinition::new("total", "Total", ColumnDataType::Monetary)
                .with_required(true),
        ],
    );

    let mut record = MockRecord::new(1, 100, 501, "acme_inv_101.pdf");
    record.set_cell("invoice_number", Some("INV-2026-001"), Some("INV-2026-001"), 0.98, true);
    record.set_cell("invoice_date", Some("10/25/2026"), normalize_date("10/25/2026").as_deref(), 0.95, true);
    record.set_cell("subtotal", Some("$1,000.00"), normalize_monetary("$1,000.00").as_deref(), 0.96, true);
    record.set_cell("tax", Some("$150.00"), normalize_monetary("$150.00").as_deref(), 0.94, true);
    record.set_cell("total", Some("$1,150.00"), normalize_monetary("$1,150.00").as_deref(), 0.99, true);

    let rules = vec![ValidationRule {
        rule: "sum".to_string(),
        target: "total".to_string(),
        components: vec!["subtotal".to_string(), "tax".to_string()],
        tolerance: Some(0.01),
        description: Some("Subtotal + Tax == Total".to_string()),
    }];

    let cell_list: Vec<ExtractedCellData> = record.cells.values().cloned().collect();
    let errors = validate_record_row(&schema.columns, &rules, &cell_list);
    assert!(errors.is_empty());

    // Record approval
    assert_eq!(record.status, "extracted");
    record.approve();
    assert_eq!(record.status, "approved");
    assert!(record.cells["invoice_number"].is_reviewed);
    assert!(record.cells["total"].is_reviewed);
}

#[test]
fn test_human_in_the_loop_inline_cell_correction() {
    let schema = SchemaDefinition::new(
        "HR Records",
        vec![
            ColumnDefinition::new("emp_id", "Employee ID", ColumnDataType::String)
                .with_required(true)
                .with_regex(r"^EMP-\d{4}$"),
            ColumnDefinition::new("name", "Full Name", ColumnDataType::String)
                .with_required(true),
            ColumnDefinition::new("salary", "Salary", ColumnDataType::Monetary),
        ],
    );

    // Initial noisy scan extraction
    let mut record = MockRecord::new(2, 101, 502, "scanned_hr_sheet.pdf");
    record.set_cell("emp_id", Some("EMP-XXXX"), Some("EMP-XXXX"), 0.35, false); // Failed regex & low confidence
    record.set_cell("name", Some("Jane Doe"), Some("Jane Doe"), 0.95, true);
    record.set_cell("salary", Some("75,000"), normalize_monetary("75,000").as_deref(), 0.92, false);

    // Validate: should detect regex failure on emp_id
    let cell_list_initial: Vec<ExtractedCellData> = record.cells.values().cloned().collect();
    let initial_errors = validate_record_row(&schema.columns, &[], &cell_list_initial);
    assert_eq!(initial_errors.len(), 1);
    assert!(initial_errors.contains_key("emp_id"));

    // User corrects the cell in the frontend review grid (PATCH /api/extraction/cells/:id/)
    record.set_cell("emp_id", Some("EMP-9021"), Some("EMP-9021"), 1.0, true);

    // Re-validate: should now be 100% valid
    let cell_list_corrected: Vec<ExtractedCellData> = record.cells.values().cloned().collect();
    let corrected_errors = validate_record_row(&schema.columns, &[], &cell_list_corrected);
    assert!(corrected_errors.is_empty(), "Expected 0 errors after inline correction");

    // Approve the record
    record.approve();
    assert_eq!(record.status, "approved");
}

#[test]
fn test_batch_export_pipeline() {
    let cols = vec![
        ColumnDefinition::new("inv_num", "Invoice Number", ColumnDataType::String),
        ColumnDefinition::new("amount", "Amount", ColumnDataType::Monetary),
    ];

    let mut record1 = MockRecord::new(10, 1, 1001, "inv_1.pdf");
    record1.set_cell("inv_num", Some("INV-001"), Some("INV-001"), 0.99, true);
    record1.set_cell("amount", Some("450.00"), Some("450.00"), 0.98, true);

    let mut record2 = MockRecord::new(11, 1, 1002, "inv_2.pdf");
    record2.set_cell("inv_num", Some("INV-002"), Some("INV-002"), 0.97, true);
    record2.set_cell("amount", Some("890.50"), Some("890.50"), 0.99, true);

    let export_rows = vec![
        ExportRow {
            record_id: record1.id,
            filename: record1.filename,
            status: "approved".to_string(),
            overall_confidence: 0.985,
            cells: record1.cells,
        },
        ExportRow {
            record_id: record2.id,
            filename: record2.filename,
            status: "approved".to_string(),
            overall_confidence: 0.98,
            cells: record2.cells,
        },
    ];

    // Export CSV with UTF-8 BOM
    let csv_bytes = export_delimited(&cols, &export_rows, &ExportOptions::default());
    assert_eq!(&csv_bytes[0..3], &[0xEF, 0xBB, 0xBF]); // UTF-8 BOM check

    let csv_str = String::from_utf8_lossy(&csv_bytes[3..]);
    assert!(csv_str.contains("Record ID,Document Filename,Invoice Number,Amount"));
    assert!(csv_str.contains("10,inv_1.pdf,INV-001,450.00"));
    assert!(csv_str.contains("11,inv_2.pdf,INV-002,890.50"));

    // Export TSV
    let mut tsv_opts = ExportOptions::default();
    tsv_opts.delimiter = '\t';
    tsv_opts.include_utf8_bom = false;
    let tsv_bytes = export_delimited(&cols, &export_rows, &tsv_opts);
    let tsv_str = String::from_utf8_lossy(&tsv_bytes);
    assert!(tsv_str.contains("Record ID\tDocument Filename\tInvoice Number\tAmount"));
    assert!(tsv_str.contains("10\tinv_1.pdf\tINV-001\t450.00"));
}
