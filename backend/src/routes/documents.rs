use crate::error::AppError;
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Json, Response},
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use tokio_util::io::ReaderStream;

use crate::search::{self, filters, SearchMode};
use sqlx::{QueryBuilder, Sqlite};
use std::collections::{HashMap, HashSet};

/// Build the JSON representation of one document.
pub async fn document_json(
    state: &AppState,
    id: i64,
    truncate_content: bool,
) -> Result<Value, AppError> {
    let r = sqlx::query(
        r#"
        SELECT d.id, d.title, d.content, d.mime_type, d.checksum,
               d.archive_checksum, d.page_count, d.created, d.modified, d.added,
               d.filename, d.archive_filename, d.original_filename,
               d.archive_serial_number, d.root_document_id,
               d.correspondent_id, d.document_type_id, d.storage_path_id
        FROM documents d
        WHERE d.id = $1 AND d.deleted_at IS NULL
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Document {} not found", id)))?;

    let content: String = r.get("content");
    let content = if truncate_content && content.chars().count() > 500 {
        content.chars().take(500).collect()
    } else {
        content
    };

    let tag_ids: Vec<i64> = sqlx::query_scalar(
        "SELECT tag_id FROM document_tags WHERE document_id = $1 ORDER BY tag_id",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let notes: Vec<Value> = sqlx::query(
        "SELECT id, note, created FROM notes WHERE document_id = $1 AND deleted_at IS NULL ORDER BY created ASC",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|n| {
        json!({
            "id": n.get::<i64, _>("id"),
            "note": n.get::<String, _>("note"),
            "created": n.get::<String, _>("created"),
        })
    })
    .collect();

    let custom_fields: Vec<Value> = sqlx::query(
        "SELECT field_id, value_text, value_bool, value_url, value_int, value_float, value_date, value_select,
                value_long_text, value_monetary, value_document_ids
         FROM custom_field_instances WHERE document_id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|cf| {
        let docs: Option<String> = cf.get("value_document_ids");
        let val = cf
            .get::<Option<String>, _>("value_text")
            .map(Value::String)
            .or_else(|| cf.get::<Option<bool>, _>("value_bool").map(Value::Bool))
            .or_else(|| cf.get::<Option<String>, _>("value_url").map(Value::String))
            .or_else(|| cf.get::<Option<i64>, _>("value_int").map(|i| json!(i)))
            .or_else(|| cf.get::<Option<f64>, _>("value_float").map(|f| json!(f)))
            .or_else(|| cf.get::<Option<String>, _>("value_date").map(Value::String))
            .or_else(|| cf.get::<Option<String>, _>("value_select").map(Value::String))
            .or_else(|| cf.get::<Option<String>, _>("value_long_text").map(Value::String))
            .or_else(|| cf.get::<Option<String>, _>("value_monetary").map(Value::String))
            .or_else(|| docs.and_then(|d| serde_json::from_str::<Value>(&d).ok()))
            .unwrap_or(Value::Null);
        json!({ "field": cf.get::<i64, _>("field_id"), "value": val })
    })
    .collect();

    let root_id = r.get::<Option<i64>, _>("root_document_id").unwrap_or(id);
    let versions: Vec<Value> = sqlx::query(
        "SELECT id, added, version_index, version_label, checksum FROM documents WHERE (root_document_id = $1 OR id = $1) AND deleted_at IS NULL ORDER BY version_index ASC, id ASC",
    )
    .bind(root_id)
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|v| {
        let vid: i64 = v.get("id");
        json!({
            "id": vid,
            "added": v.get::<String, _>("added"),
            "version_index": v.get::<i64, _>("version_index"),
            "version_label": v.get::<Option<String>, _>("version_label"),
            "checksum": v.get::<String, _>("checksum"),
            "is_root": vid == root_id,
            "is_current": vid == id,
        })
    })
    .collect();

    Ok(json!({
        "id": id,
        "title": r.get::<String, _>("title"),
        "content": content,
        "correspondent": r.get::<Option<i64>, _>("correspondent_id"),
        "document_type": r.get::<Option<i64>, _>("document_type_id"),
        "storage_path": r.get::<Option<i64>, _>("storage_path_id"),
        "tags": tag_ids,
        "checksum": r.get::<String, _>("checksum"),
        "archive_checksum": r.get::<Option<String>, _>("archive_checksum"),
        "mime_type": r.get::<String, _>("mime_type"),
        "page_count": r.get::<Option<i64>, _>("page_count"),
        "created": r.get::<String, _>("created"),
        "modified": r.get::<String, _>("modified"),
        "added": r.get::<String, _>("added"),
        "original_file_name": r.get::<Option<String>, _>("original_filename"),
        "archived_file_name": r.get::<Option<String>, _>("archive_filename"),
        "download_url": format!("/api/documents/{}/download/", id),
        "thumbnail_url": format!("/api/documents/{}/thumb/", id),
        "archive_serial_number": r.get::<Option<i64>, _>("archive_serial_number"),
        "notes": notes,
        "custom_fields": custom_fields,
        "root_document": r.get::<Option<i64>, _>("root_document_id"),
        "versions": versions
    }))
}

/// `GET /api/documents/` — search and list documents.
///
/// Supports the full filter set (see `search::filters`), `ordering`, pagination and the
/// full-text parameters `text`, `title_search`, `query` and `more_like_id`.
pub async fn list_documents(
    State(state): State<AppState>,
    Query(params): Query<Vec<(String, String)>>,
) -> Result<Json<Value>, AppError> {
    let page = filters::get(&params, "page")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(1)
        .max(1);
    let page_size = filters::get(&params, "page_size")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(25)
        .clamp(1, 100_000);
    let truncate = filters::get(&params, "truncate_content")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    let ordering = filters::get(&params, "ordering").unwrap_or("").to_string();

    let text = filters::get(&params, "text").filter(|s| !s.trim().is_empty());
    let title_search = filters::get(&params, "title_search").filter(|s| !s.trim().is_empty());
    let query = filters::get(&params, "query").filter(|s| !s.trim().is_empty());
    let more_like = filters::get(&params, "more_like_id").filter(|s| !s.trim().is_empty());

    let exclusive = [
        text.is_some(),
        title_search.is_some(),
        query.is_some(),
        more_like.is_some(),
    ];
    let combinable =
        (text.is_some() && query.is_some() && !title_search.is_some() && more_like.is_none())
            || (title_search.is_some() && query.is_some() && text.is_none() && more_like.is_none());
    if exclusive.iter().filter(|b| **b).count() > 1 && !combinable {
        return Err(AppError::Validation(
            "Specify only one of text, title_search, query, or more_like_id.".into(),
        ));
    }

    // Ranked ids from the full-text index (None = no full-text constraint).
    let mut ranked: Option<Vec<search::Hit>> = None;
    let mut hl_query: Option<(String, SearchMode)> = None;
    if let Some(id) = more_like {
        let id: i64 = id
            .parse()
            .map_err(|_| AppError::Validation("Invalid more_like_id".into()))?;
        ranked = Some(search::more_like_this_ids(&state, id, search::MAX_HITS)?);
    } else {
        let primary = if let Some(t) = text {
            Some((t, SearchMode::Text))
        } else if let Some(t) = title_search {
            Some((t, SearchMode::Title))
        } else {
            query.map(|q| (q, SearchMode::Query))
        };
        if let Some((q, mode)) = primary {
            let mut hits = search::search_ids(&state, q, mode, search::MAX_HITS)?;
            if mode != SearchMode::Query {
                if let Some(fq) = query {
                    let allowed: HashSet<i64> =
                        search::search_ids(&state, fq, SearchMode::Query, search::MAX_HITS)?
                            .into_iter()
                            .map(|h| h.id)
                            .collect();
                    hits.retain(|h| allowed.contains(&h.id));
                }
            }
            hl_query = Some((q.to_string(), mode));
            ranked = Some(hits);
        }
    }

    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        "SELECT d.id FROM documents d WHERE d.deleted_at IS NULL AND d.root_document_id IS NULL",
    );
    filters::apply(&mut qb, &state.db, &params).await?;

    if let Some(ref hits) = ranked {
        let list = hits
            .iter()
            .map(|h| h.id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        qb.push(format!(
            " AND d.id IN ({})",
            if list.is_empty() {
                "NULL".to_string()
            } else {
                list
            }
        ));
    }

    let relevance_sort =
        ranked.is_some() && matches!(ordering.trim_start_matches('-'), "" | "score");
    let sql_order = if relevance_sort {
        None
    } else {
        filters::order_by(&ordering)
    };
    if ranked.is_none() || !relevance_sort {
        match sql_order {
            Some(ob) => qb.push(format!(" ORDER BY {ob}, d.id DESC")),
            None => qb.push(" ORDER BY d.created DESC, d.id DESC"),
        };
    }

    let mut all_ids: Vec<i64> = qb.build_query_scalar().fetch_all(&state.db).await?;

    if relevance_sort {
        let hits = ranked.as_ref().unwrap();
        let present: HashSet<i64> = all_ids.iter().copied().collect();
        all_ids = hits
            .iter()
            .map(|h| h.id)
            .filter(|id| present.contains(id))
            .collect();
        // Tantivy returns best-first; `ordering=score` (ascending) means worst-first.
        if ordering == "score" {
            all_ids.reverse();
        }
    }

    let count = all_ids.len() as i64;
    let offset = ((page - 1) * page_size) as usize;
    let page_ids: Vec<i64> = all_ids
        .iter()
        .skip(offset)
        .take(page_size as usize)
        .copied()
        .collect();

    let mut hit_meta: HashMap<i64, (f32, usize)> = HashMap::new();
    let mut highlights = HashMap::new();
    if let Some(ref hits) = ranked {
        let rank_of: HashMap<i64, (usize, f32)> = hits
            .iter()
            .enumerate()
            .map(|(i, h)| (h.id, (i + 1, h.score)))
            .collect();
        for id in &page_ids {
            if let Some((rank, score)) = rank_of.get(id) {
                hit_meta.insert(*id, (*score, *rank));
            }
        }
        if let Some((q, mode)) = &hl_query {
            highlights = search::highlights_for(&state, q, *mode, &page_ids);
        }
    }

    let mut results = Vec::with_capacity(page_ids.len());
    for id in &page_ids {
        let mut doc = document_json(&state, *id, truncate).await?;
        if let Some((score, rank)) = hit_meta.get(id) {
            let (h, nh) = highlights
                .get(id)
                .map(|x| (x.highlights.clone(), x.note_highlights.clone()))
                .unwrap_or_default();
            doc["__search_hit__"] = json!({
                "score": score,
                "highlights": h,
                "note_highlights": nh,
                "rank": rank
            });
        }
        results.push(doc);
    }

    let base_qs = |p: i64| {
        let mut parts: Vec<String> = params
            .iter()
            .filter(|(k, _)| k != "page")
            .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v)))
            .collect();
        parts.push(format!("page={p}"));
        format!("/api/documents/?{}", parts.join("&"))
    };
    let next = if (page * page_size) < count {
        Some(base_qs(page + 1))
    } else {
        None
    };
    let previous = if page > 1 {
        Some(base_qs(page - 1))
    } else {
        None
    };

    Ok(Json(json!({
        "count": count,
        "next": next,
        "previous": previous,
        "all": all_ids,
        "results": results
    })))
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

