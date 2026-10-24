//! Integration tests for FileDeck's saved views, custom filter presets, and UI display configurations.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedViewDisplayFields {
    pub columns: Vec<String>,
    pub sort_field: String,
    pub sort_reverse: bool,
    pub page_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedViewModel {
    pub id: i64,
    pub name: String,
    pub show_on_dashboard: bool,
    pub show_in_sidebar: bool,
    pub sort_field: String,
    pub sort_reverse: bool,
    pub filter_rules: Vec<FilterRuleItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FilterRuleItem {
    pub rule_type: i64,
    pub value: Option<String>,
}

#[test]
fn test_saved_view_display_preferences() {
    let prefs = SavedViewDisplayFields {
        columns: vec![
            "title".to_string(),
            "correspondent".to_string(),
            "created".to_string(),
            "tags".to_string(),
            "archive_serial_number".to_string(),
        ],
        sort_field: "created".to_string(),
        sort_reverse: true,
        page_size: 50,
    };

    assert_eq!(prefs.columns.len(), 5);
    assert_eq!(prefs.sort_field, "created");
    assert!(prefs.sort_reverse);
    assert_eq!(prefs.page_size, 50);
}

#[test]
fn test_saved_view_filter_rules_serialization() {
    let view = SavedViewModel {
        id: 10,
        name: "Tax Invoices 2024".to_string(),
        show_on_dashboard: true,
        show_in_sidebar: true,
        sort_field: "created".to_string(),
        sort_reverse: true,
        filter_rules: vec![
            FilterRuleItem {
                rule_type: 1, // Has tag
                value: Some("2".to_string()),
            },
            FilterRuleItem {
                rule_type: 2, // Has correspondent
                value: Some("15".to_string()),
            },
            FilterRuleItem {
                rule_type: 8, // Created after date
                value: Some("2024-01-01".to_string()),
            },
        ],
    };

    let serialized = serde_json::to_string(&view).unwrap();
    let deserialized: SavedViewModel = serde_json::from_str(&serialized).unwrap();
    assert_eq!(view, deserialized);
    assert_eq!(deserialized.filter_rules.len(), 3);
}

#[test]
fn test_saved_view_url_param_generation() {
    let generate_query_string = |view: &SavedViewModel| -> String {
        let mut parts = vec![
            format!("ordering={}{}", if view.sort_reverse { "-" } else { "" }, view.sort_field),
        ];
        for rule in &view.filter_rules {
            if let Some(ref val) = rule.value {
                match rule.rule_type {
                    1 => parts.push(format!("tags__id__all={val}")),
                    2 => parts.push(format!("correspondent__id={val}")),
                    8 => parts.push(format!("created__date__gte={val}")),
                    _ => {}
                }
            }
        }
        parts.join("&")
    };

    let view = SavedViewModel {
        id: 1,
        name: "Recent Finance".to_string(),
        show_on_dashboard: false,
        show_in_sidebar: true,
        sort_field: "created".to_string(),
        sort_reverse: true,
        filter_rules: vec![
            FilterRuleItem {
                rule_type: 1,
                value: Some("5".to_string()),
            },
            FilterRuleItem {
                rule_type: 8,
                value: Some("2024-01-01".to_string()),
            },
        ],
    };

    let qs = generate_query_string(&view);
    assert_eq!(qs, "ordering=-created&tags__id__all=5&created__date__gte=2024-01-01");
}
