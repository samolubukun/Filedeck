use axum::{
    extract::State,
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Serialize, Deserialize)]
#[allow(dead_code)]
pub struct UiSettingsPayload {
    pub settings: Value,
}

pub async fn get_ui_settings(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let row = sqlx::query("SELECT settings FROM ui_settings WHERE id = 1")
        .fetch_optional(&state.db)
        .await?;

    let settings = match row {
        Some(r) => {
            let s: String = r.get("settings");
            serde_json::from_str(&s).unwrap_or(json!({}))
        }
        None => json!({}),
    };

    Ok(Json(json!({
        "id": 1,
        "settings": settings
    })))
}

pub async fn update_ui_settings(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let raw = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    sqlx::query(
        "INSERT INTO ui_settings (id, settings) VALUES (1, $1)
         ON CONFLICT(id) DO UPDATE SET settings = excluded.settings"
    )
    .bind(raw)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({
        "id": 1,
        "settings": payload
    })))
}

#[derive(Serialize, Deserialize)]
pub struct AppConfigPayload {
    pub app_title: Option<String>,
    pub app_logo: Option<String>,
    pub output_type: Option<String>,
    pub language: Option<String>,
    pub mode: Option<String>,
}

pub async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let row = sqlx::query("SELECT app_title, app_logo, output_type, language, mode FROM app_config WHERE id = 1")
        .fetch_optional(&state.db)
        .await?;

    if let Some(r) = row {
        let app_title: String = r.get("app_title");
        let app_logo: Option<String> = r.get("app_logo");
        let output_type: String = r.get("output_type");
        let language: String = r.get("language");
        let mode: String = r.get("mode");

        Ok(Json(json!({
            "id": 1,
            "app_title": app_title,
            "app_logo": app_logo,
            "output_type": output_type,
            "language": language,
            "mode": mode
        })))
    } else {
        Ok(Json(json!({
            "id": 1,
            "app_title": "FileDeck",
            "app_logo": null,
            "output_type": "pdfa",
            "language": "eng",
            "mode": "skip_noarchive"
        })))
    }
}

pub async fn update_config(
    State(state): State<AppState>,
    Json(payload): Json<AppConfigPayload>,
) -> Result<Json<Value>, AppError> {
    let current = get_config(State(state.clone())).await?.0;

    let app_title = payload.app_title.unwrap_or_else(|| current["app_title"].as_str().unwrap_or("FileDeck").to_string());
    let app_logo = payload.app_logo.or_else(|| current["app_logo"].as_str().map(|s| s.to_string()));
    let output_type = payload.output_type.unwrap_or_else(|| current["output_type"].as_str().unwrap_or("pdfa").to_string());
    let language = payload.language.unwrap_or_else(|| current["language"].as_str().unwrap_or("eng").to_string());
    let mode = payload.mode.unwrap_or_else(|| current["mode"].as_str().unwrap_or("skip_noarchive").to_string());

    sqlx::query(
        "INSERT INTO app_config (id, app_title, app_logo, output_type, language, mode)
         VALUES (1, $1, $2, $3, $4, $5)
         ON CONFLICT(id) DO UPDATE SET
            app_title = excluded.app_title,
            app_logo = excluded.app_logo,
            output_type = excluded.output_type,
            language = excluded.language,
            mode = excluded.mode"
    )
    .bind(&app_title)
    .bind(&app_logo)
    .bind(&output_type)
    .bind(&language)
    .bind(&mode)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({
        "id": 1,
        "app_title": app_title,
        "app_logo": app_logo,
        "output_type": output_type,
        "language": language,
        "mode": mode
    })))
}

pub async fn get_status(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let worker_healthy = state.worker_client
        .get(format!("{}/health", state.config.worker.url))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    Ok(Json(json!({
        "status": "OK",
        "system": {
            "database": "sqlite",
            "storage_media": state.config.storage.media_root.exists(),
            "consume_dir": state.config.storage.consume_dir.exists(),
            "index_dir": state.config.storage.index_dir.exists(),
            "worker_connected": worker_healthy
        }
    })))
}

pub async fn get_statistics(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let doc_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents WHERE deleted_at IS NULL")
        .fetch_one(&state.db)
        .await?;

    let doc_inbox_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT d.id) FROM documents d
         JOIN document_tags dt ON d.id = dt.document_id
         JOIN tags t ON dt.tag_id = t.id
         WHERE t.is_inbox_tag = 1 AND d.deleted_at IS NULL"
    )
    .fetch_one(&state.db)
    .await?;

    let char_count: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(content_length), 0) FROM documents WHERE deleted_at IS NULL")
        .fetch_one(&state.db)
        .await?;

    let tag_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tags")
        .fetch_one(&state.db)
        .await?;

    let correspondent_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM correspondents")
        .fetch_one(&state.db)
        .await?;

    let document_type_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM document_types")
        .fetch_one(&state.db)
        .await?;

    let storage_path_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM storage_paths")
        .fetch_one(&state.db)
        .await?;

    let mime_rows = sqlx::query(
        "SELECT COALESCE(mime_type, 'application/pdf') AS mime, COUNT(*) AS count
         FROM documents WHERE deleted_at IS NULL GROUP BY mime"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let file_types: Vec<Value> = mime_rows
        .into_iter()
        .map(|r| {
            let mime: String = r.get("mime");
            let count: i64 = r.get("count");
            json!({
                "mime_type": mime,
                "count": count
            })
        })
        .collect();

    Ok(Json(json!({
        "documents_total": doc_count,
        "documents_inbox": doc_inbox_count,
        "character_count": char_count,
        "tags_total": tag_count,
        "correspondents_total": correspondent_count,
        "document_types_total": document_type_count,
        "storage_paths_total": storage_path_count,
        "document_file_type_counts": file_types
    })))
}

