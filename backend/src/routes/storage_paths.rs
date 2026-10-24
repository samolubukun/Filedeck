use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use crate::error::AppError;
use crate::routes::correspondents::slugify;
use crate::state::AppState;

#[derive(Serialize, Deserialize)]
pub struct StoragePathPayload {
    pub name: String,
    pub path: String,
    #[serde(rename = "match", alias = "match_pattern")]
    pub match_pattern: Option<String>,
    pub matching_algorithm: Option<i64>,
    pub is_insensitive: Option<bool>,
}

pub async fn list_storage_paths(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT sp.id, sp.name, sp.path, sp.match, sp.matching_algorithm, sp.is_insensitive,
               COUNT(d.id) as document_count
        FROM storage_paths sp
        LEFT JOIN documents d ON d.storage_path_id = sp.id AND d.deleted_at IS NULL
        GROUP BY sp.id
        ORDER BY sp.name COLLATE NOCASE ASC
        "#
    )
    .fetch_all(&state.db)
    .await?;

    let results: Vec<Value> = rows.into_iter().map(|r| {
        let id: i64 = r.get("id");
        let name: String = r.get("name");
        let path: String = r.get("path");
        let match_pattern: String = r.get("match");
        let algo: i64 = r.get("matching_algorithm");
        let is_insensitive: bool = r.get("is_insensitive");
        let count: i64 = r.get("document_count");
        json!({
            "id": id,
            "slug": slugify(&name),
            "name": name,
            "path": path,
            "match": match_pattern,
            "matching_algorithm": algo,
            "is_insensitive": is_insensitive,
            "document_count": count
        })
    }).collect();

    Ok(Json(json!({
        "count": results.len(),
        "results": results
    })))
}

pub async fn create_storage_path(
    State(state): State<AppState>,
    Json(payload): Json<StoragePathPayload>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let match_val = payload.match_pattern.unwrap_or_default();
    let algo = payload.matching_algorithm.unwrap_or(1);
    let insensitive = payload.is_insensitive.unwrap_or(true);

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO storage_paths (name, path, match, matching_algorithm, is_insensitive)
         VALUES ($1, $2, $3, $4, $5) RETURNING id"
    )
    .bind(&payload.name)
    .bind(&payload.path)
    .bind(&match_val)
    .bind(algo)
    .bind(insensitive)
    .fetch_one(&state.db)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": id,
            "slug": slugify(&payload.name),
            "name": payload.name,
            "path": payload.path,
            "match": match_val,
            "matching_algorithm": algo,
            "is_insensitive": insensitive,
            "document_count": 0
        })),
    ))
}

pub async fn get_storage_path(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let r = sqlx::query(
        r#"
        SELECT sp.id, sp.name, sp.path, sp.match, sp.matching_algorithm, sp.is_insensitive,
               COUNT(d.id) as document_count
        FROM storage_paths sp
        LEFT JOIN documents d ON d.storage_path_id = sp.id AND d.deleted_at IS NULL
        WHERE sp.id = $1
        GROUP BY sp.id
        "#
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Storage path {} not found", id)))?;

    let name: String = r.get("name");
    let path: String = r.get("path");
    let match_pattern: String = r.get("match");
    let algo: i64 = r.get("matching_algorithm");
    let is_insensitive: bool = r.get("is_insensitive");
    let count: i64 = r.get("document_count");

    Ok(Json(json!({
        "id": id,
        "slug": slugify(&name),
        "name": name,
        "path": path,
        "match": match_pattern,
        "matching_algorithm": algo,
        "is_insensitive": is_insensitive,
        "document_count": count
    })))
}

pub async fn update_storage_path(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<StoragePathPayload>,
) -> Result<Json<Value>, AppError> {
    let current = get_storage_path(State(state.clone()), Path(id)).await?.0;

    let path_val = payload.path;
    let match_val = payload.match_pattern.unwrap_or_else(|| current["match"].as_str().unwrap_or("").to_string());
    let algo = payload.matching_algorithm.unwrap_or_else(|| current["matching_algorithm"].as_i64().unwrap_or(1));
    let insensitive = payload.is_insensitive.unwrap_or_else(|| current["is_insensitive"].as_bool().unwrap_or(true));

    sqlx::query(
        "UPDATE storage_paths SET name = $1, path = $2, match = $3, matching_algorithm = $4, is_insensitive = $5 WHERE id = $6"
    )
    .bind(&payload.name)
    .bind(&path_val)
    .bind(&match_val)
    .bind(algo)
    .bind(insensitive)
    .bind(id)
    .execute(&state.db)
    .await?;

    crate::search::reindex_all_background(&state);
    get_storage_path(State(state), Path(id)).await
}

pub async fn delete_storage_path(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    sqlx::query("DELETE FROM storage_paths WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    crate::search::reindex_all_background(&state);
    Ok(StatusCode::NO_CONTENT)
}
