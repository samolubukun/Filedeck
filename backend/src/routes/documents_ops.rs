//! Document operations beyond CRUD: file metadata, suggestions, trash, bulk object edits and
//! PDF manipulation (rotate / merge / edit / split / delete pages / remove password / reprocess).
//!
//! PDF work is delegated to the Python worker (`/pdf/*`); this module owns the bookkeeping.

use crate::error::AppError;
use crate::ingest::{self, ConsumeRequest};
use crate::matching::{self, Predicted};
use crate::search;
use crate::state::AppState;
use axum::{
    extract::{Multipart, Path, State},
    routing::{get, post},
    Json, Router,
};
use base64::Engine;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::path::PathBuf;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/documents/:id/metadata/", get(metadata))
        .route("/api/documents/:id/suggestions/", get(suggestions))
        .route("/api/documents/next_asn/", get(next_asn))
        .route("/api/documents/delete/", post(delete_many))
        .route("/api/documents/reprocess/", post(reprocess_many))
        .route("/api/documents/rotate/", post(rotate_many))
        .route("/api/documents/merge/", post(merge_many))
        .route("/api/documents/merge_as_versions/", post(merge_as_versions_endpoint))
        .route(
            "/api/documents/:id/update_version/",
            post(update_version_endpoint),
        )
        .route(
            "/api/documents/:id/update_version_label/",
            post(update_version_label_endpoint)
                .patch(update_version_label_endpoint)
                .put(update_version_label_endpoint),
        )
        .route("/api/documents/edit_pdf/", post(edit_pdf_many))
        .route(
            "/api/documents/remove_password/",
            post(remove_password_many),
        )
        .route("/api/trash/", get(trash_list).post(trash_action))
        .route("/api/bulk_edit_objects/", post(bulk_edit_objects))
}

