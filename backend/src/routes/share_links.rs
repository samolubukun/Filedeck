use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use tokio_util::io::ReaderStream;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Serialize, Deserialize)]
pub struct ShareLinkPayload {
    pub document: i64,
    pub file_version: Option<String>,
    pub expiration: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ShareLinkBundlePayload {
    pub documents: Vec<i64>,
    pub file_version: Option<String>,
    pub expiration: Option<String>,
}

pub async fn list_share_links(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query(
        "SELECT id, document_id, slug, expiration, file_version, created FROM share_links WHERE deleted_at IS NULL ORDER BY created DESC"
    )
    .fetch_all(&state.db)
    .await?;

    let results: Vec<Value> = rows.into_iter().map(|r| {
        let id: i64 = r.get("id");
        let document_id: i64 = r.get("document_id");
        let slug: String = r.get("slug");
        let expiration: Option<String> = r.get("expiration");
        let file_version: String = r.get("file_version");
        let created: String = r.get("created");
        json!({
            "id": id,
            "document": document_id,
            "slug": slug,
            "expiration": expiration,
            "file_version": file_version,
            "created": created
        })
    }).collect();

    Ok(Json(json!({
        "count": results.len(),
        "results": results
    })))
}

pub async fn create_share_link(
    State(state): State<AppState>,
    Json(payload): Json<ShareLinkPayload>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let slug = uuid::Uuid::new_v4().to_string().replace('-', "")[..16].to_string();
    let version = payload.file_version.unwrap_or_else(|| "archive".to_string());
    let now = chrono::Utc::now().to_rfc3339();

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO share_links (document_id, slug, expiration, file_version, created)
         VALUES ($1, $2, $3, $4, $5) RETURNING id"
    )
    .bind(payload.document)
    .bind(&slug)
    .bind(&payload.expiration)
    .bind(&version)
    .bind(&now)
    .fetch_one(&state.db)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": id,
            "document": payload.document,
            "slug": slug,
            "expiration": payload.expiration,
            "file_version": version,
            "created": now
        })),
    ))
}

pub async fn delete_share_link(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE share_links SET deleted_at = $1 WHERE id = $2")
        .bind(&now)
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_share_link_bundles(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query(
        "SELECT id, slug, expiration, file_version, status, size_bytes, file_path, built_at
         FROM share_link_bundles ORDER BY id DESC"
    )
    .fetch_all(&state.db)
    .await?;

    let mut results = Vec::new();
    for r in rows {
        let id: i64 = r.get("id");
        let slug: String = r.get("slug");
        let expiration: Option<String> = r.get("expiration");
        let file_version: String = r.get("file_version");
        let status: String = r.get("status");
        let size_bytes: Option<i64> = r.get("size_bytes");
        let built_at: Option<String> = r.get("built_at");

        let doc_rows = sqlx::query(
            "SELECT document_id FROM share_link_bundle_documents WHERE bundle_id = $1"
        )
        .bind(id)
        .fetch_all(&state.db)
        .await?;

        let docs: Vec<i64> = doc_rows.into_iter().map(|dr| dr.get("document_id")).collect();

        results.push(json!({
            "id": id,
            "slug": slug,
            "expiration": expiration,
            "file_version": file_version,
            "status": status,
            "size_bytes": size_bytes,
            "built_at": built_at,
            "documents": docs
        }));
    }

    Ok(Json(json!({
        "count": results.len(),
        "results": results
    })))
}

pub async fn create_share_link_bundle(
    State(state): State<AppState>,
    Json(payload): Json<ShareLinkBundlePayload>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let slug = uuid::Uuid::new_v4().to_string().replace('-', "")[..16].to_string();
    let version = payload.file_version.unwrap_or_else(|| "archive".to_string());

    let mut tx = state.db.begin().await?;

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO share_link_bundles (slug, expiration, file_version, status)
         VALUES ($1, $2, $3, 'pending') RETURNING id"
    )
    .bind(&slug)
    .bind(&payload.expiration)
    .bind(&version)
    .fetch_one(&mut *tx)
    .await?;

    for doc_id in &payload.documents {
        sqlx::query(
            "INSERT INTO share_link_bundle_documents (bundle_id, document_id) VALUES ($1, $2)"
        )
        .bind(id)
        .bind(doc_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": id,
            "slug": slug,
            "expiration": payload.expiration,
            "file_version": version,
            "status": "pending",
            "documents": payload.documents
        })),
    ))
}

pub async fn delete_share_link_bundle(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    sqlx::query("DELETE FROM share_link_bundles WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn view_shared_link(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Response, AppError> {
    let link_row = sqlx::query(
        "SELECT document_id, file_version, expiration FROM share_links WHERE slug = $1 AND deleted_at IS NULL"
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Shared link not found or expired".to_string()))?;

    let doc_id: i64 = link_row.get("document_id");
    let file_version: String = link_row.get("file_version");
    let expiration: Option<String> = link_row.get("expiration");

    if let Some(exp) = expiration {
        if let Ok(exp_time) = chrono::DateTime::parse_from_rfc3339(&exp) {
            if chrono::Utc::now() > exp_time {
                return Err(AppError::NotFound("Shared link expired".to_string()));
            }
        }
    }

    let doc = sqlx::query(
        "SELECT filename, archive_filename, original_filename, mime_type FROM documents WHERE id = $1"
    )
    .bind(doc_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Document not found".to_string()))?;

    let filename: Option<String> = doc.get("filename");
    let archive_filename: Option<String> = doc.get("archive_filename");
    let original_filename: Option<String> = doc.get("original_filename");
    let mime_type: String = doc.get("mime_type");

    let file_to_serve = if file_version == "archive" && archive_filename.is_some() {
        state.config.storage.media_root.join("archive").join(archive_filename.unwrap())
    } else if let Some(fname) = filename {
        state.config.storage.media_root.join("originals").join(fname)
    } else {
        return Err(AppError::NotFound("File not found on disk".to_string()));
    };

    if !file_to_serve.exists() {
        return Err(AppError::NotFound("File not found on storage".to_string()));
    }

    let file = tokio::fs::File::open(&file_to_serve).await?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let display_name = original_filename.unwrap_or_else(|| "document.pdf".to_string());
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_str(&mime_type).unwrap_or(HeaderValue::from_static("application/pdf")));
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("inline; filename=\"{}\"", display_name)).unwrap_or(HeaderValue::from_static("inline")),
    );

    Ok((headers, body).into_response())
}
