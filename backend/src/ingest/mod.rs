//! Document ingestion pipeline.
//!
//! Sources (API upload, consume folder, mail) build a [`ConsumeRequest`] and hand it to
//! [`submit`] (fire-and-forget, returns the task id) or [`consume_and_wait`].
//!
//! Pipeline: task record -> mime detection -> worker (OCR/text/thumbnail/barcodes/archive)
//! -> barcode split -> duplicate + ASN checks -> classifier + matching engine -> store
//! document -> index -> task result + WebSocket updates.

pub mod dates;
pub mod extras;
pub mod office;
pub mod watcher;

use crate::error::AppError;
use crate::matching::{self, Predicted};
use crate::routes::documents::set_custom_field_value;
use crate::search;
use crate::state::AppState;
use base64::Engine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::time::Instant;

#[derive(Clone, Default)]
pub struct ConsumeRequest {
    pub bytes: Vec<u8>,
    pub filename: String,
    pub title: Option<String>,
    pub correspondent: Option<i64>,
    pub document_type: Option<i64>,
    pub storage_path: Option<i64>,
    pub tags: Vec<i64>,
    pub created: Option<String>,
    pub asn: Option<i64>,
    pub custom_fields: Vec<(i64, Value)>,
    /// `api_upload`, `consume_folder`, `mail_fetch`, ...
    pub source: String,
    pub root_document_id: Option<i64>,
    pub version_index: Option<i64>,
    pub version_label: Option<String>,
}


#[derive(Debug)]
pub struct IngestFailure(pub String);

impl From<sqlx::Error> for IngestFailure {
    fn from(e: sqlx::Error) -> Self {
        IngestFailure(format!("Database error: {e}"))
    }
}
impl From<std::io::Error> for IngestFailure {
    fn from(e: std::io::Error) -> Self {
        IngestFailure(format!("IO error: {e}"))
    }
}
impl From<AppError> for IngestFailure {
    fn from(e: AppError) -> Self {
        IngestFailure(e.to_string())
    }
}
impl From<reqwest::Error> for IngestFailure {
    fn from(e: reqwest::Error) -> Self {
        IngestFailure(format!("Network error: {e}"))
    }
}


/// Result of the worker's `/process-doc`.
#[derive(Default)]
pub struct Processed {
    pub content: String,
    pub page_count: i64,
    pub thumbnail: Option<Vec<u8>>,
    pub barcodes: Vec<Value>,
    pub separator_pages: Vec<i64>,
    pub archive: Option<Vec<u8>>,
    pub worker_warning: Option<String>,
}

const OFFICE_EXT: &[&str] = &[
    "doc", "docx", "odt", "rtf", "xls", "xlsx", "ods", "ppt", "pptx", "odp",
];
const IMAGE_EXT: &[(&str, &str)] = &[
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("webp", "image/webp"),
    ("tif", "image/tiff"),
    ("tiff", "image/tiff"),
    ("bmp", "image/bmp"),
    ("gif", "image/gif"),
];
const TEXT_EXT: &[&str] = &["txt", "md", "csv", "json", "log"];

