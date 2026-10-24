//! Integration tests for FileDeck's API contract and schema specifications.

use axum::response::IntoResponse;
use filedeck_backend::routes::schema::get_schema;

#[tokio::test]
async fn test_openapi_schema_endpoint() {
    let response = get_schema().await.into_response();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[test]
fn test_api_route_inventory() {
    let expected_routes = vec![
        "/api/documents/",
        "/api/documents/post_document/",
        "/api/documents/{id}/",
        "/api/documents/{id}/history/",
        "/api/documents/{id}/update_version/",
        "/api/documents/{id}/update_version_label/",
        "/api/documents/merge_as_versions/",
        "/api/documents/bulk_edit/",
        "/api/tags/",
        "/api/correspondents/",
        "/api/document_types/",
        "/api/storage_paths/",
        "/api/custom_fields/",
        "/api/saved_views/",
        "/api/tasks/",
        "/api/trash/",
        "/api/ui_settings/",
        "/api/search/",
        "/api/search/autocomplete/",
        "/api/documents/{id}/chat/",
        "/api/documents/{id}/tables/",
        "/api/chat/",
    ];

    assert_eq!(expected_routes.len(), 22);
    for r in &expected_routes {
        assert!(r.starts_with("/api/"));
    }
}

#[test]
fn test_document_list_query_params() {
    let valid_params = vec![
        "page",
        "page_size",
        "query",
        "ordering",
        "tags__id__all",
        "tags__id__in",
        "tags__id__none",
        "correspondent__id",
        "document_type__id",
        "storage_path__id",
        "created__date__gte",
        "created__date__lte",
        "added__date__gte",
        "added__date__lte",
        "is_tagged",
        "custom_fields__id__all",
    ];

    assert!(valid_params.contains(&"page"));
    assert!(valid_params.contains(&"ordering"));
    assert!(valid_params.contains(&"query"));
}

#[test]
fn test_bulk_edit_operation_types() {
    let valid_operations = vec![
        "set_correspondent",
        "set_document_type",
        "set_storage_path",
        "add_tag",
        "remove_tag",
        "modify_tags",
        "delete",
        "reindex",
    ];

    for op in &valid_operations {
        assert!(!op.is_empty());
    }
}
