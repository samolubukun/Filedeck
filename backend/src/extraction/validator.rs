//! Validation engine for extraction schemas, fields, and cross-column business rules.
//!
//! Evaluates required constraints, regular expression patterns, type fidelity,
//! and mathematical reconciliation rules (e.g. Subtotal + Tax == Total Amount).

use crate::extraction::normalizer::{normalize_date, normalize_monetary};
use crate::extraction::{ColumnDataType, ColumnDefinition, ExtractedCellData, ValidationRule};
use chrono::NaiveDate;
use regex::Regex;
use std::collections::HashMap;

/// Result of evaluating a validation rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationCheckResult {
    pub is_valid: bool,
    pub field_key: String,
    pub error_message: Option<String>,
}

/// Validates an individual cell value against its column definition.
pub fn validate_cell_value(
    col: &ColumnDefinition,
    raw_value: Option<&str>,
    normalized_value: Option<&str>,
    confidence: f64,
) -> Option<String> {
    let effective_val = normalized_value.or(raw_value).map(|s| s.trim()).unwrap_or("");

    // 1. Required constraint
    if col.required && (effective_val.is_empty() || confidence < 0.25) {
        if effective_val.is_empty() {
            return Some(format!("Required field '{}' is empty", col.label));
        } else {
            return Some(format!(
                "Required field '{}' detected with low confidence ({:.0}%)",
                col.label,
                confidence * 100.0
            ));
        }
    }

    if effective_val.is_empty() {
        return None;
    }

    // 2. Data type format check
    match col.r#type {
        ColumnDataType::Monetary => {
            if normalize_monetary(effective_val).is_none() {
                return Some(format!(
                    "Value '{}' is not a valid currency or monetary amount",
                    effective_val
                ));
            }
        }
        ColumnDataType::Date => {
            if normalize_date(effective_val).is_none() {
                return Some(format!(
                    "Value '{}' does not match any recognized date format",
                    effective_val
                ));
            }
        }
        ColumnDataType::Integer => {
            if effective_val.parse::<i64>().is_err() {
                return Some(format!("Value '{}' is not a valid integer", effective_val));
            }
        }
        ColumnDataType::Boolean => {
            let lower = effective_val.to_lowercase();
            if !matches!(
                lower.as_str(),
                "true" | "false" | "yes" | "no" | "1" | "0" | "y" | "n"
            ) {
                return Some(format!("Value '{}' is not a valid boolean", effective_val));
            }
        }
        ColumnDataType::String => {}
    }

    // 3. Regex pattern check
    if let Some(ref pattern) = col.regex {
        if let Ok(re) = Regex::new(pattern) {
            if !re.is_match(effective_val) {
                return Some(format!(
                    "Value does not match required format pattern ({})",
                    pattern
                ));
            }
        }
    }

    None
}

/// Evaluates a cross-field mathematical or logical validation rule against a row's cells.
pub fn evaluate_rule(
    rule: &ValidationRule,
    cells_by_key: &HashMap<String, &ExtractedCellData>,
) -> Option<ValidationCheckResult> {
    let target_key = &rule.target;
    let target_cell = cells_by_key.get(target_key)?;

    match rule.rule.to_lowercase().as_str() {
        // Mathematical sum: components[0] + components[1] + ... == target
        "sum" | "addition" => {
            let target_val_str = target_cell
                .normalized_value
                .as_deref()
                .or(target_cell.raw_value.as_deref())?;
            let target_num = target_val_str.parse::<f64>().ok()?;

            let mut sum = 0.0;
            let mut all_components_present = true;

            for comp_key in &rule.components {
                if let Some(comp_cell) = cells_by_key.get(comp_key) {
                    if let Some(val_str) = comp_cell
                        .normalized_value
                        .as_deref()
                        .or(comp_cell.raw_value.as_deref())
                    {
                        if let Ok(n) = val_str.parse::<f64>() {
                            sum += n;
                        } else {
                            all_components_present = false;
                            break;
                        }
                    } else {
                        all_components_present = false;
                        break;
                    }
                } else {
                    all_components_present = false;
                    break;
                }
            }

            if all_components_present {
                let tolerance = rule.tolerance.unwrap_or(0.05);
                let diff = (sum - target_num).abs();
                if diff > tolerance {
                    let desc = rule.description.as_deref().unwrap_or(
                        "Sum of component fields does not match target total amount",
                    );
                    return Some(ValidationCheckResult {
                        is_valid: false,
                        field_key: target_key.clone(),
                        error_message: Some(format!(
                            "{}: Calculated {:.2} vs Expected {:.2}",
                            desc, sum, target_num
                        )),
                    });
                } else {
                    return Some(ValidationCheckResult {
                        is_valid: true,
                        field_key: target_key.clone(),
                        error_message: None,
                    });
                }
            }
        }

        // Chronological order: components[0] <= target (e.g. invoice_date <= due_date)
        "date_order" | "date_range" => {
            if let Some(start_key) = rule.components.first() {
                if let Some(start_cell) = cells_by_key.get(start_key) {
                    let start_date_str = start_cell
                        .normalized_value
                        .as_deref()
                        .or(start_cell.raw_value.as_deref())?;
                    let target_date_str = target_cell
                        .normalized_value
                        .as_deref()
                        .or(target_cell.raw_value.as_deref())?;

                    if let (Ok(d_start), Ok(d_target)) = (
                        NaiveDate::parse_from_str(start_date_str, "%Y-%m-%d"),
                        NaiveDate::parse_from_str(target_date_str, "%Y-%m-%d"),
                    ) {
                        if d_start > d_target {
                            return Some(ValidationCheckResult {
                                is_valid: false,
                                field_key: target_key.clone(),
                                error_message: Some(format!(
                                    "Target date '{}' cannot be earlier than start date '{}'",
                                    target_date_str, start_date_str
                                )),
                            });
                        } else {
                            return Some(ValidationCheckResult {
                                is_valid: true,
                                field_key: target_key.clone(),
                                error_message: None,
                            });
                        }
                    }
                }
            }
        }

        _ => {}
    }

    None
}