pub fn extension(name: &str) -> String {
    std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// Mime type for supported files, `None` for unsupported ones.
pub fn detect_mime(filename: &str, bytes: &[u8]) -> Option<String> {
    if bytes.starts_with(b"%PDF") {
        return Some("application/pdf".into());
    }
    let ext = extension(filename);
    if ext == "pdf" {
        return Some("application/pdf".into());
    }
    if let Some((_, m)) = IMAGE_EXT.iter().find(|(e, _)| *e == ext) {
        return Some((*m).into());
    }
    if TEXT_EXT.contains(&ext.as_str()) {
        return Some(
            match ext.as_str() {
                "csv" => "text/csv",
                "md" => "text/markdown",
                "json" => "application/json",
                _ => "text/plain",
            }
            .into(),
        );
    }
    let office = match ext.as_str() {
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "odt" => "application/vnd.oasis.opendocument.text",
        "rtf" => "application/rtf",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "odp" => "application/vnd.oasis.opendocument.presentation",
        _ => return None,
    };
    debug_assert!(OFFICE_EXT.contains(&ext.as_str()));
    Some(office.into())
}

pub fn is_supported(filename: &str) -> bool {
    detect_mime(filename, &[]).is_some()
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Broadcast a `status_update` message on the status WebSocket.
fn notify(
    state: &AppState,
    task_id: &str,
    filename: &str,
    status: &str,
    progress: u32,
    message: &str,
    document_id: Option<i64>,
) {
    let _ = state.ws_tx.send(
        json!({
            "type": "status_update",
            "data": {
                "filename": filename,
                "task_id": task_id,
                "current_progress": progress,
                "max_progress": 100,
                "status": status,
                "message": message,
                "document_id": document_id,
            }
        })
        .to_string(),
    );
}

/// Create the task record (status `pending`) and return its id.
pub async fn create_task(state: &AppState, req: &ConsumeRequest) -> Result<String, AppError> {
    let task_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO tasks (task_id, task_type, trigger_source, status, date_created, input_data)
         VALUES ($1, 'consume_file', $2, 'pending', $3, $4)",
    )
    .bind(&task_id)
    .bind(&req.source)
    .bind(now())
    .bind(json!({"filename": req.filename, "checksum": sha256_hex(&req.bytes)}).to_string())
    .execute(&state.db)
    .await?;
    notify(state, &task_id, &req.filename, "PENDING", 0, "Queued", None);
    Ok(task_id)
}

/// Queue the file and return immediately with the task id.
pub async fn submit(state: &AppState, req: ConsumeRequest) -> Result<String, AppError> {
    let task_id = create_task(state, &req).await?;
    let st = state.clone();
    let tid = task_id.clone();
    tokio::spawn(async move {
        let _ = run_task(&st, &tid, req).await;
    });
    Ok(task_id)
}

/// Run the pipeline to completion (used by the folder watcher, which deletes the source after).
pub async fn consume_and_wait(state: &AppState, req: ConsumeRequest) -> Result<Vec<i64>, String> {
    let task_id = create_task(state, &req).await.map_err(|e| e.to_string())?;
    run_task(state, &task_id, req).await
}

/// Execute a queued task, persisting its outcome.
pub async fn run_task(
    state: &AppState,
    task_id: &str,
    req: ConsumeRequest,
) -> Result<Vec<i64>, String> {
    let started = Instant::now();
    let started_at = now();
    let _ = sqlx::query(
        "UPDATE tasks SET status = 'started', date_started = $1,
                wait_time_seconds = (julianday($1) - julianday(date_created)) * 86400.0
         WHERE task_id = $2",
    )
    .bind(&started_at)
    .bind(task_id)
    .execute(&state.db)
    .await;
    notify(
        state,
        task_id,
        &req.filename,
        "STARTED",
        5,
        "Starting",
        None,
    );

    let outcome = pipeline(state, task_id, &req).await;
    let duration = started.elapsed().as_secs_f64();

    match outcome {
        Ok((ids, warning)) => {
            let mut result = json!({"document_id": ids.first(), "document_ids": ids});
            if let Some(w) = warning {
                result["warning"] = json!(w);
            }
            let _ = sqlx::query(
                "UPDATE tasks SET status = 'success', date_done = $1, duration_seconds = $2, result_data = $3 WHERE task_id = $4",
            )
            .bind(now())
            .bind(duration)
            .bind(result.to_string())
            .bind(task_id)
            .execute(&state.db)
            .await;
            notify(
                state,
                task_id,
                &req.filename,
                "SUCCESS",
                100,
                "Success",
                ids.first().copied(),
            );
            let _ = state.ws_tx.send(
                json!({"type": "document_created", "document_id": ids.first(), "task_id": task_id})
                    .to_string(),
            );
            Ok(ids)
        }
        Err(IngestFailure(msg)) => {
            tracing::warn!("Ingestion of '{}' failed: {}", req.filename, msg);
            let _ = sqlx::query(
                "UPDATE tasks SET status = 'failure', date_done = $1, duration_seconds = $2, result_data = $3 WHERE task_id = $4",
            )
            .bind(now())
            .bind(duration)
            .bind(json!({"error": msg}).to_string())
            .bind(task_id)
            .execute(&state.db)
            .await;
            notify(state, task_id, &req.filename, "FAILED", 100, &msg, None);
            Err(msg)
        }
    }
}

pub async fn call_worker(state: &AppState, req: &ConsumeRequest) -> Processed {
    let cfg = &state.config.consumer;
    let part = reqwest::multipart::Part::bytes(req.bytes.clone())
        .file_name(req.filename.clone())
        .mime_str("application/octet-stream")
        .unwrap();
    let mut form = reqwest::multipart::Form::new().part("file", part);
    form = form.text("separator", cfg.barcode_string.clone());

    let url = format!("{}/process-doc", state.config.worker.url);
    let resp = match state.worker_client.post(&url).multipart(form).send().await {
        Ok(r) => r,
        Err(e) => {
            if office::is_office_document(&req.filename) {
                let content = office::extract_text_content(&req.bytes, &req.filename).unwrap_or_default();
                let archive = office::convert_to_pdf(&req.bytes, &req.filename);
                let thumbnail = office::render_first_page_png(&req.bytes, &req.filename, 120);
                return Processed {
                    content,
                    page_count: 1,
                    thumbnail,
                    archive,
                    worker_warning: None,
                    ..Default::default()
                };
            }
            return Processed {
                page_count: 1,
                worker_warning: Some(format!("Worker unreachable, text was not extracted: {e}")),
                ..Default::default()
            };
        }
    };
    if !resp.status().is_success() {
        if office::is_office_document(&req.filename) {
            let content = office::extract_text_content(&req.bytes, &req.filename).unwrap_or_default();
            let archive = office::convert_to_pdf(&req.bytes, &req.filename);
            let thumbnail = office::render_first_page_png(&req.bytes, &req.filename, 120);
            return Processed {
                content,
                page_count: 1,
                thumbnail,
                archive,
                worker_warning: None,
                ..Default::default()
            };
        }
        return Processed {
            page_count: 1,
            worker_warning: Some(format!(
                "Worker returned HTTP {}, text was not extracted",
                resp.status()
            )),
            ..Default::default()
        };
    }
    let Ok(v) = resp.json::<Value>().await else {
        return Processed {
            page_count: 1,
            worker_warning: Some("Worker returned an invalid response".into()),
            ..Default::default()
        };
    };
    let b64 = |key: &str| {
        v.get(key)
            .and_then(|x| x.as_str())
            .and_then(|s| base64::engine::general_purpose::STANDARD.decode(s).ok())
    };

    let mut content = v
        .get("content")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let mut thumbnail = b64("thumbnail_b64");
    let mut archive = b64("archive_b64");

    if office::is_office_document(&req.filename) {
        if archive.is_none() {
            archive = office::convert_to_pdf(&req.bytes, &req.filename);
        }
        if thumbnail.is_none() {
            thumbnail = office::render_first_page_png(&req.bytes, &req.filename, 120);
        }
        if content.trim().is_empty() {
            if let Some(c) = office::extract_text_content(&req.bytes, &req.filename) {
                content = c;
            }
        }
    }

    Processed {
        content,
        page_count: v.get("page_count").and_then(|x| x.as_i64()).unwrap_or(1),
        thumbnail,
        archive,
        barcodes: v
            .get("barcodes")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default(),
        separator_pages: v
            .get("separator_pages")
            .and_then(|x| x.as_array())
            .map(|a| a.iter().filter_map(|p| p.as_i64()).collect())
            .unwrap_or_default(),
        worker_warning: None,
    }
}

async fn split_with_worker(
    state: &AppState,
    req: &ConsumeRequest,
    separator_pages: &[i64],
) -> Result<Vec<Vec<u8>>, IngestFailure> {
    let part = reqwest::multipart::Part::bytes(req.bytes.clone())
        .file_name(req.filename.clone())
        .mime_str("application/pdf")
        .unwrap();
    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text(
            "separator_pages",
            serde_json::to_string(separator_pages).unwrap(),
        )
        .text(
            "retain_separator",
            state.config.consumer.retain_split_pages.to_string(),
        );
    let resp = state
        .worker_client
        .post(format!("{}/split-pdf", state.config.worker.url))
        .multipart(form)
        .send()
        .await
        .map_err(|e| {
            IngestFailure(format!(
                "Could not split document at barcode separators: {e}"
            ))
        })?;
    if !resp.status().is_success() {
        return Err(IngestFailure(format!(
            "Splitting failed: worker returned HTTP {}",
            resp.status()
        )));
    }
    let v: Value = resp
        .json()
        .await
        .map_err(|e| IngestFailure(format!("Splitting failed: {e}")))?;
    Ok(v.get("parts")
        .and_then(|p| p.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str())
                .filter_map(|s| base64::engine::general_purpose::STANDARD.decode(s).ok())
                .collect()
        })
        .unwrap_or_default())
}