pub async fn get_document(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    Ok(Json(document_json(&state, id, false).await?))
}

#[derive(Deserialize)]
pub struct DocumentUpdatePayload {
    pub title: Option<String>,
    pub correspondent: Option<i64>,
    pub document_type: Option<i64>,
    pub storage_path: Option<i64>,
    pub tags: Option<Vec<i64>>,
    pub created: Option<String>,
    pub archive_serial_number: Option<i64>,
    pub custom_fields: Option<Vec<Value>>,
}

pub async fn update_document(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<DocumentUpdatePayload>,
) -> Result<Json<Value>, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut tx = state.db.begin().await?;

    if let Some(ref title) = payload.title {
        sqlx::query("UPDATE documents SET title = $1, modified = $2 WHERE id = $3")
            .bind(title)
            .bind(&now)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(corr) = payload.correspondent {
        sqlx::query("UPDATE documents SET correspondent_id = $1, modified = $2 WHERE id = $3")
            .bind(corr)
            .bind(&now)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(dt) = payload.document_type {
        sqlx::query("UPDATE documents SET document_type_id = $1, modified = $2 WHERE id = $3")
            .bind(dt)
            .bind(&now)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(sp) = payload.storage_path {
        sqlx::query("UPDATE documents SET storage_path_id = $1, modified = $2 WHERE id = $3")
            .bind(sp)
            .bind(&now)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(ref created) = payload.created {
        sqlx::query("UPDATE documents SET created = $1, modified = $2 WHERE id = $3")
            .bind(created)
            .bind(&now)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(asn) = payload.archive_serial_number {
        sqlx::query("UPDATE documents SET archive_serial_number = $1, modified = $2 WHERE id = $3")
            .bind(asn)
            .bind(&now)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }

    if let Some(ref tags) = payload.tags {
        sqlx::query("DELETE FROM document_tags WHERE document_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        for tag_id in tags {
            sqlx::query("INSERT INTO document_tags (document_id, tag_id) VALUES ($1, $2)")
                .bind(id)
                .bind(tag_id)
                .execute(&mut *tx)
                .await?;
        }
    }

    if let Some(ref cfs) = payload.custom_fields {
        sqlx::query("DELETE FROM custom_field_instances WHERE document_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        for cf in cfs {
            if let Some(fid) = cf.get("field").and_then(|v| v.as_i64()) {
                let val = cf.get("value").cloned().unwrap_or(Value::Null);
                set_custom_field_value(&mut tx, id, fid, &val, &now).await?;
            }
        }
    }

    tx.commit().await?;

    search::reindex_document(&state, id).await?;
    let _ = crate::workflows::run_workflows(&state, id, crate::workflows::TRIGGER_DOCUMENT_UPDATED, None).await;
    get_document(State(state), Path(id)).await
}

pub async fn delete_document(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE documents SET deleted_at = $1 WHERE id = $2")
        .bind(&now)
        .bind(id)
        .execute(&state.db)
        .await?;

    search::reindex_document(&state, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/documents/post_document/` — queue a file for ingestion, returns the task id.
///
/// Form fields (all optional except `document`): `title`, `created`, `correspondent`,
/// `document_type`, `storage_path`, `tags` (repeatable or JSON list), `archive_serial_number`,
/// `custom_fields` (JSON list of ids or `{id: value}` object).
pub async fn post_document(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Value>, AppError> {
    let mut req = crate::ingest::ConsumeRequest {
        filename: "upload.pdf".into(),
        source: "api_upload".into(),
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
        match name.as_str() {
            "title" => req.title = Some(text),
            "created" => req.created = Some(text.chars().take(10).collect()),
            "correspondent" => req.correspondent = text.parse().ok(),
            "document_type" => req.document_type = text.parse().ok(),
            "storage_path" => req.storage_path = text.parse().ok(),
            "archive_serial_number" => req.asn = text.parse().ok(),
            "tags" => {
                if let Ok(list) = serde_json::from_str::<Vec<i64>>(&text) {
                    req.tags.extend(list);
                } else if let Ok(single) = text.parse::<i64>() {
                    req.tags.push(single);
                }
            }
            "custom_fields" => match serde_json::from_str::<Value>(&text) {
                Ok(Value::Array(a)) => {
                    req.custom_fields.extend(
                        a.iter()
                            .filter_map(|v| v.as_i64())
                            .map(|id| (id, Value::Null)),
                    );
                }
                Ok(Value::Object(m)) => {
                    for (k, v) in m {
                        if let Ok(id) = k.parse::<i64>() {
                            req.custom_fields.push((id, v));
                        }
                    }
                }
                _ => return Err(AppError::BadRequest("Invalid custom_fields".into())),
            },
            _ => {}
        }
    }

    if req.bytes.is_empty() {
        return Err(AppError::BadRequest(
            "No file provided in 'document' field".to_string(),
        ));
    }
    if crate::ingest::detect_mime(&req.filename, &req.bytes).is_none() {
        return Err(AppError::BadRequest(format!(
            "Unsupported file type: '{}'",
            crate::ingest::extension(&req.filename)
        )));
    }

    let task_id = crate::ingest::submit(&state, req).await?;
    Ok(Json(json!(task_id)))
}
pub async fn download_document(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let doc = sqlx::query(
        "SELECT filename, archive_filename, original_filename, mime_type FROM documents WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Document {} not found", id)))?;

    let filename: Option<String> = doc.get("filename");
    let archive_filename: Option<String> = doc.get("archive_filename");
    let original_filename: Option<String> = doc.get("original_filename");
    let mime_type: String = doc.get("mime_type");

    let file_to_serve = if let Some(ref arch) = archive_filename {
        state.config.storage.media_root.join("archive").join(arch)
    } else if let Some(ref fname) = filename {
        state
            .config
            .storage
            .media_root
            .join("originals")
            .join(fname)
    } else {
        return Err(AppError::NotFound(
            "Document file not found on disk".to_string(),
        ));
    };

    if !file_to_serve.exists() {
        return Err(AppError::NotFound(
            "Document file missing from storage".to_string(),
        ));
    }

    let file = tokio::fs::File::open(&file_to_serve).await?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let display_name = original_filename.unwrap_or_else(|| format!("doc_{}.pdf", id));
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&mime_type).unwrap_or(HeaderValue::from_static("application/pdf")),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", display_name))
            .unwrap_or(HeaderValue::from_static("attachment")),
    );

    Ok((headers, body).into_response())
}

pub async fn preview_document(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let doc = sqlx::query(
        "SELECT filename, archive_filename, original_filename, mime_type FROM documents WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Document {} not found", id)))?;

    let filename: Option<String> = doc.get("filename");
    let archive_filename: Option<String> = doc.get("archive_filename");
    let mime_type: String = doc.get("mime_type");

    let file_to_serve = if let Some(ref arch) = archive_filename {
        state.config.storage.media_root.join("archive").join(arch)
    } else if let Some(ref fname) = filename {
        state
            .config
            .storage
            .media_root
            .join("originals")
            .join(fname)
    } else {
        return Err(AppError::NotFound("Document file not found".to_string()));
    };

    if !file_to_serve.exists() {
        return Err(AppError::NotFound(
            "Document file missing from disk".to_string(),
        ));
    }

    let file = tokio::fs::File::open(&file_to_serve).await?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&mime_type).unwrap_or(HeaderValue::from_static("application/pdf")),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("inline"),
    );

    Ok((headers, body).into_response())
}

pub async fn thumbnail_document(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let thumb_file = state
        .config
        .storage
        .media_root
        .join("thumbnails")
        .join(format!("{:07}.webp", id));
    if thumb_file.exists() {
        let file = tokio::fs::File::open(&thumb_file).await?;
        let stream = ReaderStream::new(file);
        let body = Body::from_stream(stream);
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/webp"));
        return Ok((headers, body).into_response());
    }

    let fallback: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 10, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0, 5, 0, 1,
        13, 10, 45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
    Ok((headers, Body::from(fallback.to_vec())).into_response())
}

#[derive(Deserialize)]
pub struct NotePayload {
    pub note: String,
}

pub async fn list_notes(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query(
        "SELECT id, note, created FROM notes WHERE document_id = $1 AND deleted_at IS NULL ORDER BY created ASC"
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let notes: Vec<Value> = rows
        .into_iter()
        .map(|n| {
            let nid: i64 = n.get("id");
            let text: String = n.get("note");
            let created: String = n.get("created");
            json!({
                "id": nid,
                "note": text,
                "created": created
            })
        })
        .collect();

    Ok(Json(json!(notes)))
}

pub async fn create_note(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<NotePayload>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    let note_id: i64 = sqlx::query_scalar(
        "INSERT INTO notes (document_id, note, created) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(id)
    .bind(&payload.note)
    .bind(&now)
    .fetch_one(&state.db)
    .await?;

    search::reindex_document(&state, id).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": note_id,
            "note": payload.note,
            "created": now
        })),
    ))
}

pub async fn delete_note(
    State(state): State<AppState>,
    Path((doc_id, note_id)): Path<(i64, i64)>,
) -> Result<StatusCode, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE notes SET deleted_at = $1 WHERE id = $2")
        .bind(&now)
        .bind(note_id)
        .execute(&state.db)
        .await?;

    search::reindex_document(&state, doc_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct BulkEditPayload {
    pub documents: Vec<i64>,
    pub method: String,
    pub parameters: Option<Value>,
}

fn param_ids(params: &Value, key: &str) -> Vec<i64> {
    params
        .get(key)
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_i64()).collect())
        .unwrap_or_default()
}

pub async fn bulk_edit(
    State(state): State<AppState>,
    Json(payload): Json<BulkEditPayload>,
) -> Result<Json<Value>, AppError> {
    let docs = &payload.documents;
    let params = payload.parameters.clone().unwrap_or(Value::Null);
    let now = chrono::Utc::now().to_rfc3339();
    let mut tx = state.db.begin().await?;

    match payload.method.as_str() {
        "set_correspondent" | "set_document_type" | "set_storage_path" => {
            let (key, col) = match payload.method.as_str() {
                "set_correspondent" => ("correspondent", "correspondent_id"),
                "set_document_type" => ("document_type", "document_type_id"),
                _ => ("storage_path", "storage_path_id"),
            };
            let value = params.get(key).and_then(|v| v.as_i64());
            for doc_id in docs {
                sqlx::query(&format!(
                    "UPDATE documents SET {col} = $1, modified = $2 WHERE id = $3"
                ))
                .bind(value)
                .bind(&now)
                .bind(doc_id)
                .execute(&mut *tx)
                .await?;
            }
        }
        "add_tag" | "remove_tag" | "modify_tags" => {
            let (add, remove) = match payload.method.as_str() {
                "add_tag" => (
                    params
                        .get("tag")
                        .and_then(|v| v.as_i64())
                        .into_iter()
                        .collect(),
                    vec![],
                ),
                "remove_tag" => (
                    vec![],
                    params
                        .get("tag")
                        .and_then(|v| v.as_i64())
                        .into_iter()
                        .collect(),
                ),
                _ => (
                    param_ids(&params, "add_tags"),
                    param_ids(&params, "remove_tags"),
                ),
            };
            for doc_id in docs {
                for tag_id in &add {
                    sqlx::query(
                        "INSERT OR IGNORE INTO document_tags (document_id, tag_id) VALUES ($1, $2)",
                    )
                    .bind(doc_id)
                    .bind(tag_id)
                    .execute(&mut *tx)
                    .await?;
                }
                for tag_id in &remove {
                    sqlx::query("DELETE FROM document_tags WHERE document_id = $1 AND tag_id = $2")
                        .bind(doc_id)
                        .bind(tag_id)
                        .execute(&mut *tx)
                        .await?;
                }
                sqlx::query("UPDATE documents SET modified = $1 WHERE id = $2")
                    .bind(&now)
                    .bind(doc_id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        "modify_custom_fields" => {
            let remove = param_ids(&params, "remove_fields");
            for doc_id in docs {
                // add_fields is either [id, ...] or {"id": value, ...} (accepts both)
                match params.get("add_fields") {
                    Some(Value::Array(a)) => {
                        for f in a.iter().filter_map(|x| x.as_i64()) {
                            set_custom_field_value(&mut tx, *doc_id, f, &Value::Null, &now).await?;
                        }
                    }
                    Some(Value::Object(m)) => {
                        for (k, v) in m {
                            if let Ok(f) = k.parse::<i64>() {
                                set_custom_field_value(&mut tx, *doc_id, f, v, &now).await?;
                            }
                        }
                    }
                    _ => {}
                }
                for f in &remove {
                    sqlx::query("DELETE FROM custom_field_instances WHERE document_id = $1 AND field_id = $2")
                        .bind(doc_id)
                        .bind(f)
                        .execute(&mut *tx)
                        .await?;
                }
                sqlx::query("UPDATE documents SET modified = $1 WHERE id = $2")
                    .bind(&now)
                    .bind(doc_id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        "delete" => {
            for doc_id in docs {
                sqlx::query("UPDATE documents SET deleted_at = $1 WHERE id = $2")
                    .bind(&now)
                    .bind(doc_id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        other => {
            // Release the transaction first: the PDF operations use their own connections.
            drop(tx);
            return match super::documents_ops::bulk_dispatch(&state, other, docs, &params).await? {
                Some(v) => Ok(Json(v)),
                None => Err(AppError::Validation(format!(
                    "Unsupported bulk edit method '{other}'"
                ))),
            };
        }
    }
    tx.commit().await?;

    search::reindex_documents(&state, docs).await?;

    Ok(Json(json!({
        "result": "OK",
        "affected": docs.len()
    })))
}

/// Create (or update) a custom field instance on a document, storing the value in the
/// column that matches the field's data type.
pub async fn set_custom_field_value(
    tx: &mut sqlx::SqliteConnection,
    doc_id: i64,
    field_id: i64,
    value: &Value,
    now: &str,
) -> Result<(), AppError> {
    let dtype: String = sqlx::query_scalar("SELECT data_type FROM custom_fields WHERE id = $1")
        .bind(field_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Custom field {field_id} not found")))?;

    sqlx::query("DELETE FROM custom_field_instances WHERE document_id = $1 AND field_id = $2")
        .bind(doc_id)
        .bind(field_id)
        .execute(&mut *tx)
        .await?;

    let col = match dtype.as_str() {
        "string" => "value_text",
        "longtext" => "value_long_text",
        "url" => "value_url",
        "date" => "value_date",
        "boolean" => "value_bool",
        "integer" => "value_int",
        "float" => "value_float",
        "monetary" => "value_monetary",
        "documentlink" => "value_document_ids",
        "select" => "value_select",
        _ => "value_text",
    };
    let sql = format!(
        "INSERT INTO custom_field_instances (document_id, field_id, created, {col}) VALUES ($1, $2, $3, $4)"
    );
    let q = sqlx::query(&sql).bind(doc_id).bind(field_id).bind(now);
    let q = match value {
        Value::Null => q.bind(None::<String>),
        Value::Bool(b) => q.bind(*b),
        Value::Number(n) if n.is_i64() => q.bind(n.as_i64().unwrap()),
        Value::Number(n) => q.bind(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => q.bind(s.clone()),
        other => q.bind(other.to_string()),
    };
    q.execute(&mut *tx).await?;
    Ok(())
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct BulkDownloadPayload {
    pub documents: Vec<i64>,
    pub content: Option<String>,
}

pub async fn bulk_download(
    State(state): State<AppState>,
    Json(payload): Json<BulkDownloadPayload>,
) -> Result<Response, AppError> {
    use std::io::Write;
    let mut zip_buffer = std::io::Cursor::new(Vec::new());
    {
        let mut zip_writer = zip::ZipWriter::new(&mut zip_buffer);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();
        for doc_id in payload.documents {
            let doc = sqlx::query(
                "SELECT filename, archive_filename, original_filename FROM documents WHERE id = $1",
            )
            .bind(doc_id)
            .fetch_optional(&state.db)
            .await?;

            if let Some(d) = doc {
                let filename: Option<String> = d.get("filename");
                let archive_filename: Option<String> = d.get("archive_filename");
                let original_filename: Option<String> = d.get("original_filename");

                let file_path = if let Some(arch) = archive_filename {
                    state.config.storage.media_root.join("archive").join(arch)
                } else if let Some(fname) = filename {
                    state
                        .config
                        .storage
                        .media_root
                        .join("originals")
                        .join(fname)
                } else {
                    continue;
                };

                if file_path.exists() {
                    let mut name = original_filename.unwrap_or_else(|| format!("{}.pdf", doc_id));
                    if seen_names.contains(&name) {
                        name = format!("{}_{}", doc_id, name);
                    }
                    seen_names.insert(name.clone());
                    let bytes = tokio::fs::read(&file_path).await?;
                    zip_writer
                        .start_file(name, options)
                        .map_err(|e| AppError::Internal(e.to_string()))?;
                    zip_writer
                        .write_all(&bytes)
                        .map_err(|e| AppError::Internal(e.to_string()))?;
                }
            }
        }
        zip_writer
            .finish()
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }

    let bytes = zip_buffer.into_inner();
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/zip"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"documents.zip\""),
    );

    Ok((headers, Body::from(bytes)).into_response())
}

pub async fn get_selection_data(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let corr_rows = sqlx::query(
        "SELECT correspondent_id as id, COUNT(*) as doc_count FROM documents WHERE correspondent_id IS NOT NULL AND deleted_at IS NULL GROUP BY correspondent_id"
    )
    .fetch_all(&state.db)
    .await?;

    let dt_rows = sqlx::query(
        "SELECT document_type_id as id, COUNT(*) as doc_count FROM documents WHERE document_type_id IS NOT NULL AND deleted_at IS NULL GROUP BY document_type_id"
    )
    .fetch_all(&state.db)
    .await?;

    let tag_rows = sqlx::query(
        "SELECT dt.tag_id as id, COUNT(*) as doc_count FROM document_tags dt JOIN documents d ON dt.document_id = d.id WHERE d.deleted_at IS NULL GROUP BY dt.tag_id"
    )
    .fetch_all(&state.db)
    .await?;

    let corr_items: Vec<Value> = corr_rows
        .into_iter()
        .map(|r| {
            let cid: i64 = r.get("id");
            let cnt: i64 = r.get("doc_count");
            json!({"id": cid, "document_count": cnt})
        })
        .collect();

    let dt_items: Vec<Value> = dt_rows
        .into_iter()
        .map(|r| {
            let did: i64 = r.get("id");
            let cnt: i64 = r.get("doc_count");
            json!({"id": did, "document_count": cnt})
        })
        .collect();

    let tag_items: Vec<Value> = tag_rows
        .into_iter()
        .map(|r| {
            let tid: i64 = r.get("id");
            let cnt: i64 = r.get("doc_count");
            json!({"id": tid, "document_count": cnt})
        })
        .collect();

    Ok(Json(json!({
        "selected_storage_paths": [],
        "selected_correspondents": corr_items,
        "selected_tags": tag_items,
        "selected_document_types": dt_items,
        "selected_custom_fields": []
    })))
}

/// `GET /api/documents/:id/history/` — audit log for a document.
pub async fn get_document_history(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let exists = sqlx::query_scalar::<_, i64>("SELECT 1 FROM documents WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .is_some();

    if !exists {
        return Err(AppError::NotFound(format!("Document {id} not found")));
    }

    let rows = sqlx::query(
        "SELECT id, timestamp, action, changes FROM document_history WHERE document_id = $1 ORDER BY timestamp DESC, id DESC",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let entries: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            let changes_raw: String = r.get("changes");
            let changes_val = serde_json::from_str::<Value>(&changes_raw).unwrap_or(json!({}));
            json!({
                "id": r.get::<i64, _>("id"),
                "actor": {
                    "id": 1,
                    "username": "local",
                    "first_name": "Local",
                    "last_name": "User"
                },
                "action": r.get::<String, _>("action"),
                "changes": changes_val,
                "timestamp": r.get::<String, _>("timestamp")
            })
        })
        .collect();

    Ok(Json(json!(entries)))
}