fn ok() -> Json<Value> {
    Json(json!({ "result": "OK" }))
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

struct DocFiles {
    original: PathBuf,
    archive: Option<PathBuf>,
    stored_name: String,
    archive_name: Option<String>,
    original_filename: String,
    mime: String,
    title: String,
    content_len: i64,
}

async fn doc_files(state: &AppState, id: i64, include_trashed: bool) -> Result<DocFiles, AppError> {
    let row = sqlx::query(
        "SELECT filename, archive_filename, original_filename, mime_type, title, content_length, deleted_at
         FROM documents WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Document {id} not found")))?;
    let deleted: Option<String> = row.get("deleted_at");
    if deleted.is_some() && !include_trashed {
        return Err(AppError::NotFound(format!("Document {id} not found")));
    }
    let media = &state.config.storage.media_root;
    let stored_name: String = row.get::<Option<String>, _>("filename").unwrap_or_default();
    let archive_name: Option<String> = row.get("archive_filename");
    Ok(DocFiles {
        original: media.join("originals").join(&stored_name),
        archive: archive_name.as_ref().map(|a| media.join("archive").join(a)),
        stored_name,
        archive_name,
        original_filename: row
            .get::<Option<String>, _>("original_filename")
            .unwrap_or_else(|| format!("{id}.pdf")),
        mime: row.get("mime_type"),
        title: row.get("title"),
        content_len: row.get("content_length"),
    })
}

async fn require_pdf(state: &AppState, id: i64) -> Result<(DocFiles, Vec<u8>), AppError> {
    let f = doc_files(state, id, false).await?;
    if f.mime != "application/pdf" {
        return Err(AppError::Validation(format!(
            "Document {id} is not a PDF ({}) and cannot be modified",
            f.mime
        )));
    }
    let bytes = tokio::fs::read(&f.original).await.map_err(|_| {
        AppError::NotFound(format!(
            "Original file of document {id} is missing from disk"
        ))
    })?;
    Ok((f, bytes))
}

async fn worker_post(
    state: &AppState,
    path: &str,
    form: reqwest::multipart::Form,
) -> Result<Value, AppError> {
    let resp = state
        .worker_client
        .post(format!("{}{}", state.config.worker.url, path))
        .multipart(form)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Worker unreachable: {e}")))?;
    let status = resp.status();
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        let detail = body
            .get("detail")
            .and_then(|d| d.as_str())
            .unwrap_or("worker error");
        return Err(AppError::BadRequest(detail.to_string()));
    }
    Ok(body)
}

fn pdf_part(bytes: Vec<u8>, name: &str) -> reqwest::multipart::Part {
    reqwest::multipart::Part::bytes(bytes)
        .file_name(name.to_string())
        .mime_str("application/pdf")
        .unwrap()
}

fn decode_b64(v: &Value, key: &str) -> Result<Vec<u8>, AppError> {
    v.get(key)
        .and_then(|x| x.as_str())
        .and_then(|s| base64::engine::general_purpose::STANDARD.decode(s).ok())
        .ok_or_else(|| AppError::Internal("Worker returned no PDF".into()))
}

/// Replace a document's original file in place and refresh everything derived from it
/// (checksum, text, page count, thumbnail, archive copy, index). Also used for "reprocess".
async fn replace_file(state: &AppState, id: i64, new_bytes: Vec<u8>) -> Result<(), AppError> {
    let f = doc_files(state, id, false).await?;
    let req = ConsumeRequest {
        bytes: new_bytes.clone(),
        filename: f.original_filename.clone(),
        ..Default::default()
    };
    let processed = ingest::call_worker(state, &req).await;
    if let Some(w) = &processed.worker_warning {
        return Err(AppError::Internal(w.clone()));
    }

    tokio::fs::write(&f.original, &new_bytes).await?;
    let media = &state.config.storage.media_root;

    let (archive_name, archive_checksum) = match processed.archive {
        Some(bytes) => {
            let name = f
                .archive_name
                .clone()
                .unwrap_or_else(|| format!("{}.pdf", f.stored_name.trim_end_matches(".pdf")));
            tokio::fs::create_dir_all(media.join("archive")).await?;
            tokio::fs::write(media.join("archive").join(&name), &bytes).await?;
            (Some(name), Some(ingest::sha256_hex(&bytes)))
        }
        None => {
            if let Some(old) = &f.archive {
                let _ = tokio::fs::remove_file(old).await;
            }
            (None, None)
        }
    };

    if let Some(thumb) = processed.thumbnail {
        let _ = tokio::fs::write(
            media.join("thumbnails").join(format!("{id:07}.webp")),
            thumb,
        )
        .await;
    }

    sqlx::query(
        "UPDATE documents SET checksum = $1, archive_filename = $2, archive_checksum = $3, content = $4,
                content_length = $5, page_count = $6, modified = $7 WHERE id = $8",
    )
    .bind(ingest::sha256_hex(&new_bytes))
    .bind(&archive_name)
    .bind(&archive_checksum)
    .bind(&processed.content)
    .bind(processed.content.chars().count() as i64)
    .bind(processed.page_count.max(1))
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(id)
    .execute(&state.db)
    .await?;
    let _ = f.content_len;
    search::reindex_document(state, id).await?;
    Ok(())
}

/// Queue `bytes` as a new document inheriting metadata from `src_id`.
async fn submit_derived(
    state: &AppState,
    src_id: i64,
    bytes: Vec<u8>,
    filename: String,
    title: String,
) -> Result<String, AppError> {
    let src = sqlx::query(
        "SELECT correspondent_id, document_type_id, storage_path_id, created FROM documents WHERE id = $1",
    )
    .bind(src_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Document {src_id} not found")))?;
    let tags: Vec<i64> =
        sqlx::query_scalar("SELECT tag_id FROM document_tags WHERE document_id = $1")
            .bind(src_id)
            .fetch_all(&state.db)
            .await?;
    ingest::submit(
        state,
        ConsumeRequest {
            bytes,
            filename,
            title: Some(title),
            correspondent: src.get("correspondent_id"),
            document_type: src.get("document_type_id"),
            storage_path: src.get("storage_path_id"),
            created: Some(src.get::<String, _>("created")),
            tags,
            source: "web_ui".into(),
            ..Default::default()
        },
    )
    .await
}

async fn trash_documents(state: &AppState, ids: &[i64]) -> Result<(), AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    for id in ids {
        sqlx::query("UPDATE documents SET deleted_at = $1 WHERE id = $2 AND deleted_at IS NULL")
            .bind(&now)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    search::reindex_documents(state, ids).await
}

// ---------------------------------------------------------------------------
// request payloads
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct DocsPayload {
    pub documents: Vec<i64>,
}

#[derive(Deserialize)]
pub struct RotatePayload {
    pub documents: Vec<i64>,
    pub degrees: i64,
}

#[derive(Deserialize)]
pub struct MergePayload {
    pub documents: Vec<i64>,
    pub metadata_document_id: Option<i64>,
    #[serde(default)]
    pub delete_originals: bool,
    pub archive_fallback: Option<bool>,
}

#[derive(Deserialize)]
pub struct EditPdfPayload {
    pub documents: Vec<i64>,
    pub operations: Vec<Value>,
    #[serde(default)]
    pub delete_original: bool,
    #[serde(default)]
    pub update_document: bool,
    pub include_metadata: Option<bool>,
}

#[derive(Deserialize)]
pub struct RemovePasswordPayload {
    pub documents: Vec<i64>,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub update_document: bool,
    #[serde(default)]
    pub delete_original: bool,
}

// ---------------------------------------------------------------------------
// operations (shared by the dedicated endpoints and `bulk_edit`)
// ---------------------------------------------------------------------------

pub async fn op_reprocess(state: &AppState, docs: &[i64]) -> Result<(), AppError> {
    for id in docs {
        let f = doc_files(state, *id, false).await?;
        let bytes = tokio::fs::read(&f.original).await.map_err(|_| {
            AppError::NotFound(format!("Original file of document {id} is missing"))
        })?;
        replace_file(state, *id, bytes).await?;
    }
    Ok(())
}

pub async fn op_rotate(state: &AppState, docs: &[i64], degrees: i64) -> Result<(), AppError> {
    if degrees % 90 != 0 {
        return Err(AppError::Validation(
            "degrees must be a multiple of 90".into(),
        ));
    }
    for id in docs {
        let (f, bytes) = require_pdf(state, *id).await?;
        let form = reqwest::multipart::Form::new()
            .part("file", pdf_part(bytes, &f.original_filename))
            .text("degrees", degrees.to_string());
        let out = worker_post(state, "/pdf/rotate", form).await?;
        replace_file(state, *id, decode_b64(&out, "pdf_b64")?).await?;
    }
    Ok(())
}

pub async fn op_merge(
    state: &AppState,
    docs: &[i64],
    metadata_id: Option<i64>,
    delete_originals: bool,
) -> Result<String, AppError> {
    if docs.len() < 2 {
        return Err(AppError::Validation(
            "At least two documents are required to merge".into(),
        ));
    }
    let mut form = reqwest::multipart::Form::new();
    let mut first_title = String::new();
    for (n, id) in docs.iter().enumerate() {
        let (f, bytes) = require_pdf(state, *id).await?;
        if n == 0 {
            first_title = f.title.clone();
        }
        form = form.part("files", pdf_part(bytes, &f.original_filename));
    }
    let merged = decode_b64(&worker_post(state, "/pdf/merge", form).await?, "pdf_b64")?;
    let src = metadata_id.unwrap_or(docs[0]);
    let title = format!("{first_title} (merged)");
    let task_id = submit_derived(state, src, merged, format!("{title}.pdf"), title).await?;

    if delete_originals {
        let st = state.clone();
        let tid = task_id.clone();
        let originals = docs.to_vec();
        tokio::spawn(async move {
            // Only trash the sources once the merged document actually exists.
            for _ in 0..600 {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let status: Option<String> =
                    sqlx::query_scalar("SELECT status FROM tasks WHERE task_id = $1")
                        .bind(&tid)
                        .fetch_optional(&st.db)
                        .await
                        .ok()
                        .flatten();
                match status.as_deref() {
                    Some("success") => {
                        let _ = trash_documents(&st, &originals).await;
                        return;
                    }
                    Some("failure") | None => return,
                    _ => {}
                }
            }
        });
    }
    Ok(task_id)
}

pub async fn op_edit_pdf(
    state: &AppState,
    doc: i64,
    operations: &[Value],
    update_document: bool,
    delete_original: bool,
) -> Result<Vec<String>, AppError> {
    if operations.is_empty() {
        return Err(AppError::Validation("No operations given".into()));
    }
    let (f, bytes) = require_pdf(state, doc).await?;
    let form = reqwest::multipart::Form::new()
        .part("file", pdf_part(bytes, &f.original_filename))
        .text("operations", serde_json::to_string(operations).unwrap());
    let out = worker_post(state, "/pdf/edit", form).await?;
    let parts: Vec<Vec<u8>> = out
        .get("parts")
        .and_then(|p| p.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str())
                .filter_map(|s| base64::engine::general_purpose::STANDARD.decode(s).ok())
                .collect()
        })
        .unwrap_or_default();
    if parts.is_empty() {
        return Err(AppError::Validation("Edit produced no pages".into()));
    }

    if update_document {
        if parts.len() != 1 {
            return Err(AppError::Validation(
                "update_document requires the operations to produce a single document".into(),
            ));
        }
        replace_file(state, doc, parts.into_iter().next().unwrap()).await?;
        return Ok(vec![]);
    }

    let mut tasks = Vec::new();
    for (n, bytes) in parts.into_iter().enumerate() {
        let title = format!("{} (edited {})", f.title, n + 1);
        tasks.push(submit_derived(state, doc, bytes, format!("{title}.pdf"), title).await?);
    }
    if delete_original {
        trash_documents(state, &[doc]).await?;
    }
    Ok(tasks)
}