async fn pipeline(
    state: &AppState,
    task_id: &str,
    req: &ConsumeRequest,
) -> Result<(Vec<i64>, Option<String>), IngestFailure> {
    if req.bytes.is_empty() {
        return Err(IngestFailure("File is empty".into()));
    }
    let Some(mime) = detect_mime(&req.filename, &req.bytes) else {
        return Err(IngestFailure(format!(
            "Unsupported file type: '{}'",
            extension(&req.filename)
        )));
    };

    // Cheap duplicate check before spending time on OCR.
    check_duplicate(state, &sha256_hex(&req.bytes)).await?;

    notify(
        state,
        task_id,
        &req.filename,
        "WORKING",
        20,
        "Extracting text",
        None,
    );
    let processed = call_worker(state, req).await;
    let warning = processed.worker_warning.clone();

    let cfg = &state.config.consumer;
    if cfg.enable_barcodes && mime == "application/pdf" && !processed.separator_pages.is_empty() {
        notify(
            state,
            task_id,
            &req.filename,
            "WORKING",
            40,
            "Splitting at barcode separators",
            None,
        );
        let parts = split_with_worker(state, req, &processed.separator_pages).await?;
        if parts.is_empty() {
            return Err(IngestFailure(
                "Document only contained separator pages".into(),
            ));
        }
        let stem = std::path::Path::new(&req.filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("document")
            .to_string();
        let mut ids = Vec::new();
        for (n, bytes) in parts.into_iter().enumerate() {
            let mut part_req = req.clone();
            part_req.filename = format!("{stem}_{:02}.pdf", n + 1);
            part_req.bytes = bytes;
            // Title/ASN apply to the first part only; the rest derive their own.
            if n > 0 {
                part_req.title = None;
                part_req.asn = None;
            }
            let part_processed = call_worker(state, &part_req).await;
            ids.push(
                store_document(state, task_id, &part_req, "application/pdf", part_processed)
                    .await?,
            );
        }
        return Ok((ids, warning));
    }

    let id = store_document(state, task_id, req, &mime, processed).await?;
    Ok((vec![id], warning))
}

async fn check_duplicate(state: &AppState, checksum: &str) -> Result<(), IngestFailure> {
    let dup = sqlx::query(
        "SELECT id, title, deleted_at FROM documents
         WHERE (checksum = $1 OR archive_checksum = $1) AND root_document_id IS NULL LIMIT 1",
    )
    .bind(checksum)
    .fetch_optional(&state.db)
    .await?;
    if let Some(r) = dup {
        let id: i64 = r.get("id");
        let title: String = r.get("title");
        let trashed: Option<String> = r.get("deleted_at");
        return Err(IngestFailure(format!(
            "It is a duplicate of {title} (#{id}){}",
            if trashed.is_some() {
                ". Note: existing document is in the trash."
            } else {
                ""
            }
        )));
    }
    Ok(())
}

fn asn_from_barcodes(state: &AppState, barcodes: &[Value]) -> Option<i64> {
    let cfg = &state.config.consumer;
    if !cfg.enable_asn_barcode {
        return None;
    }
    let prefix = cfg.asn_barcode_prefix.to_ascii_uppercase();
    barcodes.iter().find_map(|b| {
        let v = b.get("value")?.as_str()?.trim().to_ascii_uppercase();
        v.strip_prefix(&prefix)?.trim().parse::<i64>().ok()
    })
}

async fn store_document(
    state: &AppState,
    task_id: &str,
    req: &ConsumeRequest,
    mime: &str,
    processed: Processed,
) -> Result<i64, IngestFailure> {
    let checksum = sha256_hex(&req.bytes);
    check_duplicate(state, &checksum).await?;

    // ASN: explicit value wins, otherwise a barcode on the document.
    let asn = req
        .asn
        .or_else(|| asn_from_barcodes(state, &processed.barcodes));
    if let Some(a) = asn {
        let taken: Option<i64> =
            sqlx::query_scalar("SELECT id FROM documents WHERE archive_serial_number = $1")
                .bind(a)
                .fetch_optional(&state.db)
                .await?;
        if let Some(other) = taken {
            return Err(IngestFailure(format!(
                "Not consuming {}: archive serial number {a} is already in use by document #{other}",
                req.filename
            )));
        }
    }

    notify(
        state,
        task_id,
        &req.filename,
        "WORKING",
        60,
        "Classifying",
        None,
    );

    // Classifier predictions (honoured only for objects set to "auto") + pattern matching.
    let mut predicted = Predicted::default();
    if !processed.content.is_empty() {
        if let Ok(resp) = state
            .worker_client
            .post(format!("{}/classify", state.config.worker.url))
            .json(&json!({"content": processed.content}))
            .send()
            .await
        {
            if let Ok(v) = resp.json::<Value>().await {
                predicted.correspondents = v
                    .get("correspondent_id")
                    .and_then(|x| x.as_i64())
                    .into_iter()
                    .collect();
                predicted.document_types = v
                    .get("document_type_id")
                    .and_then(|x| x.as_i64())
                    .into_iter()
                    .collect();
                predicted.tags = v
                    .get("tags")
                    .and_then(|t| t.as_array())
                    .map(|a| a.iter().filter_map(|x| x.as_i64()).collect())
                    .unwrap_or_default();
            }
        }
    }
    let matched = matching::assign(&state.db, &processed.content, &predicted).await?;

    let correspondent = req.correspondent.or(matched.correspondent);
    let document_type = req.document_type.or(matched.document_type);
    let storage_path = req.storage_path.or(matched.storage_path);

    let mut tags = req.tags.clone();
    for t in matched.tags {
        if !tags.contains(&t) {
            tags.push(t);
        }
    }

    // Scan barcodes for TAG:<name> (Barcode tag support)
    if state.config.consumer.enable_tag_barcode {
        let tag_prefix = "TAG:";
        for bc in &processed.barcodes {
            if let Some(val) = bc.get("value").and_then(|v| v.as_str()) {
                let trimmed = val.trim();
                let tag_name = if let Some(t) = trimmed.strip_prefix(tag_prefix) {
                    Some(t.trim())
                } else {
                    None
                };
                if let Some(name) = tag_name {
                    if !name.is_empty() {
                        let tag_id: Result<i64, sqlx::Error> = match sqlx::query_scalar("SELECT id FROM tags WHERE LOWER(name) = LOWER($1)")
                            .bind(name)
                            .fetch_optional(&state.db)
                            .await?
                        {
                            Some(id) => Ok(id),
                            None => {
                                sqlx::query_scalar(
                                    "INSERT INTO tags (name, color, is_inbox_tag, match, matching_algorithm, is_insensitive)
                                     VALUES ($1, '#a6cee3', 0, '', 1, 1) RETURNING id",
                                )
                                .bind(name)
                                .fetch_one(&state.db)
                                .await
                            }
                        };
                        if let Ok(tid) = tag_id {
                            if !tags.contains(&tid) {
                                tags.push(tid);
                            }
                        }
                    }
                }
            }
        }
    }

    let inbox: Vec<i64> = sqlx::query_scalar("SELECT id FROM tags WHERE is_inbox_tag = 1")
        .fetch_all(&state.db)
        .await?;
    for t in inbox {
        if !tags.contains(&t) {
            tags.push(t);
        }
    }

    let title = req
        .title
        .clone()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| {
            std::path::Path::new(&req.filename)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&req.filename)
                .to_string()
        });

    let created = req
        .created
        .clone()
        .filter(|c| !c.trim().is_empty())
        .or_else(|| dates::find_date(&processed.content))
        .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());

    notify(
        state,
        task_id,
        &req.filename,
        "WORKING",
        80,
        "Saving document",
        None,
    );

    // Files first (so a DB failure can clean them up), then the rows in one transaction.
    let uniq = &uuid::Uuid::new_v4().to_string()[..8];
    let safe_name: String = req
        .filename
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stored_name = format!("{uniq}_{safe_name}");
    let originals = state.config.storage.media_root.join("originals");
    let archive_dir = state.config.storage.media_root.join("archive");
    tokio::fs::create_dir_all(&originals).await?;
    tokio::fs::create_dir_all(&archive_dir).await?;
    let original_path = originals.join(&stored_name);
    tokio::fs::write(&original_path, &req.bytes).await?;

    let (archive_name, archive_checksum, archive_path) = match &processed.archive {
        Some(bytes) => {
            let name = format!(
                "{uniq}_{}.pdf",
                std::path::Path::new(&safe_name)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("archive")
            );
            let path = archive_dir.join(&name);
            tokio::fs::write(&path, bytes).await?;
            (Some(name), Some(sha256_hex(bytes)), Some(path))
        }
        None => (None, None, None),
    };

    let db_result: Result<i64, IngestFailure> = async {
        let mut tx = state.db.begin().await?;
        let ts = now();
        let doc_id: i64 = sqlx::query_scalar(
            "INSERT INTO documents (title, content, content_length, mime_type, checksum, archive_checksum,
                                    page_count, created, modified, added, filename, archive_filename,
                                    original_filename, archive_serial_number,
                                    correspondent_id, document_type_id, storage_path_id,
                                    root_document_id, version_index, version_label)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19) RETURNING id",
        )
        .bind(&title)
        .bind(&processed.content)
        .bind(processed.content.chars().count() as i64)
        .bind(mime)
        .bind(&checksum)
        .bind(&archive_checksum)
        .bind(processed.page_count.max(1))
        .bind(&created)
        .bind(&ts)
        .bind(&stored_name)
        .bind(&archive_name)
        .bind(&req.filename)
        .bind(asn)
        .bind(correspondent)
        .bind(document_type)
        .bind(storage_path)
        .bind(req.root_document_id)
        .bind(req.version_index)
        .bind(&req.version_label)
        .fetch_one(&mut *tx)
        .await?;

        for tag_id in &tags {
            sqlx::query("INSERT OR IGNORE INTO document_tags (document_id, tag_id) VALUES ($1, $2)")
                .bind(doc_id)
                .bind(tag_id)
                .execute(&mut *tx)
                .await?;
        }
        for (field_id, value) in &req.custom_fields {
            set_custom_field_value(&mut tx, doc_id, *field_id, value, &ts).await?;
        }
        for bc in &processed.barcodes {
            let value = bc.get("value").and_then(|v| v.as_str()).unwrap_or_default();
            if value.is_empty() {
                continue;
            }
            sqlx::query("INSERT INTO document_barcodes (document_id, page, value, format) VALUES ($1, $2, $3, $4)")
                .bind(doc_id)
                .bind(bc.get("page").and_then(|v| v.as_i64()).unwrap_or(1))
                .bind(value)
                .bind(bc.get("format").and_then(|v| v.as_str()).unwrap_or("QRCode"))
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(doc_id)
    }
    .await;

    let doc_id = match db_result {
        Ok(id) => id,
        Err(e) => {
            let _ = tokio::fs::remove_file(&original_path).await;
            if let Some(p) = archive_path {
                let _ = tokio::fs::remove_file(p).await;
            }
            return Err(e);
        }
    };

    if let Some(thumb) = &processed.thumbnail {
        let dir = state.config.storage.media_root.join("thumbnails");
        let _ = tokio::fs::create_dir_all(&dir).await;
        let _ = tokio::fs::write(dir.join(format!("{doc_id:07}.webp")), thumb).await;
    }

    crate::history::record(&state.db, doc_id, "create", json!({"created": [Value::Null, &created]})).await;

    search::reindex_document(state, doc_id).await?;

    let _ = crate::workflows::run_workflows(state, doc_id, crate::workflows::TRIGGER_DOCUMENT_ADDED, Some(&req.source)).await;

    Ok(doc_id)
}

