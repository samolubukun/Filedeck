use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;

use crate::error::AppError;
use crate::search::{self, SearchMode};
use crate::state::AppState;

const OBJECT_LIMIT: i64 = 3;
const MAX_QUERY_LENGTH: usize = 512;

#[derive(Deserialize)]
pub struct GlobalSearchQuery {
    pub query: Option<String>,
    pub db_only: Option<bool>,
}

async fn named_objects(
    state: &AppState,
    table: &str,
    columns: &[&str],
    query: &str,
) -> Result<Vec<Value>, AppError> {
    let sql = format!(
        "SELECT id, name{} FROM {table} WHERE name LIKE $1 ESCAPE '\\' ORDER BY name LIMIT {OBJECT_LIMIT}",
        columns.iter().map(|c| format!(", {c}")).collect::<String>()
    );
    let pattern = format!(
        "%{}%",
        query
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    );
    let rows = sqlx::query(&sql).bind(pattern).fetch_all(&state.db).await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let mut obj = serde_json::Map::new();
            obj.insert("id".into(), json!(r.get::<i64, _>("id")));
            obj.insert("name".into(), json!(r.get::<String, _>("name")));
            for c in columns {
                let v: Option<String> = r.try_get(*c).ok();
                obj.insert((*c).to_string(), json!(v));
            }
            Value::Object(obj)
        })
        .collect())
}

/// `GET /api/search/` — global search used by the top-bar search box.
pub async fn global_search(
    State(state): State<AppState>,
    Query(q): Query<GlobalSearchQuery>,
) -> Result<Json<Value>, AppError> {
    let query = q
        .query
        .ok_or_else(|| AppError::BadRequest("Query required".into()))?;
    if query.chars().count() < 3 {
        return Err(AppError::BadRequest(
            "Query must be at least 3 characters".into(),
        ));
    }
    if query.len() > MAX_QUERY_LENGTH {
        return Err(AppError::BadRequest(format!(
            "Query must be at most {MAX_QUERY_LENGTH} characters"
        )));
    }

    let doc_ids: Vec<i64> = if q.db_only.unwrap_or(false) {
        let pattern = format!(
            "%{}%",
            query
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        sqlx::query_scalar(&format!(
            "SELECT id FROM documents WHERE deleted_at IS NULL AND root_document_id IS NULL \
             AND title LIKE $1 ESCAPE '\\' LIMIT {OBJECT_LIMIT}"
        ))
        .bind(pattern)
        .fetch_all(&state.db)
        .await?
    } else {
        let hits = search::search_ids(
            &state,
            &query,
            SearchMode::Text,
            (OBJECT_LIMIT * 3) as usize,
        )?;
        hits.into_iter()
            .map(|h| h.id)
            .take(OBJECT_LIMIT as usize)
            .collect()
    };

    let mut documents = Vec::new();
    for id in doc_ids {
        if let Some(r) = sqlx::query(
            "SELECT id, title, created FROM documents WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        {
            documents.push(json!({
                "id": r.get::<i64, _>("id"),
                "title": r.get::<String, _>("title"),
                "created": r.get::<String, _>("created"),
            }));
        }
    }

    let saved_views = named_objects(&state, "saved_views", &[], &query).await?;
    let tags = named_objects(&state, "tags", &["color"], &query).await?;
    let correspondents = named_objects(&state, "correspondents", &[], &query).await?;
    let document_types = named_objects(&state, "document_types", &[], &query).await?;
    let storage_paths = named_objects(&state, "storage_paths", &["path"], &query).await?;
    let mail_rules = named_objects(&state, "mail_rules", &[], &query).await?;
    let mail_accounts = named_objects(&state, "mail_accounts", &[], &query).await?;
    let workflows = named_objects(&state, "workflows", &[], &query).await?;
    let custom_fields = named_objects(&state, "custom_fields", &["data_type"], &query).await?;

    let total = documents.len()
        + saved_views.len()
        + tags.len()
        + correspondents.len()
        + document_types.len()
        + storage_paths.len()
        + mail_rules.len()
        + mail_accounts.len()
        + workflows.len()
        + custom_fields.len();

    Ok(Json(json!({
        "total": total,
        "documents": documents,
        "saved_views": saved_views,
        "tags": tags,
        "correspondents": correspondents,
        "document_types": document_types,
        "storage_paths": storage_paths,
        "users": [],
        "groups": [],
        "mail_rules": mail_rules,
        "mail_accounts": mail_accounts,
        "workflows": workflows,
        "custom_fields": custom_fields,
    })))
}

#[derive(Deserialize)]
pub struct AutocompleteQuery {
    pub term: Option<String>,
    pub limit: Option<i64>,
}

/// `GET /api/search/autocomplete/?term=&limit=` — word completions from the index.
pub async fn autocomplete(
    State(state): State<AppState>,
    Query(q): Query<AutocompleteQuery>,
) -> Result<Json<Value>, AppError> {
    let term = q
        .term
        .ok_or_else(|| AppError::BadRequest("Term required".into()))?;
    let limit = q.limit.unwrap_or(10);
    if limit <= 0 {
        return Err(AppError::BadRequest("Invalid limit".into()));
    }
    let words = search::autocomplete(&state, term.trim(), limit as usize)?;
    Ok(Json(json!(words)))
}

/// `POST /api/search/reindex/` — rebuild the full-text index from the database.
pub async fn reindex(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let n = search::reindex_all(&state).await?;
    Ok(Json(json!({ "indexed": n })))
}
