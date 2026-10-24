//! Workflows execution engine.
//!
//! Evaluates triggers when documents are added (type 2) or updated (type 3),
//! matches criteria (sources, path, filename, tags, correspondents, types),
//! and applies configured actions (assignments, removals, webhooks).

use crate::error::AppError;
use crate::state::AppState;
use sqlx::Row;
use tracing::info;

pub const TRIGGER_CONSUMPTION: i64 = 1;
pub const TRIGGER_DOCUMENT_ADDED: i64 = 2;
pub const TRIGGER_DOCUMENT_UPDATED: i64 = 3;

fn parse_ids(raw: Option<String>) -> Vec<i64> {
    raw.and_then(|s| serde_json::from_str::<Vec<i64>>(&s).ok())
        .unwrap_or_default()
}

/// Run matching workflows against a document.
pub async fn run_workflows(
    state: &AppState,
    document_id: i64,
    trigger_type: i64,
    source: Option<&str>,
) -> Result<(), AppError> {
    // 1. Fetch document metadata
    let doc_row = sqlx::query(
        "SELECT id, title, correspondent_id, document_type_id, storage_path_id,
                original_filename, created
         FROM documents WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(document_id)
    .fetch_optional(&state.db)
    .await?;

    let Some(doc) = doc_row else {
        return Ok(());
    };

    let filename: String = doc
        .get::<Option<String>, _>("original_filename")
        .unwrap_or_default();
    let correspondent_id: Option<i64> = doc.get("correspondent_id");
    let document_type_id: Option<i64> = doc.get("document_type_id");
    let storage_path_id: Option<i64> = doc.get("storage_path_id");

    let current_tags: Vec<i64> =
        sqlx::query_scalar("SELECT tag_id FROM document_tags WHERE document_id = $1")
            .bind(document_id)
            .fetch_all(&state.db)
            .await?;

    // 2. Query enabled workflows in order
    let workflows = sqlx::query(
        r#"SELECT id, name FROM workflows WHERE enabled = 1 ORDER BY "order" ASC, id ASC"#,
    )
    .fetch_all(&state.db)
    .await?;

    let mut doc_modified = false;

    for wf in workflows {
        let wf_id: i64 = wf.get("id");
        let wf_name: String = wf.get("name");

        // Query triggers for this workflow that match the trigger_type
        let triggers = sqlx::query(
            "SELECT wt.id, wt.sources, wt.filter_path, wt.filter_filename,
                    wt.filter_has_tags, wt.filter_has_all_tags, wt.filter_has_not_tags,
                    wt.filter_has_correspondent_id, wt.filter_has_document_type_id,
                    wt.filter_has_storage_path_id
             FROM workflow_triggers wt
             JOIN workflow_workflow_triggers wwt ON wt.id = wwt.trigger_id
             WHERE wwt.workflow_id = $1 AND wt.type = $2",
        )
        .bind(wf_id)
        .bind(trigger_type)
        .fetch_all(&state.db)
        .await?;

        if triggers.is_empty() {
            continue;
        }

        // Check if ANY trigger in the workflow matches
        let mut trigger_matched = false;
        for tr in &triggers {
            // Source filter
            if let Some(src) = source {
                let sources_str: Option<String> = tr.get("sources");
                if let Some(s) = sources_str {
                    let allowed_sources: Vec<String> = s
                        .split(',')
                        .map(|x| x.trim().to_ascii_lowercase())
                        .collect();
                    if !allowed_sources.is_empty()
                        && !allowed_sources.contains(&src.to_ascii_lowercase())
                    {
                        continue;
                    }
                }
            }

            // Filename / path pattern filter
            let filter_fn: Option<String> = tr.get("filter_filename");
            if let Some(pat) = filter_fn {
                if !pat.is_empty() {
                    let wildcard = pat.replace('*', ".*");
                    if let Ok(re) = regex::RegexBuilder::new(&wildcard)
                        .case_insensitive(true)
                        .build()
                    {
                        if !re.is_match(&filename) {
                            continue;
                        }
                    }
                }
            }

            // Correspondent filter
            let req_corr: Option<i64> = tr.get("filter_has_correspondent_id");
            if let Some(c) = req_corr {
                if correspondent_id != Some(c) {
                    continue;
                }
            }

            // Document type filter
            let req_type: Option<i64> = tr.get("filter_has_document_type_id");
            if let Some(t) = req_type {
                if document_type_id != Some(t) {
                    continue;
                }
            }

            // Storage path filter
            let req_path: Option<i64> = tr.get("filter_has_storage_path_id");
            if let Some(p) = req_path {
                if storage_path_id != Some(p) {
                    continue;
                }
            }

            // Tag inclusion filters
            let has_tags = parse_ids(tr.get("filter_has_tags"));
            if !has_tags.is_empty() && !has_tags.iter().any(|t| current_tags.contains(t)) {
                continue;
            }

            let has_all_tags = parse_ids(tr.get("filter_has_all_tags"));
            if !has_all_tags.is_empty() && !has_all_tags.iter().all(|t| current_tags.contains(t)) {
                continue;
            }

            let has_not_tags = parse_ids(tr.get("filter_has_not_tags"));
            if !has_not_tags.is_empty() && has_not_tags.iter().any(|t| current_tags.contains(t)) {
                continue;
            }

            trigger_matched = true;
            break;
        }

        if !trigger_matched {
            continue;
        }

        info!("Executing workflow '{}' (id={}) for document {}", wf_name, wf_id, document_id);

        // Fetch actions for this workflow
        let actions = sqlx::query(
            "SELECT wa.*
             FROM workflow_actions wa
             JOIN workflow_workflow_actions wwa ON wa.id = wwa.action_id
             WHERE wwa.workflow_id = $1
             ORDER BY wa.id ASC",
        )
        .bind(wf_id)
        .fetch_all(&state.db)
        .await?;

        for act in actions {
            let act_type: i64 = act.get("type");

            match act_type {
                1 => {
                    // Type 1: Assignment
                    let assign_corr: Option<i64> = act.get("assign_correspondent_id");
                    if assign_corr.is_some() {
                        sqlx::query("UPDATE documents SET correspondent_id = $1 WHERE id = $2")
                            .bind(assign_corr)
                            .bind(document_id)
                            .execute(&state.db)
                            .await?;
                        doc_modified = true;
                    }

                    let assign_type: Option<i64> = act.get("assign_document_type_id");
                    if assign_type.is_some() {
                        sqlx::query("UPDATE documents SET document_type_id = $1 WHERE id = $2")
                            .bind(assign_type)
                            .bind(document_id)
                            .execute(&state.db)
                            .await?;
                        doc_modified = true;
                    }

                    let assign_path: Option<i64> = act.get("assign_storage_path_id");
                    if assign_path.is_some() {
                        sqlx::query("UPDATE documents SET storage_path_id = $1 WHERE id = $2")
                            .bind(assign_path)
                            .bind(document_id)
                            .execute(&state.db)
                            .await?;
                        doc_modified = true;
                    }

                    let add_tags = parse_ids(act.get("assign_tags"));
                    for tag_id in add_tags {
                        sqlx::query(
                            "INSERT OR IGNORE INTO document_tags (document_id, tag_id) VALUES ($1, $2)",
                        )
                        .bind(document_id)
                        .bind(tag_id)
                        .execute(&state.db)
                        .await?;
                        doc_modified = true;
                    }

                    if let Some(title_template) = act.get::<Option<String>, _>("assign_title") {
                        if !title_template.trim().is_empty() {
                            sqlx::query("UPDATE documents SET title = $1 WHERE id = $2")
                                .bind(&title_template)
                                .bind(document_id)
                                .execute(&state.db)
                                .await?;
                            doc_modified = true;
                        }
                    }
                }
                2 => {
                    // Type 2: Removal
                    let remove_all_tags: bool = act.get("remove_all_tags");
                    if remove_all_tags {
                        sqlx::query("DELETE FROM document_tags WHERE document_id = $1")
                            .bind(document_id)
                            .execute(&state.db)
                            .await?;
                        doc_modified = true;
                    } else {
                        let rm_tags = parse_ids(act.get("remove_tags"));
                        for tag_id in rm_tags {
                            sqlx::query(
                                "DELETE FROM document_tags WHERE document_id = $1 AND tag_id = $2",
                            )
                            .bind(document_id)
                            .bind(tag_id)
                            .execute(&state.db)
                            .await?;
                            doc_modified = true;
                        }
                    }

                    let remove_all_corr: bool = act.get("remove_all_correspondents");
                    if remove_all_corr {
                        sqlx::query("UPDATE documents SET correspondent_id = NULL WHERE id = $1")
                            .bind(document_id)
                            .execute(&state.db)
                            .await?;
                        doc_modified = true;
                    }

                    let remove_all_types: bool = act.get("remove_all_document_types");
                    if remove_all_types {
                        sqlx::query("UPDATE documents SET document_type_id = NULL WHERE id = $1")
                            .bind(document_id)
                            .execute(&state.db)
                            .await?;
                        doc_modified = true;
                    }

                    let remove_all_paths: bool = act.get("remove_all_storage_paths");
                    if remove_all_paths {
                        sqlx::query("UPDATE documents SET storage_path_id = NULL WHERE id = $1")
                            .bind(document_id)
                            .execute(&state.db)
                            .await?;
                        doc_modified = true;
                    }
                }
                4 => {
                    // Type 4: Webhook
                    if let Some(url) = act.get::<Option<String>, _>("webhook_url") {
                        if !url.trim().is_empty() {
                            let client = reqwest::Client::new();
                            let payload = serde_json::json!({
                                "document_id": document_id,
                                "workflow_id": wf_id,
                                "event": if trigger_type == TRIGGER_DOCUMENT_ADDED { "document_added" } else { "document_updated" },
                            });
                            let url_clone = url.clone();
                            tokio::spawn(async move {
                                let _ = client.post(&url_clone).json(&payload).send().await;
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        // Record execution in workflow_runs
        let _ = sqlx::query(
            "INSERT INTO workflow_runs (workflow_id, document_id, type, status)
             VALUES ($1, $2, $3, 'SUCCESS')",
        )
        .bind(wf_id)
        .bind(document_id)
        .bind(trigger_type)
        .execute(&state.db)
        .await;
    }

    if doc_modified {
        let _ = crate::search::reindex_documents(state, &[document_id]).await;
    }

    Ok(())
}
