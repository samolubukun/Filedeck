//! PDF Studio Gateway & Vault Interoperability Routes.
//!
//! Provides reverse proxying to the Python PDF worker service as well as zero-copy
//! vault document registration and direct save-back to FileDeck's vault.

use axum::{
    body::Body,
    extract::{Path as AxumPath, Query, State},
    http::{header, HeaderMap, Method, Request, Response, StatusCode},
    routing::{any, get, post},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::AppError;
use crate::ingest::ConsumeRequest;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        // Vault direct bridge
        .route(
            "/api/documents/:id/send-to-pdf-studio/",
            get(send_document_to_pdf_studio).post(send_document_to_pdf_studio),
        )
        .route(
            "/api/documents/:id/save-pdf-version/",
            post(save_document_version_from_pdf_studio),
        )
        .route(
            "/api/pdf/save-as-new-document/",
            post(save_as_new_document_from_pdf_studio),
        )
        // Proxy router catch-all for /api/pdf/*
        .route("/api/pdf/*path", any(pdf_proxy_handler))
}

/// POST /api/documents/:id/send-to-pdf-studio/
/// Resolves document path on disk and performs zero-copy registration with the worker.
pub async fn send_document_to_pdf_studio(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<i64>,
) -> Result<Json<Value>, AppError> {
    let row = sqlx::query(
        "SELECT filename, archive_filename, original_filename, title, mime_type
         FROM documents WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Document {id} not found")))?;

    let media_root = &state.config.storage.media_root;
    let filename: Option<String> = row.get("filename");
    let archive_filename: Option<String> = row.get("archive_filename");
    let original_filename: String = row
        .get::<Option<String>, _>("original_filename")
        .unwrap_or_else(|| format!("document_{id}.pdf"));
    let title: String = row.get("title");

    let file_path: PathBuf = if let Some(ref arch) = archive_filename {
        let p = media_root.join("archive").join(arch);
        if p.exists() {
            p
        } else if let Some(ref orig) = filename {
            media_root.join("originals").join(orig)
        } else {
            return Err(AppError::NotFound(format!("Document file missing on disk for id {id}")));
        }
    } else if let Some(ref orig) = filename {
        media_root.join("originals").join(orig)
    } else {
        return Err(AppError::NotFound(format!("Document file missing on disk for id {id}")));
    };

    if !file_path.exists() {
        return Err(AppError::NotFound(format!(
            "Document file not found at path {}",
            file_path.display()
        )));
    }

    let worker_url = format!("{}/api/pdf/load-vault-file", state.config.worker.url);
    let payload = json!({
        "path": file_path.to_string_lossy(),
        "name": if !title.is_empty() { format!("{}.pdf", title.trim()) } else { original_filename },
        "vault_doc_id": id
    });

    let resp = state
        .worker_client
        .post(&worker_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Worker load-vault-file failed: {e}")))?;

    if !resp.status().is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("Worker failed to load vault file: {err_text}")));
    }

    let mut result: Value = resp.json().await.map_err(|e| AppError::Internal(e.to_string()))?;
    if let Value::Object(ref mut map) = result {
        map.insert("vault_doc_id".into(), json!(id));
    }

    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
pub struct SaveVersionPayload {
    pub output_id: String,
    pub version_label: Option<String>,
}

/// POST /api/documents/:id/save-pdf-version/
/// Downloads the processed output from the worker and appends it as a new version.
pub async fn save_document_version_from_pdf_studio(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<i64>,
    Json(payload): Json<SaveVersionPayload>,
) -> Result<Json<Value>, AppError> {
    let download_url = format!("{}/api/pdf/download/{}", state.config.worker.url, payload.output_id);
    let resp = state
        .worker_client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to retrieve output from worker: {e}")))?;

    if !resp.status().is_success() {
        return Err(AppError::NotFound("Worker output file expired or not found".into()));
    }

    let bytes = resp.bytes().await.map_err(|e| AppError::Internal(e.to_string()))?.to_vec();

    // Query parent document details
    let doc = sqlx::query(
        "SELECT title, correspondent_id, document_type_id, storage_path_id FROM documents WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Document {id} not found")))?;

    let title: String = doc.get("title");
    let correspondent: Option<i64> = doc.get("correspondent_id");
    let document_type: Option<i64> = doc.get("document_type_id");
    let storage_path: Option<i64> = doc.get("storage_path_id");

    let req = ConsumeRequest {
        bytes,
        filename: format!("{}_modified.pdf", title),
        title: Some(title),
        correspondent,
        document_type,
        storage_path,
        tags: vec![],
        created: Some(Utc::now().to_rfc3339()),
        asn: None,
        custom_fields: vec![],
        source: "pdf_studio".to_string(),
        root_document_id: Some(id),
        version_index: None,
        version_label: payload.version_label,
    };

    let task_id = crate::ingest::submit(&state, req).await?;
    Ok(Json(json!({
        "status": "success",
        "task_id": task_id,
        "message": "Saved as new document version."
    })))
}