pub async fn op_delete_pages(
    state: &AppState,
    docs: &[i64],
    pages: &[i64],
) -> Result<(), AppError> {
    for id in docs {
        let (f, bytes) = require_pdf(state, *id).await?;
        let total =
            sqlx::query_scalar::<_, Option<i64>>("SELECT page_count FROM documents WHERE id = $1")
                .bind(id)
                .fetch_one(&state.db)
                .await?
                .unwrap_or(1);
        let ops: Vec<Value> = (1..=total)
            .filter(|p| !pages.contains(p))
            .map(|p| json!({"page": p, "doc": 0}))
            .collect();
        if ops.is_empty() {
            return Err(AppError::Validation(
                "Cannot delete every page of a document".into(),
            ));
        }
        let form = reqwest::multipart::Form::new()
            .part("file", pdf_part(bytes, &f.original_filename))
            .text("operations", serde_json::to_string(&ops).unwrap());
        let out = worker_post(state, "/pdf/edit", form).await?;
        let part = out
            .get("parts")
            .and_then(|p| p.as_array())
            .and_then(|a| a.first())
            .and_then(|s| s.as_str())
            .and_then(|s| base64::engine::general_purpose::STANDARD.decode(s).ok())
            .ok_or_else(|| AppError::Internal("Worker returned no PDF".into()))?;
        replace_file(state, *id, part).await?;
    }
    Ok(())
}

