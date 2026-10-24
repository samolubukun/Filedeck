use crate::error::AppError;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;

#[derive(Serialize, Deserialize)]
pub struct WorkflowPayload {
    pub name: Option<String>,
    pub order: Option<i64>,
    pub enabled: Option<bool>,
    pub triggers: Option<Vec<i64>>,
    pub actions: Option<Vec<i64>>,
}

pub async fn list_workflows(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query(
        r#"SELECT id, name, "order", enabled FROM workflows ORDER BY "order" ASC, name ASC"#,
    )
    .fetch_all(&state.db)
    .await?;

    let mut results = Vec::new();
    for r in rows {
        let id: i64 = r.get("id");
        let name: String = r.get("name");
        let order: i64 = r.get("order");
        let enabled: bool = r.get("enabled");

        let trigger_rows =
            sqlx::query("SELECT trigger_id FROM workflow_workflow_triggers WHERE workflow_id = $1")
                .bind(id)
                .fetch_all(&state.db)
                .await?;

        let triggers: Vec<i64> = trigger_rows
            .into_iter()
            .map(|tr| tr.get("trigger_id"))
            .collect();

        let action_rows =
            sqlx::query("SELECT action_id FROM workflow_workflow_actions WHERE workflow_id = $1")
                .bind(id)
                .fetch_all(&state.db)
                .await?;

        let actions: Vec<i64> = action_rows
            .into_iter()
            .map(|ar| ar.get("action_id"))
            .collect();

        results.push(json!({
            "id": id,
            "name": name,
            "order": order,
            "enabled": enabled,
            "triggers": triggers,
            "actions": actions
        }));
    }

    Ok(Json(json!({
        "count": results.len(),
        "results": results
    })))
}