#[derive(Debug, Deserialize)]
pub struct SaveNewDocPayload {
    pub output_id: String,
    pub title: Option<String>,
    pub correspondent: Option<i64>,
    pub document_type: Option<i64>,
    pub storage_path: Option<i64>,
    pub tags: Option<Vec<i64>>,
}

/// POST /api/pdf/save-as-new-document/
/// Downloads the processed output from the worker and ingests it as a brand-new document.
pub async fn save_as_new_document_from_pdf_studio(
    State(state): State<AppState>,
    Json(payload): Json<SaveNewDocPayload>,
) -> Result<Json<Value>, AppError> {
    let download_url = format!("{}/api/pdf/download/{}", state.config.worker.url, payload.output_id);
    let resp = state
        .worker_client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to retrieve output from worker: {e}")))?;

    if !resp.status().is_success() {
        return Err(AppError::NotFound("Worker output file expired or not found".into()));
    }

    let bytes = resp.bytes().await.map_err(|e| AppError::Internal(e.to_string()))?.to_vec();
    let title = payload.title.unwrap_or_else(|| "PDF Studio Output".to_string());

    let req = ConsumeRequest {
        bytes,
        filename: format!("{}.pdf", title.trim()),
        title: Some(title),
        correspondent: payload.correspondent,
        document_type: payload.document_type,
        storage_path: payload.storage_path,
        tags: payload.tags.unwrap_or_default(),
        created: Some(Utc::now().to_rfc3339()),
        asn: None,
        custom_fields: vec![],
        source: "pdf_studio".to_string(),
        root_document_id: None,
        version_index: None,
        version_label: None,
    };

    let task_id = crate::ingest::submit(&state, req).await?;
    Ok(Json(json!({
        "status": "success",
        "task_id": task_id,
        "message": "Document submitted to vault ingestion pipeline."
    })))
}

/// Reverse proxy handler forwarding /api/pdf/* to the worker service.
pub async fn pdf_proxy_handler(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
    Query(query_params): Query<HashMap<String, String>>,
    method: Method,
    headers: HeaderMap,
    req: Request<Body>,
) -> Result<Response<Body>, AppError> {
    let target_url = format!("{}/api/pdf/{}", state.config.worker.url, path.trim_start_matches('/'));
    let mut forward_req = state
        .worker_client
        .request(method.clone(), &target_url)
        .query(&query_params);

    // Forward content-type header if present
    if let Some(content_type) = headers.get(header::CONTENT_TYPE) {
        forward_req = forward_req.header(header::CONTENT_TYPE, content_type);
    }

    // Stream body
    let body_bytes = axum::body::to_bytes(req.into_body(), 100 * 1024 * 1024)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to read request body: {e}")))?;

    if !body_bytes.is_empty() {
        forward_req = forward_req.body(body_bytes);
    }

    let worker_resp = forward_req
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Worker proxy connection error: {e}")))?;

    let status = StatusCode::from_u16(worker_resp.status().as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut builder = Response::builder().status(status);

    if let Some(ct) = worker_resp.headers().get(header::CONTENT_TYPE) {
        builder = builder.header(header::CONTENT_TYPE, ct);
    }
    if let Some(cd) = worker_resp.headers().get(header::CONTENT_DISPOSITION) {
        builder = builder.header(header::CONTENT_DISPOSITION, cd);
    }

    let resp_bytes = worker_resp
        .bytes()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    builder
        .body(Body::from(resp_bytes))
        .map_err(|e| AppError::Internal(e.to_string()))
}
