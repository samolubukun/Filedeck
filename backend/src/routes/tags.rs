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
pub struct TagPayload {
    pub name: Option<String>,
    pub color: Option<String>,
    pub is_inbox_tag: Option<bool>,
    pub parent: Option<i64>,
    #[serde(rename = "match", alias = "match_pattern")]
    pub match_pattern: Option<String>,
    pub matching_algorithm: Option<i64>,
    pub is_insensitive: Option<bool>,
}

pub async fn list_tags(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT t.id, t.name, t.color, t.is_inbox_tag, t.parent_id,
               t.match, t.matching_algorithm, t.is_insensitive,
               COUNT(dt.document_id) as document_count
        FROM tags t
        LEFT JOIN document_tags dt ON dt.tag_id = t.id
        LEFT JOIN documents d ON d.id = dt.document_id AND d.deleted_at IS NULL
        GROUP BY t.id
        ORDER BY t.name COLLATE NOCASE ASC
        "#
    )
    .fetch_all(&state.db)
    .await?;

    let results: Vec<Value> = rows.into_iter().map(|r| {
        let id: i64 = r.get("id");
        let name: String = r.get("name");
        let color: String = r.get("color");
        let is_inbox: bool = r.get("is_inbox_tag");
        let parent: Option<i64> = r.get("parent_id");
        let match_pattern: String = r.get("match");
        let algo: i64 = r.get("matching_algorithm");
        let is_insensitive: bool = r.get("is_insensitive");
        let count: i64 = r.get("document_count");
        json!({
            "id": id,
            "slug": slugify(&name),
            "name": name,
            "color": color,
            "is_inbox_tag": is_inbox,
            "parent": parent,
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

pub async fn create_tag(
    State(state): State<AppState>,
    Json(payload): Json<TagPayload>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let name = payload.name.ok_or_else(|| AppError::BadRequest("Field 'name' is required".into()))?;
    let color = payload.color.unwrap_or_else(|| "#a6cee3".to_string());
    let is_inbox = payload.is_inbox_tag.unwrap_or(false);
    let match_val = payload.match_pattern.unwrap_or_default();
    let algo = payload.matching_algorithm.unwrap_or(1);
    let insensitive = payload.is_insensitive.unwrap_or(true);

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO tags (name, color, is_inbox_tag, parent_id, match, matching_algorithm, is_insensitive)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id"
    )
    .bind(&name)
    .bind(&color)
    .bind(is_inbox)
    .bind(payload.parent)
    .bind(&match_val)
    .bind(algo)
    .bind(insensitive)
    .fetch_one(&state.db)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": id,
            "slug": slugify(&name),
            "name": name,
            "color": color,
            "is_inbox_tag": is_inbox,
            "parent": payload.parent,
            "match": match_val,
            "matching_algorithm": algo,
            "is_insensitive": insensitive,
            "document_count": 0
        })),
    ))
}

pub async fn get_tag(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let r = sqlx::query(
        r#"
        SELECT t.id, t.name, t.color, t.is_inbox_tag, t.parent_id,
               t.match, t.matching_algorithm, t.is_insensitive,
               COUNT(dt.document_id) as document_count
        FROM tags t
        LEFT JOIN document_tags dt ON dt.tag_id = t.id
        LEFT JOIN documents d ON d.id = dt.document_id AND d.deleted_at IS NULL
        WHERE t.id = $1
        GROUP BY t.id
        "#
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Tag {} not found", id)))?;

    let name: String = r.get("name");
    let color: String = r.get("color");
    let is_inbox: bool = r.get("is_inbox_tag");
    let parent: Option<i64> = r.get("parent_id");
    let match_pattern: String = r.get("match");
    let algo: i64 = r.get("matching_algorithm");
    let is_insensitive: bool = r.get("is_insensitive");
    let count: i64 = r.get("document_count");

    Ok(Json(json!({
        "id": id,
        "slug": slugify(&name),
        "name": name,
        "color": color,
        "is_inbox_tag": is_inbox,
        "parent": parent,
        "match": match_pattern,
        "matching_algorithm": algo,
        "is_insensitive": is_insensitive,
        "document_count": count
    })))
}

pub async fn update_tag(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<TagPayload>,
) -> Result<Json<Value>, AppError> {
    let current = get_tag(State(state.clone()), Path(id)).await?.0;

    let name = payload.name.unwrap_or_else(|| current["name"].as_str().unwrap_or("").to_string());
    let color = payload.color.unwrap_or_else(|| current["color"].as_str().unwrap_or("#a6cee3").to_string());
    let is_inbox = payload.is_inbox_tag.unwrap_or_else(|| current["is_inbox_tag"].as_bool().unwrap_or(false));
    let parent = payload.parent.or_else(|| current["parent"].as_i64());
    let match_val = payload.match_pattern.unwrap_or_else(|| current["match"].as_str().unwrap_or("").to_string());
    let algo = payload.matching_algorithm.unwrap_or_else(|| current["matching_algorithm"].as_i64().unwrap_or(1));
    let insensitive = payload.is_insensitive.unwrap_or_else(|| current["is_insensitive"].as_bool().unwrap_or(true));

    if let Some(pid) = parent {
        if pid == id {
            return Err(AppError::BadRequest("A tag cannot be its own parent".to_string()));
        }
    }

    sqlx::query(
        "UPDATE tags SET name = $1, color = $2, is_inbox_tag = $3, parent_id = $4,
                         match = $5, matching_algorithm = $6, is_insensitive = $7 WHERE id = $8"
    )
    .bind(&name)
    .bind(&color)
    .bind(is_inbox)
    .bind(parent)
    .bind(&match_val)
    .bind(algo)
    .bind(insensitive)
    .bind(id)
    .execute(&state.db)
    .await?;

    crate::search::reindex_all_background(&state);
    get_tag(State(state), Path(id)).await
}

pub async fn delete_tag(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    sqlx::query("DELETE FROM tags WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    crate::search::reindex_all_background(&state);
    Ok(StatusCode::NO_CONTENT)
}