pub async fn op_split(
    state: &AppState,
    doc: i64,
    groups: &[Vec<i64>],
    delete_original: bool,
) -> Result<Vec<String>, AppError> {
    let ops: Vec<Value> = groups
        .iter()
        .enumerate()
        .flat_map(|(g, pages)| pages.iter().map(move |p| json!({"page": p, "doc": g})))
        .collect();
    op_edit_pdf(state, doc, &ops, false, delete_original).await
}

pub async fn op_remove_password(
    state: &AppState,
    doc: i64,
    password: &str,
    update_document: bool,
    delete_original: bool,
) -> Result<Option<String>, AppError> {
    let (f, bytes) = require_pdf(state, doc).await?;
    let form = reqwest::multipart::Form::new()
        .part("file", pdf_part(bytes, &f.original_filename))
        .text("password", password.to_string());
    let out = decode_b64(
        &worker_post(state, "/pdf/remove-password", form).await?,
        "pdf_b64",
    )?;
    if update_document {
        replace_file(state, doc, out).await?;
        return Ok(None);
    }
    let title = format!("{} (unlocked)", f.title);
    let task = submit_derived(state, doc, out, format!("{title}.pdf"), title).await?;
    if delete_original {
        trash_documents(state, &[doc]).await?;
    }
    Ok(Some(task))
}