pub async fn create_workflow(
    State(state): State<AppState>,
    Json(payload): Json<WorkflowPayload>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let order = payload.order.unwrap_or(0);
    let enabled = payload.enabled.unwrap_or(true);
    let name = payload
        .name
        .clone()
        .ok_or_else(|| AppError::BadRequest("name is required".into()))?;

    let mut tx = state.db.begin().await?;

    let id: i64 = sqlx::query_scalar(
        r#"INSERT INTO workflows (name, "order", enabled) VALUES ($1, $2, $3) RETURNING id"#,
    )
    .bind(&name)
    .bind(order)
    .bind(enabled)
    .fetch_one(&mut *tx)
    .await?;

    if let Some(triggers) = payload.triggers {
        for tid in triggers {
            sqlx::query(
                "INSERT INTO workflow_workflow_triggers (workflow_id, trigger_id) VALUES ($1, $2)",
            )
            .bind(id)
            .bind(tid)
            .execute(&mut *tx)
            .await?;
        }
    }

    if let Some(actions) = payload.actions {
        for aid in actions {
            sqlx::query(
                "INSERT INTO workflow_workflow_actions (workflow_id, action_id) VALUES ($1, $2)",
            )
            .bind(id)
            .bind(aid)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    get_workflow(State(state), Path(id))
        .await
        .map(|j| (StatusCode::CREATED, j))
}

pub async fn get_workflow(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let r = sqlx::query(r#"SELECT id, name, "order", enabled FROM workflows WHERE id = $1"#)
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Workflow {} not found", id)))?;

    let name: String = r.get("name");
    let order: i64 = r.get("order");
    let enabled: bool = r.get("enabled");

    let trigger_rows =
        sqlx::query("SELECT trigger_id FROM workflow_workflow_triggers WHERE workflow_id = $1")
            .bind(id)
            .fetch_all(&state.db)
            .await?;

    let triggers: Vec<i64> = trigger_rows
        .into_iter()
        .map(|tr| tr.get("trigger_id"))
        .collect();

    let action_rows =
        sqlx::query("SELECT action_id FROM workflow_workflow_actions WHERE workflow_id = $1")
            .bind(id)
            .fetch_all(&state.db)
            .await?;

    let actions: Vec<i64> = action_rows
        .into_iter()
        .map(|ar| ar.get("action_id"))
        .collect();

    Ok(Json(json!({
        "id": id,
        "name": name,
        "order": order,
        "enabled": enabled,
        "triggers": triggers,
        "actions": actions
    })))
}

pub async fn update_workflow(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<WorkflowPayload>,
) -> Result<Json<Value>, AppError> {
    let _ = get_workflow(State(state.clone()), Path(id)).await?;
    let mut tx = state.db.begin().await?;

    sqlx::query(
        r#"UPDATE workflows SET name = COALESCE($1, name), "order" = COALESCE($2, "order"),
           enabled = COALESCE($3, enabled) WHERE id = $4"#,
    )
    .bind(&payload.name)
    .bind(payload.order)
    .bind(payload.enabled)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    if let Some(triggers) = payload.triggers {
        sqlx::query("DELETE FROM workflow_workflow_triggers WHERE workflow_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        for tid in triggers {
            sqlx::query(
                "INSERT INTO workflow_workflow_triggers (workflow_id, trigger_id) VALUES ($1, $2)",
            )
            .bind(id)
            .bind(tid)
            .execute(&mut *tx)
            .await?;
        }
    }

    if let Some(actions) = payload.actions {
        sqlx::query("DELETE FROM workflow_workflow_actions WHERE workflow_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        for aid in actions {
            sqlx::query(
                "INSERT INTO workflow_workflow_actions (workflow_id, action_id) VALUES ($1, $2)",
            )
            .bind(id)
            .bind(aid)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    get_workflow(State(state), Path(id)).await
}

pub async fn delete_workflow(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    sqlx::query("DELETE FROM workflows WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, Deserialize)]
pub struct WorkflowTriggerPayload {
    pub r#type: i64,
    pub sources: Option<String>,
    pub filter_path: Option<String>,
    pub filter_filename: Option<String>,
    pub filter_mailrule: Option<i64>,
    #[serde(rename = "match", alias = "match_pattern")]
    pub match_pattern: Option<String>,
    pub matching_algorithm: Option<i64>,
    pub is_insensitive: Option<bool>,
}

pub async fn list_workflow_triggers(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query(
        "SELECT id, type, sources, filter_path, filter_filename, filter_mailrule_id,
                match, matching_algorithm, is_insensitive FROM workflow_triggers ORDER BY id ASC",
    )
    .fetch_all(&state.db)
    .await?;

    let results: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            let id: i64 = r.get("id");
            let r#type: i64 = r.get("type");
            let sources: String = r.get("sources");
            let filter_path: Option<String> = r.get("filter_path");
            let filter_filename: Option<String> = r.get("filter_filename");
            let filter_mailrule: Option<i64> = r.get("filter_mailrule_id");
            let match_pattern: Option<String> = r.get("match");
            let algo: i64 = r.get("matching_algorithm");
            let is_insensitive: bool = r.get("is_insensitive");

            json!({
                "id": id,
                "type": r#type,
                "sources": sources,
                "filter_path": filter_path,
                "filter_filename": filter_filename,
                "filter_mailrule": filter_mailrule,
                "match": match_pattern,
                "matching_algorithm": algo,
                "is_insensitive": is_insensitive
            })
        })
        .collect();

    Ok(Json(json!({
        "count": results.len(),
        "results": results
    })))
}

pub async fn create_workflow_trigger(
    State(state): State<AppState>,
    Json(payload): Json<WorkflowTriggerPayload>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let sources = payload.sources.unwrap_or_default();
    let algo = payload.matching_algorithm.unwrap_or(1);
    let insensitive = payload.is_insensitive.unwrap_or(true);

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO workflow_triggers (type, sources, filter_path, filter_filename, filter_mailrule_id,
                                       match, matching_algorithm, is_insensitive)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id"
    )
    .bind(payload.r#type)
    .bind(&sources)
    .bind(&payload.filter_path)
    .bind(&payload.filter_filename)
    .bind(payload.filter_mailrule)
    .bind(&payload.match_pattern)
    .bind(algo)
    .bind(insensitive)
    .fetch_one(&state.db)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": id,
            "type": payload.r#type,
            "sources": sources,
            "filter_path": payload.filter_path,
            "filter_filename": payload.filter_filename,
            "filter_mailrule": payload.filter_mailrule,
            "match": payload.match_pattern,
            "matching_algorithm": algo,
            "is_insensitive": insensitive
        })),
    ))
}

