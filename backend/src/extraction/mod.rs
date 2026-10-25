//! Intelligent Document Processing (IDP) and Data Entry engine for FileDeck.
//!
//! Provides schema definitions, field validators, currency and date normalizers,
//! cross-field mathematical reconciliation (e.g. Subtotal + Tax == Total),
//! and high-performance streaming tabular data exporters (CSV, JSON Lines, TSV).

pub mod barcode;
pub mod exporter;
pub mod normalizer;
pub mod validator;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Data type supported for extraction schema columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnDataType {
    String,
    Date,
    Monetary,
    Integer,
    Boolean,
}

impl Default for ColumnDataType {
    fn default() -> Self {
        Self::String
    }
}

impl ColumnDataType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Date => "date",
            Self::Monetary => "monetary",
            Self::Integer => "integer",
            Self::Boolean => "boolean",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().trim() {
            "date" => Self::Date,
            "monetary" | "money" | "currency" | "amount" => Self::Monetary,
            "integer" | "int" | "number" => Self::Integer,
            "boolean" | "bool" => Self::Boolean,
            _ => Self::String,
        }
    }
}

/// Column specification within an extraction schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub r#type: ColumnDataType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub regex: Option<String>,
    #[serde(default)]
    pub hints: Vec<String>,
}

impl ColumnDefinition {
    pub fn new(key: impl Into<String>, label: impl Into<String>, data_type: ColumnDataType) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            r#type: data_type,
            required: false,
            regex: None,
            hints: Vec::new(),
        }
    }

    pub fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn with_regex(mut self, pattern: impl Into<String>) -> Self {
        self.regex = Some(pattern.into());
        self
    }

    pub fn with_hints(mut self, hints: Vec<String>) -> Self {
        self.hints = hints;
        self
    }
}

/// Validation rule for cross-field consistency and mathematical reconciliation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub rule: String,
    pub target: String,
    #[serde(default)]
    pub components: Vec<String>,
    #[serde(default)]
    pub tolerance: Option<f64>,
    #[serde(default)]
    pub description: Option<String>,
}

/// High-level schema definition for batch document extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDefinition {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub document_type_id: Option<i64>,
    pub columns: Vec<ColumnDefinition>,
    #[serde(default)]
    pub validation_rules: Vec<ValidationRule>,
}

impl SchemaDefinition {
    pub fn new(name: impl Into<String>, columns: Vec<ColumnDefinition>) -> Self {
        Self {
            id: None,
            name: name.into(),
            description: None,
            document_type_id: None,
            columns,
            validation_rules: Vec::new(),
        }
    }

    pub fn get_column(&self, key: &str) -> Option<&ColumnDefinition> {
        self.columns.iter().find(|c| c.key == key)
    }

    pub fn required_keys(&self) -> Vec<&str> {
        self.columns
            .iter()
            .filter(|c| c.required)
            .map(|c| c.key.as_str())
            .collect()
    }
}

/// Individual extracted cell item in a review grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedCellData {
    pub id: Option<i64>,
    pub column_key: String,
    pub raw_value: Option<String>,
    pub normalized_value: Option<String>,
    pub confidence: f64,
    pub is_reviewed: bool,
    pub validation_error: Option<String>,
    pub bbox: Option<[f64; 4]>,
    pub page_number: i32,
}

/// Summary metrics for an IDP extraction batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSummary {
    pub total_documents: usize,
    pub processed_documents: usize,
    pub approved_documents: usize,
    pub flagged_fields_count: usize,
    pub overall_confidence: f64,
    pub field_aggregations: HashMap<String, FieldAggregation>,
}

/// Aggregated numeric or count metrics for a single column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldAggregation {
    pub count: usize,
    pub populated_count: usize,
    pub null_count: usize,
    pub average_confidence: f64,
    pub sum: Option<f64>,
    pub min_numeric: Option<f64>,
    pub max_numeric: Option<f64>,
}
