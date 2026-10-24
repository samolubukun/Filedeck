use axum::{response::IntoResponse, Json};
use serde_json::json;

/// `GET /api/schema/` — OpenAPI 3.0 specification for FileDeck.
pub async fn get_schema() -> impl IntoResponse {
    Json(json!({
        "openapi": "3.0.3",
        "info": {
            "title": "FileDeck API",
            "version": "1.0.0",
            "description": "FileDeck local-first document management API"
        },
        "paths": {
            "/api/documents/": {
                "get": {
                    "summary": "List documents",
                    "operationId": "documents_list",
                    "parameters": [
                        {"name": "page", "in": "query", "schema": {"type": "integer"}},
                        {"name": "page_size", "in": "query", "schema": {"type": "integer"}},
                        {"name": "query", "in": "query", "schema": {"type": "string"}},
                        {"name": "ordering", "in": "query", "schema": {"type": "string"}}
                    ],
                    "responses": {"200": {"description": "Document list"}}
                }
            },
            "/api/documents/post_document/": {
                "post": {
                    "summary": "Upload and consume a document",
                    "operationId": "documents_post_document",
                    "responses": {"200": {"description": "Task ID queued"}}
                }
            },
            "/api/documents/{id}/": {
                "get": {"summary": "Retrieve document", "operationId": "documents_retrieve", "responses": {"200": {"description": "Document details"}}},
                "put": {"summary": "Update document", "operationId": "documents_update", "responses": {"200": {"description": "Document updated"}}},
                "patch": {"summary": "Partial update document", "operationId": "documents_partial_update", "responses": {"200": {"description": "Document updated"}}},
                "delete": {"summary": "Delete document", "operationId": "documents_destroy", "responses": {"204": {"description": "Moved to trash"}}}
            },
            "/api/documents/{id}/history/": {
                "get": {"summary": "Document audit history", "operationId": "documents_history", "responses": {"200": {"description": "Audit history list"}}}
            },
            "/api/documents/{id}/update_version/": {
                "post": {"summary": "Upload a new version of a document", "operationId": "documents_update_version", "responses": {"200": {"description": "Task ID queued"}}}
            },
            "/api/documents/{id}/update_version_label/": {
                "patch": {"summary": "Update version label", "operationId": "documents_update_version_label", "responses": {"200": {"description": "OK"}}}
            },
            "/api/documents/merge_as_versions/": {
                "post": {"summary": "Merge documents as versions of a root document", "operationId": "documents_merge_as_versions", "responses": {"200": {"description": "OK"}}}
            },
            "/api/documents/bulk_edit/": {
                "post": {"summary": "Bulk edit documents", "operationId": "documents_bulk_edit", "responses": {"200": {"description": "OK"}}}
            },
            "/api/tags/": {
                "get": {"summary": "List tags", "operationId": "tags_list", "responses": {"200": {"description": "Tag list"}}},
                "post": {"summary": "Create tag", "operationId": "tags_create", "responses": {"201": {"description": "Tag created"}}}
            },
            "/api/correspondents/": {
                "get": {"summary": "List correspondents", "operationId": "correspondents_list", "responses": {"200": {"description": "Correspondent list"}}},
                "post": {"summary": "Create correspondent", "operationId": "correspondents_create", "responses": {"201": {"description": "Correspondent created"}}}
            },
            "/api/document_types/": {
                "get": {"summary": "List document types", "operationId": "document_types_list", "responses": {"200": {"description": "Document type list"}}},
                "post": {"summary": "Create document type", "operationId": "document_types_create", "responses": {"201": {"description": "Document type created"}}}
            },
            "/api/storage_paths/": {
                "get": {"summary": "List storage paths", "operationId": "storage_paths_list", "responses": {"200": {"description": "Storage path list"}}},
                "post": {"summary": "Create storage path", "operationId": "storage_paths_create", "responses": {"201": {"description": "Storage path created"}}}
            },
            "/api/custom_fields/": {
                "get": {"summary": "List custom fields", "operationId": "custom_fields_list", "responses": {"200": {"description": "Custom field list"}}},
                "post": {"summary": "Create custom field", "operationId": "custom_fields_create", "responses": {"201": {"description": "Custom field created"}}}
            },
            "/api/saved_views/": {
                "get": {"summary": "List saved views", "operationId": "saved_views_list", "responses": {"200": {"description": "Saved view list"}}},
                "post": {"summary": "Create saved view", "operationId": "saved_views_create", "responses": {"201": {"description": "Saved view created"}}}
            },
            "/api/tasks/": {
                "get": {"summary": "List background tasks", "operationId": "tasks_list", "responses": {"200": {"description": "Task list"}}}
            },
            "/api/trash/": {
                "get": {"summary": "List trashed documents", "operationId": "trash_list", "responses": {"200": {"description": "Trash list"}}},
                "post": {"summary": "Restore or empty trash", "operationId": "trash_action", "responses": {"200": {"description": "OK"}}}
            },
            "/api/ui_settings/": {
                "get": {"summary": "Get UI settings", "operationId": "ui_settings_get", "responses": {"200": {"description": "UI settings"}}},
                "patch": {"summary": "Update UI settings", "operationId": "ui_settings_update", "responses": {"200": {"description": "UI settings updated"}}}
            },
            "/api/search/": {
                "get": {"summary": "Global search", "operationId": "search_global", "responses": {"200": {"description": "Search results"}}}
            },
            "/api/search/autocomplete/": {
                "get": {"summary": "Search autocomplete", "operationId": "search_autocomplete", "responses": {"200": {"description": "Autocomplete terms"}}}
            },
            "/api/documents/{id}/chat/": {
                "post": {"summary": "Chat with a document via SSE stream or JSON", "operationId": "documents_chat", "responses": {"200": {"description": "Streaming or JSON answer"}}}
            },
            "/api/documents/{id}/tables/": {
                "get": {"summary": "Extract structured tables from document PDF", "operationId": "documents_tables", "responses": {"200": {"description": "Extracted table list"}}}
            },
            "/api/chat/": {
                "post": {"summary": "Global cross-document semantic chat", "operationId": "global_chat", "responses": {"200": {"description": "Streaming or JSON answer"}}}
            }
        }
    }))
}
