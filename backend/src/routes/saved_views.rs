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
pub struct FilterRulePayload {
    pub rule_type: i64,
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SavedViewPayload {
    pub name: String,
    pub icon: Option<String>,
    pub sort_field: Option<String>,
    pub sort_reverse: Option<bool>,
    pub page_size: Option<i64>,
    pub display_mode: Option<String>,
    pub display_fields: Option<Value>,
    pub filter_rules: Option<Vec<FilterRulePayload>>,
}

pub async fn list_saved_views(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query(
        "SELECT id, name, icon, sort_field, sort_reverse, page_size, display_mode, display_fields
         FROM saved_views ORDER BY name ASC"
    )
    .fetch_all(&state.db)
    .await?;

    let mut results = Vec::new();
    for r in rows {
        let id: i64 = r.get("id");
        let name: String = r.get("name");
        let icon: String = r.get("icon");
        let sort_field: Option<String> = r.get("sort_field");
        let sort_reverse: bool = r.get("sort_reverse");
        let page_size: Option<i64> = r.get("page_size");
        let display_mode: Option<String> = r.get("display_mode");
        let display_fields_str: Option<String> = r.get("display_fields");

        let rules = sqlx::query(
            "SELECT rule_type, value FROM saved_view_filter_rules WHERE saved_view_id = $1"
        )
        .bind(id)
        .fetch_all(&state.db)
        .await?;

        let filter_rules: Vec<Value> = rules.into_iter().map(|rule| {
            let rt: i64 = rule.get("rule_type");
            let val: Option<String> = rule.get("value");
            json!({
                "rule_type": rt,
                "value": val
            })
        }).collect();

        let display_fields: Option<Value> = display_fields_str.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok());

        results.push(json!({
            "id": id,
            "name": name,
            "icon": icon,
            "sort_field": sort_field,
            "sort_reverse": sort_reverse,
            "page_size": page_size,
            "display_mode": display_mode,
            "display_fields": display_fields,
            "filter_rules": filter_rules
        }));
    }

    Ok(Json(json!({
        "count": results.len(),
        "results": results
    })))
}

pub async fn create_saved_view(
    State(state): State<AppState>,
    Json(payload): Json<SavedViewPayload>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let icon = payload.icon.unwrap_or_else(|| "funnel".to_string());
    let sort_reverse = payload.sort_reverse.unwrap_or(false);
    let display_fields_str = payload.display_fields.map(|v| v.to_string());

    let mut tx = state.db.begin().await?;

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO saved_views (name, icon, sort_field, sort_reverse, page_size, display_mode, display_fields)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id"
    )
    .bind(&payload.name)
    .bind(&icon)
    .bind(&payload.sort_field)
    .bind(sort_reverse)
    .bind(payload.page_size)
    .bind(&payload.display_mode)
    .bind(display_fields_str)
    .fetch_one(&mut *tx)
    .await?;

    if let Some(rules) = payload.filter_rules {
        for rule in rules {
            sqlx::query(
                "INSERT INTO saved_view_filter_rules (saved_view_id, rule_type, value) VALUES ($1, $2, $3)"
            )
            .bind(id)
            .bind(rule.rule_type)
            .bind(rule.value)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    get_saved_view(State(state), Path(id)).await.map(|j| (StatusCode::CREATED, j))
}

pub async fn get_saved_view(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let r = sqlx::query(
        "SELECT id, name, icon, sort_field, sort_reverse, page_size, display_mode, display_fields
         FROM saved_views WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Saved view {} not found", id)))?;

    let name: String = r.get("name");
    let icon: String = r.get("icon");
    let sort_field: Option<String> = r.get("sort_field");
    let sort_reverse: bool = r.get("sort_reverse");
    let page_size: Option<i64> = r.get("page_size");
    let display_mode: Option<String> = r.get("display_mode");
    let display_fields_str: Option<String> = r.get("display_fields");

    let rules = sqlx::query(
        "SELECT rule_type, value FROM saved_view_filter_rules WHERE saved_view_id = $1"
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let filter_rules: Vec<Value> = rules.into_iter().map(|rule| {
        let rt: i64 = rule.get("rule_type");
        let val: Option<String> = rule.get("value");
        json!({
            "rule_type": rt,
            "value": val
        })
    }).collect();

    let display_fields: Option<Value> = display_fields_str.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok());

    Ok(Json(json!({
        "id": id,
        "name": name,
        "icon": icon,
        "sort_field": sort_field,
        "sort_reverse": sort_reverse,
        "page_size": page_size,
        "display_mode": display_mode,
        "display_fields": display_fields,
        "filter_rules": filter_rules
    })))
}

pub async fn update_saved_view(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<SavedViewPayload>,
) -> Result<Json<Value>, AppError> {
    let icon = payload.icon.unwrap_or_else(|| "funnel".to_string());
    let sort_reverse = payload.sort_reverse.unwrap_or(false);
    let display_fields_str = payload.display_fields.map(|v| v.to_string());

    let mut tx = state.db.begin().await?;

    sqlx::query(
        "UPDATE saved_views SET name = $1, icon = $2, sort_field = $3, sort_reverse = $4,
                                page_size = $5, display_mode = $6, display_fields = $7 WHERE id = $8"
    )
    .bind(&payload.name)
    .bind(&icon)
    .bind(&payload.sort_field)
    .bind(sort_reverse)
    .bind(payload.page_size)
    .bind(&payload.display_mode)
    .bind(display_fields_str)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    if let Some(rules) = payload.filter_rules {
        sqlx::query("DELETE FROM saved_view_filter_rules WHERE saved_view_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        for rule in rules {
            sqlx::query(
                "INSERT INTO saved_view_filter_rules (saved_view_id, rule_type, value) VALUES ($1, $2, $3)"
            )
            .bind(id)
            .bind(rule.rule_type)
            .bind(rule.value)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    get_saved_view(State(state), Path(id)).await
}

pub async fn delete_saved_view(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    sqlx::query("DELETE FROM saved_views WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