pub async fn delete_workflow_trigger(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    sqlx::query("DELETE FROM workflow_triggers WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, Deserialize)]
pub struct WorkflowActionPayload {
    pub r#type: i64,
    pub order: Option<i64>,
    pub assign_title: Option<String>,
    pub assign_tags: Option<Vec<i64>>,
    pub assign_correspondent: Option<i64>,
    pub assign_document_type: Option<i64>,
    pub assign_storage_path: Option<i64>,
    pub assign_asn: Option<i64>,
    pub remove_all_tags: Option<bool>,
    pub email_to: Option<String>,
    pub email_subject: Option<String>,
    pub email_body: Option<String>,
    pub webhook_url: Option<String>,
}

pub async fn list_workflow_actions(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query(
        r#"SELECT id, type, "order", assign_title, assign_tags, assign_correspondent_id,
                 assign_document_type_id, assign_storage_path_id, assign_asn,
                 remove_all_tags, email_to, email_subject, email_body, webhook_url
          FROM workflow_actions ORDER BY "order" ASC"#,
    )
    .fetch_all(&state.db)
    .await?;

    let results: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            let id: i64 = r.get("id");
            let r#type: i64 = r.get("type");
            let order: i64 = r.get("order");
            let assign_title: Option<String> = r.get("assign_title");
            let assign_tags_str: Option<String> = r.get("assign_tags");
            let assign_corr: Option<i64> = r.get("assign_correspondent_id");
            let assign_doc_type: Option<i64> = r.get("assign_document_type_id");
            let assign_sp: Option<i64> = r.get("assign_storage_path_id");
            let assign_asn: Option<i64> = r.get("assign_asn");
            let remove_all: bool = r.get("remove_all_tags");
            let email_to: Option<String> = r.get("email_to");
            let email_subject: Option<String> = r.get("email_subject");
            let email_body: Option<String> = r.get("email_body");
            let webhook_url: Option<String> = r.get("webhook_url");

            let tags: Option<Vec<i64>> = assign_tags_str
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok());
            json!({
                "id": id,
                "type": r#type,
                "order": order,
                "assign_title": assign_title,
                "assign_tags": tags,
                "assign_correspondent": assign_corr,
                "assign_document_type": assign_doc_type,
                "assign_storage_path": assign_sp,
                "assign_asn": assign_asn,
                "remove_all_tags": remove_all,
                "email_to": email_to,
                "email_subject": email_subject,
                "email_body": email_body,
                "webhook_url": webhook_url
            })
        })
        .collect();

    Ok(Json(json!({
        "count": results.len(),
        "results": results
    })))
}

pub async fn create_workflow_action(
    State(state): State<AppState>,
    Json(payload): Json<WorkflowActionPayload>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let order = payload.order.unwrap_or(0);
    let tags_str = payload
        .assign_tags
        .as_ref()
        .map(|t| serde_json::to_string(t).unwrap_or_default());
    let remove_all = payload.remove_all_tags.unwrap_or(false);

    let id: i64 = sqlx::query_scalar(
        r#"INSERT INTO workflow_actions (type, "order", assign_title, assign_tags,
                                        assign_correspondent_id, assign_document_type_id,
                                        assign_storage_path_id, assign_asn, remove_all_tags,
                                        email_to, email_subject, email_body, webhook_url)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) RETURNING id"#,
    )
    .bind(payload.r#type)
    .bind(order)
    .bind(&payload.assign_title)
    .bind(tags_str)
    .bind(payload.assign_correspondent)
    .bind(payload.assign_document_type)
    .bind(payload.assign_storage_path)
    .bind(payload.assign_asn)
    .bind(remove_all)
    .bind(&payload.email_to)
    .bind(&payload.email_subject)
    .bind(&payload.email_body)
    .bind(&payload.webhook_url)
    .fetch_one(&state.db)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": id,
            "type": payload.r#type,
            "order": order,
            "assign_title": payload.assign_title,
            "assign_tags": payload.assign_tags,
            "assign_correspondent": payload.assign_correspondent,
            "assign_document_type": payload.assign_document_type,
            "assign_storage_path": payload.assign_storage_path,
            "assign_asn": payload.assign_asn,
            "remove_all_tags": remove_all,
            "email_to": payload.email_to,
            "email_subject": payload.email_subject,
            "email_body": payload.email_body,
            "webhook_url": payload.webhook_url
        })),
    ))
}

pub async fn delete_workflow_action(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    sqlx::query("DELETE FROM workflow_actions WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
