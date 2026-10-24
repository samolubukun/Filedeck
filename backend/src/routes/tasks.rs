use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct TaskQuery {
    pub task_id: Option<String>,
    pub acknowledged: Option<bool>,
    pub status: Option<String>,
}

#[derive(Deserialize)]
pub struct AcknowledgePayload {
    pub tasks: Vec<i64>,
}

pub async fn list_tasks(
    State(state): State<AppState>,
    Query(query): Query<TaskQuery>,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query(
        "SELECT id, task_id, task_type, trigger_source, status,
                date_created, date_started, date_done,
                duration_seconds, wait_time_seconds, input_data, result_data, acknowledged
         FROM tasks ORDER BY date_created DESC LIMIT 100"
    )
    .fetch_all(&state.db)
    .await?;

    let results: Vec<Value> = rows.into_iter()
        .filter(|r| {
            let acknowledged: bool = r.get("acknowledged");
            let status: String = r.get("status");
            let task_id: String = r.get("task_id");

            if let Some(ack) = query.acknowledged {
                if acknowledged != ack {
                    return false;
                }
            }
            if let Some(ref st) = query.status {
                if &status != st {
                    return false;
                }
            }
            if let Some(ref tid) = query.task_id {
                if &task_id != tid {
                    return false;
                }
            }
            true
        })
        .map(|r| {
            let id: i64 = r.get("id");
            let task_id: String = r.get("task_id");
            let task_type: String = r.get("task_type");
            let trigger_source: String = r.get("trigger_source");
            let status: String = r.get("status");
            let date_created: String = r.get("date_created");
            let date_started: Option<String> = r.get("date_started");
            let date_done: Option<String> = r.get("date_done");
            let duration_seconds: Option<f64> = r.get("duration_seconds");
            let wait_time_seconds: Option<f64> = r.get("wait_time_seconds");
            let input_data_str: String = r.get("input_data");
            let result_data_str: Option<String> = r.get("result_data");
            let acknowledged: bool = r.get("acknowledged");

            let input = serde_json::from_str(&input_data_str).unwrap_or(json!({}));
            let result: Option<Value> = result_data_str.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok());
            json!({
                "id": id,
                "task_id": task_id,
                "task_type": task_type,
                "trigger_source": trigger_source,
                "status": status,
                "date_created": date_created,
                "date_started": date_started,
                "date_done": date_done,
                "duration_seconds": duration_seconds,
                "wait_time_seconds": wait_time_seconds,
                "input_data": input,
                "result_data": result,
                "acknowledged": acknowledged,
                "related_document": result.as_ref().and_then(|res: &Value| res.get("document_id").and_then(|v| v.as_i64()))
            })
        }).collect();

    Ok(Json(json!(results)))
}

pub async fn acknowledge_tasks(
    State(state): State<AppState>,
    Json(payload): Json<AcknowledgePayload>,
) -> Result<Json<Value>, AppError> {
    for tid in &payload.tasks {
        sqlx::query("UPDATE tasks SET acknowledged = 1 WHERE id = $1")
            .bind(tid)
            .execute(&state.db)
            .await?;
    }

    Ok(Json(json!({
        "result": payload.tasks.len()
    })))
}
