pub mod ai;
pub mod correspondents;
pub mod crud_extra;
pub mod custom_fields;
pub mod document_types;
pub mod documents;
pub mod documents_ops;
pub mod mail;
pub mod pdf_studio;
pub mod saved_views;
pub mod schema;
pub mod search;
pub mod settings;
pub mod share_links;
pub mod storage_paths;
pub mod tags;
pub mod tasks;
pub mod workflows;
pub mod ws;

use crate::state::AppState;
use axum::{
    routing::{delete, get, patch, post},
    Router,
};

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // WebSocket
        .route("/ws/status/", get(ws::ws_status_handler))
        // Settings & System
        .route(
            "/api/ui_settings/",
            get(settings::get_ui_settings).patch(settings::update_ui_settings),
        )
        .route(
            "/api/config/",
            get(settings::get_config).patch(settings::update_config),
        )
        .route("/api/status/", get(settings::get_status))
        .route("/api/statistics/", get(settings::get_statistics))
        .route("/api/remote_version/", get(settings::get_remote_version))
        .route("/api/logs/", get(settings::get_logs))
        .route("/api/sanity_check/", get(settings::sanity_check))
        // Correspondents
        .route(
            "/api/correspondents/",
            get(correspondents::list_correspondents).post(correspondents::create_correspondent),
        )
        .route(
            "/api/correspondents/:id/",
            get(correspondents::get_correspondent)
                .put(correspondents::update_correspondent)
                .patch(correspondents::update_correspondent)
                .delete(correspondents::delete_correspondent),
        )
        // Document Types
        .route(
            "/api/document_types/",
            get(document_types::list_document_types).post(document_types::create_document_type),
        )
        .route(
            "/api/document_types/:id/",
            get(document_types::get_document_type)
                .put(document_types::update_document_type)
                .patch(document_types::update_document_type)
                .delete(document_types::delete_document_type),
        )
        // Tags
        .route("/api/tags/", get(tags::list_tags).post(tags::create_tag))
        .route(
            "/api/tags/:id/",
            get(tags::get_tag)
                .put(tags::update_tag)
                .patch(tags::update_tag)
                .delete(tags::delete_tag),
        )
        // Storage Paths
        .route(
            "/api/storage_paths/",
            get(storage_paths::list_storage_paths).post(storage_paths::create_storage_path),
        )
        .route(
            "/api/storage_paths/:id/",
            get(storage_paths::get_storage_path)
                .put(storage_paths::update_storage_path)
                .patch(storage_paths::update_storage_path)
                .delete(storage_paths::delete_storage_path),
        )
        // Custom Fields
        .route(
            "/api/custom_fields/",
            get(custom_fields::list_custom_fields).post(custom_fields::create_custom_field),
        )
        .route(
            "/api/custom_fields/:id/",
            get(custom_fields::get_custom_field)
                .put(custom_fields::update_custom_field)
                .patch(custom_fields::update_custom_field)
                .delete(custom_fields::delete_custom_field),
        )
        // Saved Views
        .route(
            "/api/saved_views/",
            get(saved_views::list_saved_views).post(saved_views::create_saved_view),
        )
        .route(
            "/api/saved_views/:id/",
            get(saved_views::get_saved_view)
                .put(saved_views::update_saved_view)
                .patch(saved_views::update_saved_view)
                .delete(saved_views::delete_saved_view),
        )
        // Tasks
        .route("/api/tasks/", get(tasks::list_tasks))
        .route("/api/tasks/acknowledge/", post(tasks::acknowledge_tasks))
        // Share Links & Bundles
        .route(
            "/api/share_links/",
            get(share_links::list_share_links).post(share_links::create_share_link),
        )
        .route(
            "/api/share_links/:id/",
            delete(share_links::delete_share_link),
        )
        .route(
            "/api/share_link_bundles/",
            get(share_links::list_share_link_bundles).post(share_links::create_share_link_bundle),
        )
        .route(
            "/api/share_link_bundles/:id/",
            delete(share_links::delete_share_link_bundle),
        )
        .route("/share/:slug/", get(share_links::view_shared_link))
        // Workflows
        .route(
            "/api/workflows/",
            get(workflows::list_workflows).post(workflows::create_workflow),
        )
        .route(
            "/api/workflows/:id/",
            get(workflows::get_workflow)
                .put(workflows::update_workflow)
                .patch(workflows::update_workflow)
                .delete(workflows::delete_workflow),
        )
        .route(
            "/api/workflow_triggers/",
            get(workflows::list_workflow_triggers).post(workflows::create_workflow_trigger),
        )
        .route(
            "/api/workflow_triggers/:id/",
            delete(workflows::delete_workflow_trigger),
        )
        .route(
            "/api/workflow_actions/",
            get(workflows::list_workflow_actions).post(workflows::create_workflow_action),
        )
        .route(
            "/api/workflow_actions/:id/",
            delete(workflows::delete_workflow_action),
        )
        // Mail
        .route(
            "/api/mail_accounts/",
            get(mail::list_mail_accounts).post(mail::create_mail_account),
        )
        .route(
            "/api/mail_accounts/:id/",
            patch(mail::update_mail_account).delete(mail::delete_mail_account),
        )
        .route("/api/mail_accounts/test/", post(mail::test_mail_account))
        .route(
            "/api/mail_accounts/:id/test/",
            post(mail::test_mail_account_by_id),
        )
        .route(
            "/api/mail_accounts/:id/process/",
            post(mail::process_mail_account),
        )
        .route(
            "/api/mail_rules/",
            get(mail::list_mail_rules).post(mail::create_mail_rule),
        )
        .route(
            "/api/mail_rules/:id/",
            patch(mail::update_mail_rule).delete(mail::delete_mail_rule),
        )
        .route(
            "/api/mail_rules/:id/process/",
            post(mail::process_mail_rule),
        )
        .route("/api/processed_mail/", get(mail::list_processed_mail))
        .route("/api/oauth/callback/", post(mail::oauth_callback))
        // Documents
        .route("/api/documents/", get(documents::list_documents))
        .route(
            "/api/documents/post_document/",
            post(documents::post_document),
        )
        .route(
            "/api/documents/:id/",
            get(documents::get_document)
                .put(documents::update_document)
                .patch(documents::update_document)
                .delete(documents::delete_document),
        )
        .route(
            "/api/documents/:id/download/",
            get(documents::download_document),
        )
        .route(
            "/api/documents/:id/preview/",
            get(documents::preview_document),
        )
        .route(
            "/api/documents/:id/thumb/",
            get(documents::thumbnail_document),
        )
        .route(
            "/api/documents/:id/notes/",
            get(documents::list_notes).post(documents::create_note),
        )
        .route(
            "/api/documents/:id/notes/:note_id/",
            delete(documents::delete_note),
        )
        .route("/api/documents/bulk_edit/", post(documents::bulk_edit))
        .route(
            "/api/documents/bulk_download/",
            post(documents::bulk_download),
        )
        .route(
            "/api/documents/selection_data/",
            get(documents::get_selection_data),
        )
        .route(
            "/api/documents/:id/history/",
            get(documents::get_document_history),
        )
        .route("/api/schema/", get(schema::get_schema))
        .route("/api/search/", get(search::global_search))
        .route("/api/search/autocomplete/", get(search::autocomplete))
        .route("/api/search/reindex/", post(search::reindex))
        .route("/api/documents/:id/chat/", post(ai::document_chat))
        .route("/api/documents/:id/tables/", get(ai::get_document_tables))
        .route("/api/chat/", post(ai::global_chat))
        .route("/api/ai/models/status/", get(ai::get_model_status))
        .route("/api/ai/models/download/", post(ai::download_model))
        .route("/api/ai/models/cancel/", post(ai::cancel_model_download))
        .route("/api/ai/models/", delete(ai::delete_model))
        .merge(documents_ops::router())
        .merge(crud_extra::router())
        .merge(pdf_studio::router())
        .with_state(state)
}