pub async fn get_remote_version() -> Json<Value> {
    Json(json!({
        "version": "1.0.0",
        "update_available": false
    }))
}

pub async fn get_logs() -> Json<Value> {
    Json(json!([
        {
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "level": "INFO",
            "message": "FileDeck service online"
        }
    ]))
}

pub async fn sanity_check(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let docs = sqlx::query(
        "SELECT id, title, filename, archive_filename, checksum, archive_checksum FROM documents WHERE deleted_at IS NULL"
    )
    .fetch_all(&state.db)
    .await?;

    let mut messages = Vec::new();
    let mut known_originals = std::collections::HashSet::new();
    let mut known_archives = std::collections::HashSet::new();
    let mut checked_files = 0usize;

    let media_root = &state.config.storage.media_root;
    let originals_dir = media_root.join("originals");
    let archive_dir = media_root.join("archive");

    for row in &docs {
        let id: i64 = row.get("id");
        let title: String = row.get("title");
        let filename: Option<String> = row.get("filename");
        let archive_filename: Option<String> = row.get("archive_filename");
        let db_checksum: Option<String> = row.get("checksum");
        let db_archive_checksum: Option<String> = row.get("archive_checksum");

        // Check original file
        if let Some(ref fname) = filename {
            known_originals.insert(fname.clone());
            let path = originals_dir.join(fname);
            if !path.exists() {
                messages.push(json!({
                    "doc_id": id,
                    "type": "error",
                    "message": format!("Document #{id} ('{title}'): Original file not found on disk at {}", path.display())
                }));
            } else {
                checked_files += 1;
                if let Some(ref expected_hash) = db_checksum {
                    match tokio::fs::read(&path).await {
                        Ok(bytes) => {
                            let actual_hash = crate::ingest::sha256_hex(&bytes);
                            if !actual_hash.eq_ignore_ascii_case(expected_hash) {
                                messages.push(json!({
                                    "doc_id": id,
                                    "type": "error",
                                    "message": format!("Document #{id} ('{title}'): Checksum mismatch! DB: {expected_hash}, Disk: {actual_hash}")
                                }));
                            }
                        }
                        Err(e) => {
                            messages.push(json!({
                                "doc_id": id,
                                "type": "error",
                                "message": format!("Document #{id} ('{title}'): Could not read original file {}: {e}", path.display())
                            }));
                        }
                    }
                }
            }
        } else {
            messages.push(json!({
                "doc_id": id,
                "type": "warning",
                "message": format!("Document #{id} ('{title}'): Has no original filename recorded")
            }));
        }

        // Check archive file if present
        if let Some(ref afname) = archive_filename {
            known_archives.insert(afname.clone());
            let apath = archive_dir.join(afname);
            if !apath.exists() {
                messages.push(json!({
                    "doc_id": id,
                    "type": "error",
                    "message": format!("Document #{id} ('{title}'): Archive PDF file not found on disk at {}", apath.display())
                }));
            } else {
                checked_files += 1;
                if let Some(ref expected_ahash) = db_archive_checksum {
                    match tokio::fs::read(&apath).await {
                        Ok(bytes) => {
                            let actual_ahash = crate::ingest::sha256_hex(&bytes);
                            if !actual_ahash.eq_ignore_ascii_case(expected_ahash) {
                                messages.push(json!({
                                    "doc_id": id,
                                    "type": "error",
                                    "message": format!("Document #{id} ('{title}'): Archive checksum mismatch! DB: {expected_ahash}, Disk: {actual_ahash}")
                                }));
                            }
                        }
                        Err(e) => {
                            messages.push(json!({
                                "doc_id": id,
                                "type": "error",
                                "message": format!("Document #{id} ('{title}'): Could not read archive file {}: {e}", apath.display())
                            }));
                        }
                    }
                }
            }
        }
    }

    // Inspect orphan files in originals directory
    if originals_dir.exists() {
        if let Ok(mut entries) = tokio::fs::read_dir(&originals_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(ft) = entry.file_type().await {
                    if ft.is_file() {
                        let fname = entry.file_name().to_string_lossy().to_string();
                        if !known_originals.contains(&fname) {
                            messages.push(json!({
                                "doc_id": null,
                                "type": "warning",
                                "message": format!("Orphan file in originals/: {fname} is not associated with any active document")
                            }));
                        }
                    }
                }
            }
        }
    }

    // Inspect orphan files in archive directory
    if archive_dir.exists() {
        if let Ok(mut entries) = tokio::fs::read_dir(&archive_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(ft) = entry.file_type().await {
                    if ft.is_file() {
                        let fname = entry.file_name().to_string_lossy().to_string();
                        if !known_archives.contains(&fname) {
                            messages.push(json!({
                                "doc_id": null,
                                "type": "warning",
                                "message": format!("Orphan file in archive/: {fname} is not associated with any active document")
                            }));
                        }
                    }
                }
            }
        }
    }

    let has_errors = messages.iter().any(|m| m.get("type").and_then(|t| t.as_str()) == Some("error"));

    Ok(Json(json!({
        "passed": !has_errors,
        "total_documents": docs.len(),
        "checked_files": checked_files,
        "messages": messages
    })))
}

