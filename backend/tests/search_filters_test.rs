//! Integration tests for FileDeck's search filters and query translation.
//!
//! Tests verify:
//! - SQL ORDER BY translation with NULLs last semantics
//! - Ascending and descending directions
//! - Subqueries for relational and custom field sort keys
//! - Safe rejection of invalid, malicious, or unknown sort fields
//! - Parameter extraction with multi-value precedence
//! - Search modes and query configuration

use filedeck_backend::search::filters::{get, order_by, Params};
use filedeck_backend::search::SearchMode;

#[test]
fn test_order_by_basic_fields_asc() {
    let id_sort = order_by("id").expect("valid sort");
    assert!(id_sort.contains("(d.id) IS NULL"));
    assert!(id_sort.contains("d.id ASC"));

    let title_sort = order_by("title").expect("valid sort");
    assert!(title_sort.contains("LOWER(d.title) ASC"));

    let created_sort = order_by("created").expect("valid sort");
    assert!(created_sort.contains("d.created ASC"));

    let modified_sort = order_by("modified").expect("valid sort");
    assert!(modified_sort.contains("d.modified ASC"));

    let added_sort = order_by("added").expect("valid sort");
    assert!(added_sort.contains("d.added ASC"));

    let asn_sort = order_by("archive_serial_number").expect("valid sort");
    assert!(asn_sort.contains("d.archive_serial_number ASC"));

    let page_sort = order_by("page_count").expect("valid sort");
    assert!(page_sort.contains("d.page_count ASC"));
}

#[test]
fn test_order_by_basic_fields_desc() {
    let id_desc = order_by("-id").expect("valid desc sort");
    assert!(id_desc.contains("d.id DESC"));

    let title_desc = order_by("-title").expect("valid desc sort");
    assert!(title_desc.contains("LOWER(d.title) DESC"));

    let created_desc = order_by("-created").expect("valid desc sort");
    assert!(created_desc.contains("d.created DESC"));

    let asn_desc = order_by("-archive_serial_number").expect("valid desc sort");
    assert!(asn_desc.contains("d.archive_serial_number DESC"));
}

#[test]
fn test_order_by_relational_subqueries() {
    // Correspondent sort
    let corr_asc = order_by("correspondent__name").expect("valid sort");
    assert!(corr_asc.contains("SELECT LOWER(name) FROM correspondents WHERE id = d.correspondent_id"));
    assert!(corr_asc.contains("ASC"));

    let corr_desc = order_by("-correspondent__name").expect("valid sort");
    assert!(corr_desc.contains("SELECT LOWER(name) FROM correspondents WHERE id = d.correspondent_id"));
    assert!(corr_desc.contains("DESC"));

    // Document type sort
    let dt_asc = order_by("document_type__name").expect("valid sort");
    assert!(dt_asc.contains("SELECT LOWER(name) FROM document_types WHERE id = d.document_type_id"));

    // Storage path sort
    let sp_asc = order_by("storage_path__name").expect("valid sort");
    assert!(sp_asc.contains("SELECT LOWER(name) FROM storage_paths WHERE id = d.storage_path_id"));

    // Number of notes sort
    let notes_sort = order_by("-num_notes").expect("valid sort");
    assert!(notes_sort.contains("SELECT COUNT(*) FROM notes n WHERE n.document_id = d.id AND n.deleted_at IS NULL"));
    assert!(notes_sort.contains("DESC"));
}

#[test]
fn test_order_by_custom_fields() {
    let cf_sort = order_by("custom_field_15").expect("valid cf sort");
    assert!(cf_sort.contains("field_id = 15"));
    assert!(cf_sort.contains("COALESCE"));
    assert!(cf_sort.contains("i.value_text"));
    assert!(cf_sort.contains("ASC"));

    let cf_desc = order_by("-custom_field_999").expect("valid desc cf sort");
    assert!(cf_desc.contains("field_id = 999"));
    assert!(cf_desc.contains("DESC"));

    // Non-numeric custom field suffix should be rejected
    assert!(order_by("custom_field_abc").is_none());
    assert!(order_by("custom_field_").is_none());
}

#[test]
fn test_order_by_compound_keys() {
    let multi = order_by("-created,title,-id").expect("compound sort");
    let parts: Vec<&str> = multi.split(", ").collect();
    
    // Each field produces 2 segments: "(expr) IS NULL" and "expr [ASC|DESC]"
    assert_eq!(parts.len(), 6);
    assert!(parts[1].contains("d.created DESC"));
    assert!(parts[3].contains("LOWER(d.title) ASC"));
    assert!(parts[5].contains("d.id DESC"));
}

#[test]
fn test_order_by_rejections_and_sanitization() {
    // Malicious or arbitrary SQL injections must return None
    assert!(order_by("id; DROP TABLE documents;--").is_none());
    assert!(order_by("title OR 1=1").is_none());
    assert!(order_by("SELECT * FROM passwords").is_none());
    assert!(order_by("unknown_column").is_none());
    assert!(order_by("password").is_none());

    // Empty and whitespace strings
    assert!(order_by("").is_none());
    assert!(order_by("   ").is_none());
    assert!(order_by(",,").is_none());

    // Ignored empty items in compound list
    let leading_comma = order_by(",title").expect("handles leading comma");
    assert!(leading_comma.contains("LOWER(d.title) ASC"));

    let trailing_comma = order_by("title,").expect("handles trailing comma");
    assert!(trailing_comma.contains("LOWER(d.title) ASC"));
}

#[test]
fn test_params_get_lookup() {
    let params: &Params = &[
        ("page".to_string(), "1".to_string()),
        ("page_size".to_string(), "50".to_string()),
        ("query".to_string(), "tax 2024".to_string()),
        ("tag_id".to_string(), "4".to_string()),
        ("tag_id".to_string(), "7".to_string()), // Last value wins
    ];

    assert_eq!(get(&params, "page"), Some("1"));
    assert_eq!(get(&params, "page_size"), Some("50"));
    assert_eq!(get(&params, "query"), Some("tax 2024"));
    assert_eq!(get(&params, "nonexistent"), None);

    // Verifies last occurrence precedence
    assert_eq!(get(&params, "tag_id"), Some("7"));
}

#[test]
fn test_search_mode_semantics() {
    let mode_text = SearchMode::Text;
    let mode_title = SearchMode::Title;
    let mode_query = SearchMode::Query;

    assert_eq!(mode_text, SearchMode::Text);
    assert_ne!(mode_text, mode_title);
    assert_ne!(mode_title, mode_query);

    // Test Copy and Clone implementations
    let copied = mode_query;
    assert_eq!(copied, SearchMode::Query);
}