/// Evaluates all column constraints and cross-field rules for a record row.
/// Returns a map of `column_key -> validation_error_message`.
pub fn validate_record_row(
    columns: &[ColumnDefinition],
    rules: &[ValidationRule],
    cells: &[ExtractedCellData],
) -> HashMap<String, String> {
    let mut errors = HashMap::new();
    let mut cells_by_key: HashMap<String, &ExtractedCellData> = HashMap::new();

    for cell in cells {
        cells_by_key.insert(cell.column_key.clone(), cell);
    }

    // 1. Column level validation
    for col in columns {
        let cell_opt = cells_by_key.get(&col.key);
        let raw = cell_opt.and_then(|c| c.raw_value.as_deref());
        let norm = cell_opt.and_then(|c| c.normalized_value.as_deref());
        let conf = cell_opt.map(|c| c.confidence).unwrap_or(0.0);

        if let Some(err) = validate_cell_value(col, raw, norm, conf) {
            errors.insert(col.key.clone(), err);
        }
    }

    // 2. Cross-field rules validation
    for rule in rules {
        if let Some(res) = evaluate_rule(rule, &cells_by_key) {
            if !res.is_valid {
                if let Some(err) = res.error_message {
                    errors.insert(res.field_key, err);
                }
            }
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_validation_required() {
        let col = ColumnDefinition::new("inv_num", "Invoice Number", ColumnDataType::String)
            .with_required(true);

        assert!(validate_cell_value(&col, None, None, 0.0).is_some());
        assert!(validate_cell_value(&col, Some("INV-001"), Some("INV-001"), 0.95).is_none());
        assert!(validate_cell_value(&col, Some("INV-001"), Some("INV-001"), 0.15).is_some());
    }

    #[test]
    fn test_cross_field_sum_reconciliation() {
        let rule = ValidationRule {
            rule: "sum".to_string(),
            target: "total".to_string(),
            components: vec!["subtotal".to_string(), "tax".to_string()],
            tolerance: Some(0.02),
            description: Some("Subtotal + Tax == Total".to_string()),
        };

        let subtotal_cell = ExtractedCellData {
            id: None,
            column_key: "subtotal".to_string(),
            raw_value: Some("100.00".to_string()),
            normalized_value: Some("100.00".to_string()),
            confidence: 0.95,
            is_reviewed: true,
            validation_error: None,
            bbox: None,
            page_number: 1,
        };

        let tax_cell = ExtractedCellData {
            id: None,
            column_key: "tax".to_string(),
            raw_value: Some("15.00".to_string()),
            normalized_value: Some("15.00".to_string()),
            confidence: 0.95,
            is_reviewed: true,
            validation_error: None,
            bbox: None,
            page_number: 1,
        };

        let total_valid = ExtractedCellData {
            id: None,
            column_key: "total".to_string(),
            raw_value: Some("115.00".to_string()),
            normalized_value: Some("115.00".to_string()),
            confidence: 0.95,
            is_reviewed: true,
            validation_error: None,
            bbox: None,
            page_number: 1,
        };

        let mut map = HashMap::new();
        map.insert("subtotal".to_string(), &subtotal_cell);
        map.insert("tax".to_string(), &tax_cell);
        map.insert("total".to_string(), &total_valid);

        let res = evaluate_rule(&rule, &map);
        assert!(res.is_some());
        assert!(res.unwrap().is_valid);
    }
}
