use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Serialize, Deserialize)]
pub struct CustomFieldPayload {
    pub name: String,
    pub data_type: String,
    pub extra_data: Option<Value>,
}

pub async fn list_custom_fields(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query(
        "SELECT id, name, data_type, extra_data, created FROM custom_fields ORDER BY created ASC"
    )
    .fetch_all(&state.db)
    .await?;

    let results: Vec<Value> = rows.into_iter().map(|r| {
        let id: i64 = r.get("id");
        let name: String = r.get("name");
        let data_type: String = r.get("data_type");
        let extra_str: Option<String> = r.get("extra_data");
        let created: String = r.get("created");
        let extra: Option<Value> = extra_str.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok());
        json!({
            "id": id,
            "name": name,
            "data_type": data_type,
            "extra_data": extra,
            "created": created
        })
    }).collect();

    Ok(Json(json!({
        "count": results.len(),
        "results": results
    })))
}

pub async fn create_custom_field(
    State(state): State<AppState>,
    Json(payload): Json<CustomFieldPayload>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    let extra_str = payload.extra_data.map(|v| v.to_string());

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO custom_fields (name, data_type, extra_data, created)
         VALUES ($1, $2, $3, $4) RETURNING id"
    )
    .bind(&payload.name)
    .bind(&payload.data_type)
    .bind(extra_str)
    .bind(&now)
    .fetch_one(&state.db)
    .await?;

    get_custom_field(State(state), Path(id)).await.map(|j| (StatusCode::CREATED, j))
}

pub async fn get_custom_field(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let r = sqlx::query(
        "SELECT id, name, data_type, extra_data, created FROM custom_fields WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Custom field {} not found", id)))?;

    let name: String = r.get("name");
    let data_type: String = r.get("data_type");
    let extra_str: Option<String> = r.get("extra_data");
    let created: String = r.get("created");
    let extra: Option<Value> = extra_str.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok());

    Ok(Json(json!({
        "id": id,
        "name": name,
        "data_type": data_type,
        "extra_data": extra,
        "created": created
    })))
}

pub async fn update_custom_field(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<CustomFieldPayload>,
) -> Result<Json<Value>, AppError> {
    let extra_str = payload.extra_data.map(|v| v.to_string());

    sqlx::query(
        "UPDATE custom_fields SET name = $1, extra_data = $2 WHERE id = $3"
    )
    .bind(&payload.name)
    .bind(extra_str)
    .bind(id)
    .execute(&state.db)
    .await?;

    crate::search::reindex_all_background(&state);
    get_custom_field(State(state), Path(id)).await
}

pub async fn delete_custom_field(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    sqlx::query("DELETE FROM custom_fields WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    crate::search::reindex_all_background(&state);
    Ok(StatusCode::NO_CONTENT)
}
