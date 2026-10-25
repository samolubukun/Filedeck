//! High-performance tabular data exporters for document extraction batches.
//!
//! Generates RFC 4180 compliant CSV with UTF-8 BOM for Microsoft Excel,
//! TSV, streaming JSON Lines, Markdown audit tables, and statistical summaries.

use crate::extraction::{BatchSummary, ColumnDefinition, ExtractedCellData, FieldAggregation};
use std::collections::HashMap;

/// Options for tabular file export.
#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub include_record_id: bool,
    pub include_filename: bool,
    pub include_confidence: bool,
    pub include_utf8_bom: bool,
    pub delimiter: char,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            include_record_id: true,
            include_filename: true,
            include_confidence: false,
            include_utf8_bom: true,
            delimiter: ',',
        }
    }
}

/// A row of extracted document data ready for export.
#[derive(Debug, Clone)]
pub struct ExportRow {
    pub record_id: i64,
    pub filename: String,
    pub status: String,
    pub overall_confidence: f64,
    pub cells: HashMap<String, ExtractedCellData>,
}

/// Escapes a field value for CSV/TSV output according to RFC 4180.
pub fn escape_csv_field(val: &str, delimiter: char) -> String {
    let needs_quotes = val.contains(delimiter)
        || val.contains('"')
        || val.contains('\n')
        || val.contains('\r');

    if needs_quotes {
        let escaped = val.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        val.to_string()
    }
}

/// Generates a CSV or TSV byte string from columns and row data.
pub fn export_delimited(
    columns: &[ColumnDefinition],
    rows: &[ExportRow],
    options: &ExportOptions,
) -> Vec<u8> {
    let mut out = Vec::new();

    // Microsoft Excel UTF-8 Byte Order Mark (BOM: EF BB BF)
    if options.include_utf8_bom {
        out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }

    let mut headers = Vec::new();
    if options.include_record_id {
        headers.push(escape_csv_field("Record ID", options.delimiter));
    }
    if options.include_filename {
        headers.push(escape_csv_field("Document Filename", options.delimiter));
    }
    for col in columns {
        headers.push(escape_csv_field(&col.label, options.delimiter));
    }
    if options.include_confidence {
        headers.push(escape_csv_field("Confidence Score", options.delimiter));
    }

    let header_line = headers.join(&options.delimiter.to_string()) + "\r\n";
    out.extend_from_slice(header_line.as_bytes());

    for row in rows {
        let mut line_fields = Vec::new();

        if options.include_record_id {
            line_fields.push(escape_csv_field(&row.record_id.to_string(), options.delimiter));
        }
        if options.include_filename {
            line_fields.push(escape_csv_field(&row.filename, options.delimiter));
        }

        for col in columns {
            let val = row
                .cells
                .get(&col.key)
                .and_then(|c| c.normalized_value.as_deref().or(c.raw_value.as_deref()))
                .unwrap_or("");
            line_fields.push(escape_csv_field(val, options.delimiter));
        }

        if options.include_confidence {
            let conf_pct = format!("{:.1}%", row.overall_confidence * 100.0);
            line_fields.push(escape_csv_field(&conf_pct, options.delimiter));
        }

        let row_line = line_fields.join(&options.delimiter.to_string()) + "\r\n";
        out.extend_from_slice(row_line.as_bytes());
    }

    out
}

/// Generates a JSON Lines (`.jsonl`) stream.
pub fn export_json_lines(columns: &[ColumnDefinition], rows: &[ExportRow]) -> String {
    let mut lines = Vec::with_capacity(rows.len());

    for row in rows {
        let mut row_obj = serde_json::Map::new();
        row_obj.insert("record_id".to_string(), serde_json::json!(row.record_id));
        row_obj.insert("filename".to_string(), serde_json::json!(row.filename));
        row_obj.insert("status".to_string(), serde_json::json!(row.status));
        row_obj.insert("overall_confidence".to_string(), serde_json::json!(row.overall_confidence));

        let mut data_obj = serde_json::Map::new();
        for col in columns {
            let cell = row.cells.get(&col.key);
            let val = cell.and_then(|c| c.normalized_value.as_ref().or(c.raw_value.as_ref()));
            data_obj.insert(col.key.clone(), serde_json::json!(val));
        }
        row_obj.insert("fields".to_string(), serde_json::Value::Object(data_obj));

        if let Ok(json_str) = serde_json::to_string(&row_obj) {
            lines.push(json_str);
        }
    }

    lines.join("\n")
}

/// Generates a GitHub-flavored Markdown table for audits and reports.
pub fn export_markdown_table(columns: &[ColumnDefinition], rows: &[ExportRow]) -> String {
    let mut md = String::new();

    // Headers
    md.push_str("| ID | Document |");
    for col in columns {
        md.push_str(&format!(" {} |", col.label));
    }
    md.push_str(" Status |\n");

    // Divider
    md.push_str("|:---|:---|");
    for _ in columns {
        md.push_str(":---|");
    }
    md.push_str(":---|\n");

    // Rows
    for row in rows {
        md.push_str(&format!("| {} | {} |", row.record_id, row.filename));
        for col in columns {
            let val = row
                .cells
                .get(&col.key)
                .and_then(|c| c.normalized_value.as_deref().or(c.raw_value.as_deref()))
                .unwrap_or("-");
            md.push_str(&format!(" {} |", val));
        }
        md.push_str(&format!(" {} |\n", row.status));
    }

    md
}