/// Extra `bulk_edit` methods. Returns Ok(None) when `method` is not one of ours.
pub async fn bulk_dispatch(
    state: &AppState,
    method: &str,
    docs: &[i64],
    params: &Value,
) -> Result<Option<Value>, AppError> {
    let flag = |k: &str| params.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
    match method {
        "reprocess" => op_reprocess(state, docs).await?,
        "rotate" => {
            let degrees = params
                .get("degrees")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| AppError::Validation("degrees required".into()))?;
            op_rotate(state, docs, degrees).await?
        }
        "merge" => {
            let task = op_merge(
                state,
                docs,
                params.get("metadata_document_id").and_then(|v| v.as_i64()),
                flag("delete_originals"),
            )
            .await?;
            return Ok(Some(json!({"result": "OK", "task_id": task})));
        }
        "merge_as_versions" => {
            let root_id = params
                .get("root_document_id")
                .and_then(|v| v.as_i64())
                .or_else(|| docs.first().copied())
                .ok_or_else(|| AppError::Validation("root_document_id required".into()))?;
            let version_label = params.get("version_label").and_then(|v| v.as_str());
            let res = merge_as_versions_inner(state, docs, root_id, version_label).await?;
            return Ok(Some(res));
        }
        "delete_pages" => {
            let pages: Vec<i64> = params
                .get("pages")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_i64()).collect())
                .unwrap_or_default();
            op_delete_pages(state, docs, &pages).await?
        }
        "split" => {
            let groups: Vec<Vec<i64>> = params
                .get("pages")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .ok_or_else(|| AppError::Validation("pages must be a list of page lists".into()))?;
            let doc = *docs
                .first()
                .ok_or_else(|| AppError::Validation("No document given".into()))?;
            let tasks = op_split(state, doc, &groups, flag("delete_originals")).await?;
            return Ok(Some(json!({"result": "OK", "task_ids": tasks})));
        }
        "edit_pdf" => {
            let ops: Vec<Value> = params
                .get("operations")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let doc = *docs
                .first()
                .ok_or_else(|| AppError::Validation("No document given".into()))?;
            let tasks = op_edit_pdf(
                state,
                doc,
                &ops,
                flag("update_document"),
                flag("delete_original"),
            )
            .await?;
            return Ok(Some(json!({"result": "OK", "task_ids": tasks})));
        }
        "remove_password" => {
            let password = params
                .get("password")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            for id in docs {
                op_remove_password(
                    state,
                    *id,
                    password,
                    flag("update_document"),
                    flag("delete_original"),
                )
                .await?;
            }
        }
        _ => return Ok(None),
    }
    Ok(Some(json!({"result": "OK"})))
}

// ---------------------------------------------------------------------------
// handlers
// ---------------------------------------------------------------------------

async fn delete_many(
    State(state): State<AppState>,
    Json(p): Json<DocsPayload>,
) -> Result<Json<Value>, AppError> {
    trash_documents(&state, &p.documents).await?;
    Ok(ok())
}

async fn reprocess_many(
    State(state): State<AppState>,
    Json(p): Json<DocsPayload>,
) -> Result<Json<Value>, AppError> {
    op_reprocess(&state, &p.documents).await?;
    Ok(ok())
}

async fn rotate_many(
    State(state): State<AppState>,
    Json(p): Json<RotatePayload>,
) -> Result<Json<Value>, AppError> {
    op_rotate(&state, &p.documents, p.degrees).await?;
    Ok(ok())
}

async fn merge_many(
    State(state): State<AppState>,
    Json(p): Json<MergePayload>,
) -> Result<Json<Value>, AppError> {
    let _ = p.archive_fallback;
    let task = op_merge(
        &state,
        &p.documents,
        p.metadata_document_id,
        p.delete_originals,
    )
    .await?;
    Ok(Json(json!({"result": "OK", "task_id": task})))
}

async fn edit_pdf_many(
    State(state): State<AppState>,
    Json(p): Json<EditPdfPayload>,
) -> Result<Json<Value>, AppError> {
    let _ = p.include_metadata;
    let doc = *p
        .documents
        .first()
        .ok_or_else(|| AppError::Validation("No document given".into()))?;
    if p.documents.len() != 1 {
        return Err(AppError::Validation(
            "edit_pdf works on exactly one document".into(),
        ));
    }
    let tasks = op_edit_pdf(
        &state,
        doc,
        &p.operations,
        p.update_document,
        p.delete_original,
    )
    .await?;
    Ok(Json(json!({"result": "OK", "task_ids": tasks})))
}

async fn remove_password_many(
    State(state): State<AppState>,
    Json(p): Json<RemovePasswordPayload>,
) -> Result<Json<Value>, AppError> {
    for id in &p.documents {
        op_remove_password(
            &state,
            *id,
            &p.password,
            p.update_document,
            p.delete_original,
        )
        .await?;
    }
    Ok(ok())
}