/// Collate odd and even page single-sided scans via the worker's `/pdf/collate` endpoint.
pub async fn collate_double_sided(
    state: &AppState,
    odd_bytes: &[u8],
    even_bytes: &[u8],
) -> Result<Vec<u8>, IngestFailure> {
    use reqwest::multipart::{Form, Part};
    let form = Form::new()
        .part("odd_file", Part::bytes(odd_bytes.to_vec()).file_name("odd.pdf"))
        .part("even_file", Part::bytes(even_bytes.to_vec()).file_name("even.pdf"));


    let resp = state
        .worker_client
        .post(format!("{}/pdf/collate", state.config.worker.url))
        .multipart(form)
        .send()
        .await
        .map_err(|e| IngestFailure(format!("Worker collate failed: {e}")))?;

    if !resp.status().is_success() {
        let err = resp.text().await.unwrap_or_default();
        return Err(IngestFailure(format!("Worker collate error: {err}")));
    }

    let val = resp
        .json::<Value>()
        .await
        .map_err(|e| IngestFailure(format!("Invalid collate response: {e}")))?;

    let b64 = val
        .get("pdf_b64")
        .and_then(|v| v.as_str())
        .ok_or_else(|| IngestFailure("Worker response missing pdf_b64".into()))?;

    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| IngestFailure(format!("Base64 decode error on collated PDF: {e}")))
}