/// Computes statistical aggregations and confidence metrics across an extraction batch.
pub fn compute_batch_summary(columns: &[ColumnDefinition], rows: &[ExportRow]) -> BatchSummary {
    let total_documents = rows.len();
    let mut approved_documents = 0;
    let mut flagged_fields_count = 0;
    let mut total_confidence = 0.0;

    let mut field_aggs: HashMap<String, (usize, usize, f64, Option<f64>, Option<f64>, Option<f64>)> =
        HashMap::new();

    for col in columns {
        field_aggs.insert(col.key.clone(), (0, 0, 0.0, None, None, None));
    }

    for row in rows {
        if row.status == "approved" {
            approved_documents += 1;
        }
        total_confidence += row.overall_confidence;

        for col in columns {
            let entry = field_aggs.get_mut(&col.key).unwrap();
            entry.0 += 1; // total count

            if let Some(cell) = row.cells.get(&col.key) {
                if cell.validation_error.is_some() || (!cell.is_reviewed && cell.confidence < 0.6) {
                    flagged_fields_count += 1;
                }

                let effective_val = cell.normalized_value.as_deref().or(cell.raw_value.as_deref());
                if let Some(val) = effective_val {
                    if !val.trim().is_empty() {
                        entry.1 += 1; // populated
                        entry.2 += cell.confidence; // sum conf

                        if let Ok(num) = val.parse::<f64>() {
                            let curr_sum = entry.3.unwrap_or(0.0) + num;
                            entry.3 = Some(curr_sum);

                            let curr_min = entry.4.map(|m: f64| m.min(num)).unwrap_or(num);
                            entry.4 = Some(curr_min);

                            let curr_max = entry.5.map(|m: f64| m.max(num)).unwrap_or(num);
                            entry.5 = Some(curr_max);
                        }
                    }
                }
            }
        }
    }

    let overall_confidence = if total_documents > 0 {
        total_confidence / total_documents as f64
    } else {
        0.0
    };

    let mut field_aggregations = HashMap::new();
    for (key, (total, populated, sum_conf, sum_num, min_num, max_num)) in field_aggs {
        let avg_conf = if populated > 0 {
            sum_conf / populated as f64
        } else {
            0.0
        };

        field_aggregations.insert(
            key,
            FieldAggregation {
                count: total,
                populated_count: populated,
                null_count: total.saturating_sub(populated),
                average_confidence: avg_conf,
                sum: sum_num,
                min_numeric: min_num,
                max_numeric: max_num,
            },
        );
    }

    BatchSummary {
        total_documents,
        processed_documents: total_documents,
        approved_documents,
        flagged_fields_count,
        overall_confidence,
        field_aggregations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extraction::ColumnDataType;

    #[test]
    fn test_csv_rfc4180_escaping() {
        assert_eq!(escape_csv_field("Simple", ','), "Simple");
        assert_eq!(escape_csv_field("Hello, World", ','), "\"Hello, World\"");
        assert_eq!(escape_csv_field("Line\nBreak", ','), "\"Line\nBreak\"");
        assert_eq!(escape_csv_field("With \"Quotes\"", ','), "\"With \"\"Quotes\"\"\"");
    }

    #[test]
    fn test_delimited_export() {
        let cols = vec![
            ColumnDefinition::new("vendor", "Vendor Name", ColumnDataType::String),
            ColumnDefinition::new("total", "Total Amount", ColumnDataType::Monetary),
        ];

        let mut cells = HashMap::new();
        cells.insert(
            "vendor".to_string(),
            ExtractedCellData {
                id: Some(1),
                column_key: "vendor".to_string(),
                raw_value: Some("Acme Corp, Inc.".to_string()),
                normalized_value: Some("Acme Corp, Inc.".to_string()),
                confidence: 0.95,
                is_reviewed: true,
                validation_error: None,
                bbox: None,
                page_number: 1,
            },
        );
        cells.insert(
            "total".to_string(),
            ExtractedCellData {
                id: Some(2),
                column_key: "total".to_string(),
                raw_value: Some("1250.00".to_string()),
                normalized_value: Some("1250.00".to_string()),
                confidence: 0.99,
                is_reviewed: true,
                validation_error: None,
                bbox: None,
                page_number: 1,
            },
        );

        let rows = vec![ExportRow {
            record_id: 101,
            filename: "invoice.pdf".to_string(),
            status: "approved".to_string(),
            overall_confidence: 0.97,
            cells,
        }];

        let bytes = export_delimited(&cols, &rows, &ExportOptions::default());
        let csv_str = String::from_utf8_lossy(&bytes);

        assert!(csv_str.contains("Record ID,Document Filename,Vendor Name,Total Amount"));
        assert!(csv_str.contains("101,invoice.pdf,\"Acme Corp, Inc.\",1250.00"));
    }
}