async fn next_asn(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let next: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(archive_serial_number), 0) + 1 FROM documents")
            .fetch_one(&state.db)
            .await?;
    Ok(Json(json!(next)))
}

async fn metadata(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let f = doc_files(&state, id, false).await?;
    let row = sqlx::query("SELECT checksum, archive_checksum FROM documents WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await?;
    let original_size = tokio::fs::metadata(&f.original).await.map(|m| m.len()).ok();
    let (archive_size, has_archive) = match &f.archive {
        Some(p) => (
            tokio::fs::metadata(p).await.map(|m| m.len()).ok(),
            p.exists(),
        ),
        None => (None, false),
    };
    Ok(Json(json!({
        "original_checksum": row.get::<String, _>("checksum"),
        "original_size": original_size,
        "original_mime_type": f.mime,
        "media_filename": f.stored_name,
        "has_archive_version": has_archive,
        "original_metadata": [],
        "archive_checksum": row.get::<Option<String>, _>("archive_checksum"),
        "archive_media_filename": f.archive_name,
        "original_filename": f.original_filename,
        "archive_size": archive_size,
        "archive_metadata": [],
        "lang": state.config.ocr.language,
    })))
}

async fn suggestions(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let content: String =
        sqlx::query_scalar("SELECT content FROM documents WHERE id = $1 AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Document {id} not found")))?;
    let a = matching::assign(&state.db, &content, &Predicted::default()).await?;
    let dates: Vec<String> = ingest::dates::find_date(&content).into_iter().collect();
    Ok(Json(json!({
        "correspondents": a.correspondent.into_iter().collect::<Vec<_>>(),
        "tags": a.tags,
        "document_types": a.document_type.into_iter().collect::<Vec<_>>(),
        "storage_paths": a.storage_path.into_iter().collect::<Vec<_>>(),
        "dates": dates,
    })))
}

async fn trash_list(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query(
        "SELECT id, title, created, added, modified, deleted_at, correspondent_id, document_type_id,
                storage_path_id, original_filename
         FROM documents WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC",
    )
    .fetch_all(&state.db)
    .await?;
    let mut results = Vec::with_capacity(rows.len());
    for r in rows {
        let id: i64 = r.get("id");
        let tags: Vec<i64> =
            sqlx::query_scalar("SELECT tag_id FROM document_tags WHERE document_id = $1")
                .bind(id)
                .fetch_all(&state.db)
                .await?;
        results.push(json!({
            "id": id,
            "title": r.get::<String, _>("title"),
            "created": r.get::<String, _>("created"),
            "added": r.get::<String, _>("added"),
            "modified": r.get::<String, _>("modified"),
            "deleted_at": r.get::<Option<String>, _>("deleted_at"),
            "correspondent": r.get::<Option<i64>, _>("correspondent_id"),
            "document_type": r.get::<Option<i64>, _>("document_type_id"),
            "storage_path": r.get::<Option<i64>, _>("storage_path_id"),
            "original_file_name": r.get::<Option<String>, _>("original_filename"),
            "tags": tags,
            "thumbnail_url": format!("/api/documents/{id}/thumb/"),
        }));
    }
    Ok(Json(json!({
        "count": results.len(),
        "next": null,
        "previous": null,
        "all": results.iter().map(|r| r["id"].clone()).collect::<Vec<_>>(),
        "results": results,
    })))
}

#[derive(Deserialize)]
pub struct TrashPayload {
    pub documents: Option<Vec<i64>>,
    pub action: String,
}

/// Permanently remove a document row and all of its files.
pub async fn purge_documents(state: &AppState, ids: &[i64]) -> Result<(), AppError> {
    for id in ids {
        let Ok(f) = doc_files(state, *id, true).await else {
            continue;
        };
        let _ = tokio::fs::remove_file(&f.original).await;
        if let Some(a) = &f.archive {
            let _ = tokio::fs::remove_file(a).await;
        }
        let _ = tokio::fs::remove_file(
            state
                .config
                .storage
                .media_root
                .join("thumbnails")
                .join(format!("{id:07}.webp")),
        )
        .await;
        sqlx::query("DELETE FROM documents WHERE id = $1")
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    search::reindex_documents(state, ids).await
}

async fn trash_action(
    State(state): State<AppState>,
    Json(p): Json<TrashPayload>,
) -> Result<Json<Value>, AppError> {
    let ids: Vec<i64> = match p.documents {
        Some(d) if !d.is_empty() => d,
        _ => {
            sqlx::query_scalar("SELECT id FROM documents WHERE deleted_at IS NOT NULL")
                .fetch_all(&state.db)
                .await?
        }
    };
    match p.action.as_str() {
        "restore" => {
            for id in &ids {
                sqlx::query("UPDATE documents SET deleted_at = NULL WHERE id = $1")
                    .bind(id)
                    .execute(&state.db)
                    .await?;
            }
            search::reindex_documents(&state, &ids).await?;
        }
        "empty" => {
            // Only ever purge documents that really are in the trash.
            let trashed: Vec<i64> =
                sqlx::query_scalar("SELECT id FROM documents WHERE deleted_at IS NOT NULL")
                    .fetch_all(&state.db)
                    .await?;
            let ids: Vec<i64> = ids.into_iter().filter(|i| trashed.contains(i)).collect();
            purge_documents(&state, &ids).await?;
        }
        other => {
            return Err(AppError::Validation(format!(
                "Invalid trash action '{other}'"
            )))
        }
    }
    Ok(ok())
}

#[derive(Deserialize)]
pub struct BulkObjectsPayload {
    pub objects: Vec<i64>,
    pub object_type: String,
    pub operation: String,
}

async fn bulk_edit_objects(
    State(state): State<AppState>,
    Json(p): Json<BulkObjectsPayload>,
) -> Result<Json<Value>, AppError> {
    let table = match p.object_type.as_str() {
        "tags" => "tags",
        "correspondents" => "correspondents",
        "document_types" => "document_types",
        "storage_paths" => "storage_paths",
        "custom_fields" => "custom_fields",
        other => {
            return Err(AppError::Validation(format!(
                "Invalid object type '{other}'"
            )))
        }
    };
    match p.operation.as_str() {
        "delete" => {
            for id in &p.objects {
                sqlx::query(&format!("DELETE FROM {table} WHERE id = $1"))
                    .bind(id)
                    .execute(&state.db)
                    .await?;
            }
            search::reindex_all_background(&state);
        }
        // Single user: there are no owners or permissions to change.
        "set_permissions" => {}
        other => return Err(AppError::Validation(format!("Invalid operation '{other}'"))),
    }
    Ok(ok())
}

// ---------------------------------------------------------------------------
// Document versions
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct MergeAsVersionsPayload {
    pub documents: Vec<i64>,
    pub root_document_id: i64,
    pub version_label: Option<String>,
}

pub async fn merge_as_versions_inner(
    state: &AppState,
    doc_ids: &[i64],
    root_document_id: i64,
    version_label: Option<&str>,
) -> Result<Value, AppError> {
    if doc_ids.is_empty() {
        return Err(AppError::Validation("No documents specified to merge as versions".into()));
    }
    if !doc_ids.contains(&root_document_id) {
        return Err(AppError::Validation(format!(
            "root_document_id {root_document_id} must be in the documents list"
        )));
    }

    let root_row = sqlx::query("SELECT id, root_document_id, archive_serial_number FROM documents WHERE id = $1 AND deleted_at IS NULL")
        .bind(root_document_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Root document {root_document_id} not found")))?;

    if root_row.get::<Option<i64>, _>("root_document_id").is_some() {
        return Err(AppError::Validation(format!(
            "Document {root_document_id} is already a version and cannot be a root document"
        )));
    }

    let source_ids: Vec<i64> = doc_ids.iter().copied().filter(|&id| id != root_document_id).collect();
    if source_ids.is_empty() {
        return Ok(json!({ "result": "OK" }));
    }

    let mut next_version_index: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(version_index), 0) FROM documents WHERE root_document_id = $1",
    )
    .bind(root_document_id)
    .fetch_one(&state.db)
    .await?;

    let now = chrono::Utc::now().to_rfc3339();
    let mut tx = state.db.begin().await?;

    let mut root_asn: Option<i64> = root_row.get("archive_serial_number");

    for source_id in &source_ids {
        next_version_index += 1;
        let source_row = sqlx::query("SELECT archive_serial_number FROM documents WHERE id = $1")
            .bind(source_id)
            .fetch_optional(&mut *tx)
            .await?;
        if let Some(srow) = source_row {
            let s_asn: Option<i64> = srow.get("archive_serial_number");
            if root_asn.is_none() && s_asn.is_some() {
                root_asn = s_asn;
            }
        }

        sqlx::query(
            "UPDATE documents SET root_document_id = $1, version_index = $2, version_label = $3, archive_serial_number = NULL, modified = $4 WHERE id = $5",
        )
        .bind(root_document_id)
        .bind(next_version_index)
        .bind(version_label)
        .bind(&now)
        .bind(source_id)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        "UPDATE documents SET archive_serial_number = $1, modified = $2 WHERE id = $3",
    )
    .bind(root_asn)
    .bind(&now)
    .bind(root_document_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    for source_id in &source_ids {
        crate::history::record(
            &state.db,
            *source_id,
            "update",
            json!({ "root_document": [Value::Null, root_document_id] }),
        )
        .await;
    }
    crate::history::record(
        &state.db,
        root_document_id,
        "update",
        json!({ "versions_merged": [Value::Null, source_ids] }),
    )
    .await;

    search::reindex_documents(state, doc_ids).await?;

    Ok(json!({ "result": "OK" }))
}

async fn merge_as_versions_endpoint(
    State(state): State<AppState>,
    Json(p): Json<MergeAsVersionsPayload>,
) -> Result<Json<Value>, AppError> {
    let res = merge_as_versions_inner(
        &state,
        &p.documents,
        p.root_document_id,
        p.version_label.as_deref(),
    )
    .await?;
    Ok(Json(res))
}

async fn update_version_endpoint(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    mut multipart: Multipart,
) -> Result<Json<Value>, AppError> {
    let doc_row = sqlx::query("SELECT id, root_document_id FROM documents WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Document {id} not found")))?;

    let root_id = doc_row.get::<Option<i64>, _>("root_document_id").unwrap_or(id);

    let next_index: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(version_index), 0) + 1 FROM documents WHERE root_document_id = $1",
    )
    .bind(root_id)
    .fetch_one(&state.db)
    .await?;

    let mut req = ConsumeRequest {
        filename: "version_upload.pdf".into(),
        source: "update_version".into(),
        root_document_id: Some(root_id),
        version_index: Some(next_index),
        ..Default::default()
    };

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if name == "document" {
            if let Some(fname) = field.file_name() {
                req.filename = fname.to_string();
            }
            req.bytes = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(e.to_string()))?
                .to_vec();
            continue;
        }
        let text = field
            .text()
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;
        let text = text.trim().to_string();
        if text.is_empty() {
            continue;
        }
        if name == "version_label" {
            req.version_label = Some(text);
        }
    }

    if req.bytes.is_empty() {
        return Err(AppError::BadRequest("document file is required".into()));
    }

    let task_id = crate::ingest::submit(&state, req).await?;
    Ok(Json(json!(task_id)))

}

#[derive(Deserialize)]
pub struct UpdateVersionLabelPayload {
    pub version_id: Option<i64>,
    pub version_label: Option<String>,
}

async fn update_version_label_endpoint(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(p): Json<UpdateVersionLabelPayload>,
) -> Result<Json<Value>, AppError> {
    let target_id = if let Some(vid) = p.version_id {
        let parent: Option<i64> = sqlx::query_scalar("SELECT root_document_id FROM documents WHERE id = $1")
            .bind(vid)
            .fetch_optional(&state.db)
            .await?
            .flatten();
        if parent != Some(id) && vid != id {
            return Err(AppError::NotFound(format!("Version {vid} does not belong to document {id}")));
        }
        vid
    } else {
        id
    };

    let trimmed = p.version_label.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query("UPDATE documents SET version_label = $1, modified = $2 WHERE id = $3")
        .bind(trimmed)
        .bind(&now)
        .bind(target_id)
        .execute(&state.db)
        .await?;

    crate::history::record(
        &state.db,
        target_id,
        "update",
        json!({ "version_label": [Value::Null, trimmed] }),
    )
    .await;

    Ok(Json(json!({ "result": "OK" })))
}

