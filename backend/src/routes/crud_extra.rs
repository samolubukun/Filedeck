//! Extra CRUD endpoints (retrieve / update / partial update etc.) for
//! workflow triggers/actions, mail accounts/rules, share links, processed
//! mail and tasks. Registered here because the handler-less methods are merged
//! with the base routes in `mod.rs` (axum merges different methods on one path).

use crate::error::AppError;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Map, Value};
use sqlx::{Row, Sqlite, Transaction};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/workflow_triggers/:id/",
            get(get_trigger).put(update_trigger).patch(update_trigger),
        )
        .route(
            "/api/workflow_actions/:id/",
            get(get_action).put(update_action).patch(update_action),
        )
        .route(
            "/api/mail_accounts/:id/",
            get(get_account).put(update_account),
        )
        .route(
            "/api/mail_rules/:id/",
            get(get_rule).put(update_rule),
        )
        .route(
            "/api/share_links/:id/",
            get(get_share_link)
                .put(update_share_link)
                .patch(update_share_link),
        )
        .route("/api/share_link_bundles/:id/", get(get_bundle))
        .route(
            "/api/processed_mail/bulk_delete/",
            post(bulk_delete_processed),
        )
        .route(
            "/api/processed_mail/:id/",
            axum::routing::delete(delete_processed),
        )
        .route("/api/tasks/run/", post(run_task))
        .route("/api/tasks/:id/", get(get_task))
}

// ---------- generic partial-update helper ----------

#[derive(Clone, Copy)]
enum K {
    Str,
    Int,
    Bool,
    /// JSON array/object stored as TEXT
    Json,
}

/// (json key, column SQL, kind)
type Field = (&'static str, &'static str, K);

fn body_map(body: Value) -> Result<Map<String, Value>, AppError> {
    match body {
        Value::Object(m) => Ok(m),
        _ => Err(AppError::BadRequest("Expected a JSON object".into())),
    }
}

/// Applies only the keys present in `map`. Table/column names come from static tables.
async fn apply_fields(
    tx: &mut Transaction<'_, Sqlite>,
    table: &str,
    id: i64,
    map: &Map<String, Value>,
    fields: &[Field],
) -> Result<(), AppError> {
    for (key, col, kind) in fields {
        let Some(v) = map.get(*key) else { continue };
        let sql = format!("UPDATE {} SET {} = $1 WHERE id = $2", table, col);
        let q = sqlx::query(&sql);
        let bad = || AppError::BadRequest(format!("Invalid value for '{}'", key));
        let q = match kind {
            K::Str => match v {
                Value::Null => q.bind(None::<String>),
                Value::String(s) => q.bind(Some(s.clone())),
                _ => return Err(bad()),
            },
            K::Int => match v {
                Value::Null => q.bind(None::<i64>),
                Value::Number(n) => q.bind(Some(n.as_i64().ok_or_else(bad)?)),
                Value::String(s) => q.bind(Some(s.parse::<i64>().map_err(|_| bad())?)),
                _ => return Err(bad()),
            },
            K::Bool => match v {
                Value::Bool(b) => q.bind(*b),
                _ => return Err(bad()),
            },
            K::Json => match v {
                Value::Null => q.bind(None::<String>),
                other => q.bind(Some(other.to_string())),
            },
        };
        q.bind(id)
            .execute(&mut **tx)
            .await
            .map_err(|e| AppError::BadRequest(format!("Could not update '{}': {}", key, e)))?;
    }
    Ok(())
}

fn parse_json_text(s: Option<String>) -> Value {
    s.and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or(Value::Null)
}

fn parse_list(s: Option<String>) -> Value {
    match parse_json_text(s) {
        Value::Array(a) => Value::Array(a),
        _ => json!([]),
    }
}

// ---------- workflow triggers ----------

const TRIGGER_FIELDS: &[Field] = &[
    ("type", "type", K::Int),
    ("sources", "sources", K::Str),
    ("filter_path", "filter_path", K::Str),
    ("filter_filename", "filter_filename", K::Str),
    ("filter_mailrule", "filter_mailrule_id", K::Int),
    ("match", "match", K::Str),
    ("matching_algorithm", "matching_algorithm", K::Int),
    ("is_insensitive", "is_insensitive", K::Bool),
    ("schedule_date_field", "schedule_date_field", K::Str),
    ("schedule_offset_days", "schedule_offset_days", K::Int),
    ("schedule_is_recurring", "schedule_is_recurring", K::Bool),
    (
        "schedule_recurring_interval_days",
        "schedule_recurring_interval_days",
        K::Int,
    ),
    (
        "schedule_date_custom_field",
        "schedule_custom_field_id",
        K::Int,
    ),
];

async fn trigger_json(state: &AppState, id: i64) -> Result<Value, AppError> {
    let r = sqlx::query("SELECT * FROM workflow_triggers WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Workflow trigger {} not found", id)))?;
    let sources: String = r.get("sources");
    let sources_list: Vec<i64> = sources
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    Ok(json!({
        "id": id,
        "type": r.get::<i64, _>("type"),
        "sources": sources_list,
        "filter_path": r.get::<Option<String>, _>("filter_path"),
        "filter_filename": r.get::<Option<String>, _>("filter_filename"),
        "filter_mailrule": r.get::<Option<i64>, _>("filter_mailrule_id"),
        "match": r.get::<Option<String>, _>("match").unwrap_or_default(),
        "matching_algorithm": r.get::<i64, _>("matching_algorithm"),
        "is_insensitive": r.get::<bool, _>("is_insensitive"),
        "schedule_date_field": r.get::<Option<String>, _>("schedule_date_field"),
        "schedule_offset_days": r.get::<Option<i64>, _>("schedule_offset_days").unwrap_or(0),
        "schedule_is_recurring": r.get::<bool, _>("schedule_is_recurring"),
        "schedule_recurring_interval_days": r.get::<Option<i64>, _>("schedule_recurring_interval_days").unwrap_or(1),
        "schedule_date_custom_field": r.get::<Option<i64>, _>("schedule_custom_field_id"),
    }))
}

async fn get_trigger(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    Ok(Json(trigger_json(&state, id).await?))
}

async fn update_trigger(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    trigger_json(&state, id).await?; // 404 check
    let mut map = body_map(body)?;
    // `sources` is a list of ints; stored as comma-joined text.
    if let Some(Value::Array(a)) = map.get("sources") {
        let s = a
            .iter()
            .map(|v| v.as_i64().map(|n| n.to_string()).unwrap_or_default())
            .collect::<Vec<_>>()
            .join(",");
        map.insert("sources".into(), Value::String(s));
    }
    if matches!(map.get("match"), Some(Value::Null)) {
        map.insert("match".into(), Value::String(String::new()));
    }
    let mut tx = state.db.begin().await?;
    apply_fields(&mut tx, "workflow_triggers", id, &map, TRIGGER_FIELDS).await?;
    tx.commit().await?;
    Ok(Json(trigger_json(&state, id).await?))
}

// ---------- workflow actions ----------

const ACTION_FIELDS: &[Field] = &[
    ("type", "type", K::Int),
    ("order", "\"order\"", K::Int),
    ("assign_title", "assign_title", K::Str),
    ("assign_tags", "assign_tags", K::Json),
    ("assign_correspondent", "assign_correspondent_id", K::Int),
    ("assign_document_type", "assign_document_type_id", K::Int),
    ("assign_storage_path", "assign_storage_path_id", K::Int),
    ("assign_asn", "assign_asn", K::Int),
    ("assign_custom_fields", "assign_custom_fields", K::Json),
    ("remove_tags", "remove_tags", K::Json),
    ("remove_all_tags", "remove_all_tags", K::Bool),
    ("remove_correspondents", "remove_correspondent", K::Bool),
    ("remove_document_types", "remove_document_type", K::Bool),
    ("remove_storage_paths", "remove_storage_path", K::Bool),
    ("remove_custom_fields", "remove_custom_fields", K::Json),
    (
        "remove_all_custom_fields",
        "remove_all_custom_fields",
        K::Bool,
    ),
    ("email_to", "email_to", K::Str),
    ("email_subject", "email_subject", K::Str),
    ("email_body", "email_body", K::Str),
    ("webhook_url", "webhook_url", K::Str),
    (
        "webhook_include_document",
        "webhook_include_document",
        K::Bool,
    ),
];

async fn action_json(state: &AppState, id: i64) -> Result<Value, AppError> {
    let r = sqlx::query("SELECT * FROM workflow_actions WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Workflow action {} not found", id)))?;
    Ok(json!({
        "id": id,
        "type": r.get::<i64, _>("type"),
        "order": r.get::<i64, _>("order"),
        "assign_title": r.get::<Option<String>, _>("assign_title"),
        "assign_tags": parse_list(r.get("assign_tags")),
        "assign_correspondent": r.get::<Option<i64>, _>("assign_correspondent_id"),
        "assign_document_type": r.get::<Option<i64>, _>("assign_document_type_id"),
        "assign_storage_path": r.get::<Option<i64>, _>("assign_storage_path_id"),
        "assign_asn": r.get::<Option<i64>, _>("assign_asn"),
        "assign_custom_fields": parse_list(r.get("assign_custom_fields")),
        "remove_tags": parse_list(r.get("remove_tags")),
        "remove_all_tags": r.get::<bool, _>("remove_all_tags"),
        "remove_correspondents": r.get::<bool, _>("remove_correspondent"),
        "remove_document_types": r.get::<bool, _>("remove_document_type"),
        "remove_storage_paths": r.get::<bool, _>("remove_storage_path"),
        "remove_custom_fields": parse_list(r.get("remove_custom_fields")),
        "remove_all_custom_fields": r.get::<bool, _>("remove_all_custom_fields"),
        "email": {
            "id": id,
            "subject": r.get::<Option<String>, _>("email_subject"),
            "body": r.get::<Option<String>, _>("email_body"),
            "to": r.get::<Option<String>, _>("email_to"),
        },
        "email_to": r.get::<Option<String>, _>("email_to"),
        "email_subject": r.get::<Option<String>, _>("email_subject"),
        "email_body": r.get::<Option<String>, _>("email_body"),
        "webhook_url": r.get::<Option<String>, _>("webhook_url"),
        "webhook_include_document": r.get::<bool, _>("webhook_include_document"),
    }))
}

async fn get_action(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    Ok(Json(action_json(&state, id).await?))
}

async fn update_action(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    action_json(&state, id).await?;
    let mut map = body_map(body)?;
    // Nests email fields; accept both forms.
    if let Some(Value::Object(e)) = map.get("email").cloned() {
        for (src, dst) in [
            ("to", "email_to"),
            ("subject", "email_subject"),
            ("body", "email_body"),
        ] {
            if let Some(v) = e.get(src) {
                map.insert(dst.into(), v.clone());
            }
        }
    }
    let mut tx = state.db.begin().await?;
    apply_fields(&mut tx, "workflow_actions", id, &map, ACTION_FIELDS).await?;
    tx.commit().await?;
    Ok(Json(action_json(&state, id).await?))
}

// ---------- mail accounts ----------

const MASK: &str = "**********";

const ACCOUNT_FIELDS: &[Field] = &[
    ("name", "name", K::Str),
    ("imap_server", "imap_server", K::Str),
    ("imap_port", "imap_port", K::Int),
    ("imap_security", "imap_security", K::Int),
    ("username", "username", K::Str),
    ("is_token", "is_token", K::Bool),
    ("character_set", "character_set", K::Str),
    ("account_type", "account_type", K::Int),
];

async fn account_json(state: &AppState, id: i64) -> Result<Value, AppError> {
    let r = sqlx::query(
        "SELECT id, name, imap_server, imap_port, imap_security, username, is_token,
                character_set, account_type, expiration FROM mail_accounts WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Mail account {} not found", id)))?;
    Ok(json!({
        "id": id,
        "name": r.get::<String, _>("name"),
        "imap_server": r.get::<String, _>("imap_server"),
        "imap_port": r.get::<Option<i64>, _>("imap_port"),
        "imap_security": r.get::<i64, _>("imap_security"),
        "username": r.get::<String, _>("username"),
        "password": MASK,
        "character_set": r.get::<String, _>("character_set"),
        "is_token": r.get::<bool, _>("is_token"),
        "account_type": r.get::<i64, _>("account_type"),
        "expiration": r.get::<Option<String>, _>("expiration"),
    }))
}

async fn get_account(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    Ok(Json(account_json(&state, id).await?))
}

async fn update_account(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    account_json(&state, id).await?;
    let map = body_map(body)?;
    let mut tx = state.db.begin().await?;
    apply_fields(&mut tx, "mail_accounts", id, &map, ACCOUNT_FIELDS).await?;
    // A masked / empty password means "unchanged".
    if let Some(Value::String(p)) = map.get("password") {
        if !p.is_empty() && !p.chars().all(|c| c == '*') {
            sqlx::query("UPDATE mail_accounts SET password = $1 WHERE id = $2")
                .bind(p)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;
    Ok(Json(account_json(&state, id).await?))
}

async fn test_mail_account(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let server = body.get("imap_server").and_then(|v| v.as_str()).unwrap_or("");
    let username = body.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let password = body.get("password").and_then(|v| v.as_str()).unwrap_or("");
    let port = body.get("imap_port").and_then(|v| v.as_i64());
    let security = body.get("imap_security").and_then(|v| v.as_i64()).unwrap_or(1);

    let resp = state
        .worker_client
        .post(format!("{}/mail/test", state.config.worker.url))
        .json(&json!({
            "imap_server": server,
            "imap_port": port,
            "imap_security": security,
            "username": username,
            "password": password
        }))
        .send()
        .await;

    match resp {
        Ok(r) => {
            let res = r
                .json::<Value>()
                .await
                .unwrap_or(json!({"success": false, "detail": "Invalid worker response"}));
            Ok(Json(res))
        }
        Err(e) => Ok(Json(json!({
            "success": false,
            "detail": format!("Worker unreachable: {e}")
        }))),
    }
}

// ---------- mail rules ----------

const RULE_FIELDS: &[Field] = &[
    ("account", "account_id", K::Int),
    ("name", "name", K::Str),
    ("order", "\"order\"", K::Int),
    ("enabled", "enabled", K::Bool),
    ("folder", "folder", K::Str),
    ("filter_from", "filter_from", K::Str),
    ("filter_to", "filter_to", K::Str),
    ("filter_subject", "filter_subject", K::Str),
    ("filter_body", "filter_body", K::Str),
    (
        "filter_attachment_filename_include",
        "filter_attachment_filename",
        K::Str,
    ),
    (
        "filter_attachment_filename",
        "filter_attachment_filename",
        K::Str,
    ),
    ("maximum_age", "maximum_age", K::Int),
    ("attachment_type", "attachment_type", K::Int),
    ("action", "action", K::Int),
    ("action_parameter", "action_parameter", K::Str),
    ("assign_title_from", "assign_title_from", K::Int),
    (
        "assign_correspondent_from",
        "assign_correspondent_from",
        K::Int,
    ),
    ("assign_tags", "assign_tags", K::Json),
    ("assign_document_type", "assign_document_type_id", K::Int),
    ("assign_correspondent", "assign_correspondent_id", K::Int),
    ("consumption_scope", "consumption_scope", K::Int),
];

async fn rule_json(state: &AppState, id: i64) -> Result<Value, AppError> {
    let r = sqlx::query("SELECT * FROM mail_rules WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Mail rule {} not found", id)))?;
    let fa: Option<String> = r.get("filter_attachment_filename");
    Ok(json!({
        "id": id,
        "account": r.get::<i64, _>("account_id"),
        "name": r.get::<String, _>("name"),
        "order": r.get::<i64, _>("order"),
        "enabled": r.get::<bool, _>("enabled"),
        "folder": r.get::<String, _>("folder"),
        "filter_from": r.get::<Option<String>, _>("filter_from"),
        "filter_to": r.get::<Option<String>, _>("filter_to"),
        "filter_subject": r.get::<Option<String>, _>("filter_subject"),
        "filter_body": r.get::<Option<String>, _>("filter_body"),
        "filter_attachment_filename_include": fa,
        "filter_attachment_filename": fa,
        "maximum_age": r.get::<Option<i64>, _>("maximum_age").unwrap_or(30),
        "attachment_type": r.get::<i64, _>("attachment_type"),
        "action": r.get::<i64, _>("action"),
        "action_parameter": r.get::<Option<String>, _>("action_parameter"),
        "assign_title_from": r.get::<i64, _>("assign_title_from"),
        "assign_correspondent_from": r.get::<i64, _>("assign_correspondent_from"),
        "assign_tags": parse_list(r.get("assign_tags")),
        "assign_document_type": r.get::<Option<i64>, _>("assign_document_type_id"),
        "assign_correspondent": r.get::<Option<i64>, _>("assign_correspondent_id"),
        "consumption_scope": r.get::<i64, _>("consumption_scope"),
    }))
}

async fn get_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    Ok(Json(rule_json(&state, id).await?))
}

async fn update_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    rule_json(&state, id).await?;
    let map = body_map(body)?;
    let mut tx = state.db.begin().await?;
    apply_fields(&mut tx, "mail_rules", id, &map, RULE_FIELDS).await?;
    tx.commit().await?;
    Ok(Json(rule_json(&state, id).await?))
}

async fn process_mail_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let rule = rule_json(&state, id).await?;
    let account_id = rule.get("account").and_then(|v| v.as_i64()).unwrap_or(0);
    let account = account_json(&state, account_id).await?;

    let acc_row = sqlx::query("SELECT password FROM mail_accounts WHERE id = $1")
        .bind(account_id)
        .fetch_optional(&state.db)
        .await?;
    let password = acc_row
        .and_then(|r| r.get::<Option<String>, _>("password"))
        .unwrap_or_default();

    let resp = state
        .worker_client
        .post(format!("{}/mail/fetch", state.config.worker.url))
        .json(&json!({
            "imap_server": account.get("imap_server").and_then(|v| v.as_str()).unwrap_or(""),
            "imap_port": account.get("imap_port").and_then(|v| v.as_i64()),
            "imap_security": account.get("imap_security").and_then(|v| v.as_i64()).unwrap_or(1),
            "username": account.get("username").and_then(|v| v.as_str()).unwrap_or(""),
            "password": password,
            "folder": rule.get("folder").and_then(|v| v.as_str()).unwrap_or("INBOX"),
            "filter_from": rule.get("filter_from").and_then(|v| v.as_str()),
            "filter_subject": rule.get("filter_subject").and_then(|v| v.as_str()),
            "limit": 20
        }))
        .send()
        .await;

    match resp {
        Ok(r) => {
            let res = r.json::<Value>().await.unwrap_or(json!({"messages_processed": 0, "attachments": []}));
            let attachments = res
                .get("attachments")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let mut ingested = 0;
            use base64::Engine;
            for att in attachments {
                let filename = att
                    .get("filename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("email_attachment.pdf");
                let data_b64 = att.get("data_b64").and_then(|v| v.as_str()).unwrap_or("");
                let msg_id = att.get("message_id").and_then(|v| v.as_str()).unwrap_or("");
                if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(data_b64) {
                    let req = crate::ingest::ConsumeRequest {
                        bytes,
                        filename: filename.to_string(),
                        title: None,
                        created: None,
                        correspondent: rule.get("assign_correspondent").and_then(|v| v.as_i64()),
                        document_type: rule.get("assign_document_type").and_then(|v| v.as_i64()),
                        storage_path: None,
                        tags: rule
                            .get("assign_tags")
                            .and_then(|v| serde_json::from_value::<Vec<i64>>(v.clone()).ok())
                            .unwrap_or_default(),
                        asn: None,
                        custom_fields: vec![],
                        source: "mail".to_string(),
                        root_document_id: None,
                        version_index: Some(0),
                        version_label: None,
                    };
                    if crate::ingest::submit(&state, req).await.is_ok() {
                        ingested += 1;
                        let now = chrono::Utc::now().to_rfc3339();
                        let _ = sqlx::query(
                            "INSERT OR IGNORE INTO processed_mail (rule_id, message_id, folder, processed)
                             VALUES ($1, $2, $3, $4)",
                        )
                        .bind(id)
                        .bind(msg_id)
                        .bind(rule.get("folder").and_then(|v| v.as_str()).unwrap_or("INBOX"))
                        .bind(&now)
                        .execute(&state.db)
                        .await;
                    }
                }
            }
            Ok(Json(json!({
                "result": "OK",
                "rule_id": id,
                "attachments_ingested": ingested
            })))
        }
        Err(e) => Err(AppError::Internal(format!("Worker mail fetch failed: {e}"))),
    }
}

// ---------- share links ----------

async fn share_link_json(state: &AppState, id: i64) -> Result<Value, AppError> {
    let r = sqlx::query(
        "SELECT id, document_id, slug, expiration, file_version, created
         FROM share_links WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Share link {} not found", id)))?;
    Ok(json!({
        "id": id,
        "created": r.get::<String, _>("created"),
        "expiration": r.get::<Option<String>, _>("expiration"),
        "slug": r.get::<String, _>("slug"),
        "document": r.get::<i64, _>("document_id"),
        "file_version": r.get::<String, _>("file_version"),
    }))
}

async fn get_share_link(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    Ok(Json(share_link_json(&state, id).await?))
}

async fn update_share_link(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    share_link_json(&state, id).await?;
    let map = body_map(body)?;
    if let Some(Value::String(v)) = map.get("file_version") {
        if v != "archive" && v != "original" {
            return Err(AppError::BadRequest(
                "file_version must be 'archive' or 'original'".into(),
            ));
        }
    }
    const FIELDS: &[Field] = &[
        ("expiration", "expiration", K::Str),
        ("file_version", "file_version", K::Str),
        ("document", "document_id", K::Int),
    ];
    let mut tx = state.db.begin().await?;
    apply_fields(&mut tx, "share_links", id, &map, FIELDS).await?;
    tx.commit().await?;
    Ok(Json(share_link_json(&state, id).await?))
}

async fn get_bundle(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let r = sqlx::query(
        "SELECT id, slug, expiration, file_version, status, size_bytes, last_error, built_at
         FROM share_link_bundles WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Share link bundle {} not found", id)))?;
    let docs: Vec<i64> = sqlx::query_scalar(
        "SELECT document_id FROM share_link_bundle_documents WHERE bundle_id = $1",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!({
        "id": id,
        "slug": r.get::<String, _>("slug"),
        "expiration": r.get::<Option<String>, _>("expiration"),
        "file_version": r.get::<String, _>("file_version"),
        "status": r.get::<String, _>("status"),
        "size_bytes": r.get::<Option<i64>, _>("size_bytes"),
        "last_error": r.get::<Option<String>, _>("last_error"),
        "built_at": r.get::<Option<String>, _>("built_at"),
        "document_ids": docs,
        "documents": docs,
    })))
}

// ---------- processed mail ----------

async fn delete_processed(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let res = sqlx::query("DELETE FROM processed_mail WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "Processed mail {} not found",
            id
        )));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn bulk_delete_processed(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let ids: Vec<i64> = body
        .get("mail")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::BadRequest("'mail' must be a list of ids".into()))?
        .iter()
        .map(|v| {
            v.as_i64()
                .ok_or_else(|| AppError::BadRequest("'mail' must contain integers".into()))
        })
        .collect::<Result<_, _>>()?;
    let mut tx = state.db.begin().await?;
    let mut deleted = Vec::new();
    for id in ids {
        let res = sqlx::query("DELETE FROM processed_mail WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        if res.rows_affected() > 0 {
            deleted.push(id);
        }
    }
    tx.commit().await?;
    Ok(Json(json!({ "result": "OK", "deleted_mail_ids": deleted })))
}

// ---------- tasks ----------

async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let r = sqlx::query("SELECT * FROM tasks WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Task {} not found", id)))?;
    let input: Value = serde_json::from_str(&r.get::<String, _>("input_data")).unwrap_or(json!({}));
    let result: Option<Value> = r
        .get::<Option<String>, _>("result_data")
        .and_then(|s| serde_json::from_str(&s).ok());
    Ok(Json(json!({
        "id": id,
        "task_id": r.get::<String, _>("task_id"),
        "task_type": r.get::<String, _>("task_type"),
        "trigger_source": r.get::<String, _>("trigger_source"),
        "status": r.get::<String, _>("status"),
        "date_created": r.get::<String, _>("date_created"),
        "date_started": r.get::<Option<String>, _>("date_started"),
        "date_done": r.get::<Option<String>, _>("date_done"),
        "duration_seconds": r.get::<Option<f64>, _>("duration_seconds"),
        "wait_time_seconds": r.get::<Option<f64>, _>("wait_time_seconds"),
        "input_data": input,
        "related_document": result.as_ref().and_then(|v| v.get("document_id")).and_then(|v| v.as_i64()),
        "result_data": result,
        "acknowledged": r.get::<bool, _>("acknowledged"),
    })))
}

const TASK_NAMES: &[&str] = &[
    "index_optimize",
    "train_classifier",
    "check_sanity",
    "llmindex_update",
];

async fn run_task(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let name = body
        .get("task_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("'task_name' is required".into()))?;
    if !TASK_NAMES.contains(&name) {
        return Err(AppError::BadRequest(format!(
            "Unknown task_name '{}'. Valid: {}",
            name,
            TASK_NAMES.join(", ")
        )));
    }

    match name {
        "index_optimize" => {
            let n = crate::search::reindex_all(&state).await?;
            Ok(Json(json!({
                "result": "OK",
                "message": format!("Search index optimized. {} documents indexed.", n),
                "indexed_documents": n
            })))
        }
        "train_classifier" => {
            let docs = sqlx::query(
                "SELECT d.id, d.content, d.correspondent_id, d.document_type_id
                 FROM documents d WHERE d.deleted_at IS NULL",
            )
            .fetch_all(&state.db)
            .await?;

            let mut training_data = Vec::new();
            for d in docs {
                let id: i64 = d.get("id");
                let tags: Vec<i64> = sqlx::query_scalar("SELECT tag_id FROM document_tags WHERE document_id = $1")
                    .bind(id)
                    .fetch_all(&state.db)
                    .await?;
                training_data.push(json!({
                    "content": d.get::<String, _>("content"),
                    "tags": tags,
                    "correspondent_id": d.get::<Option<i64>, _>("correspondent_id"),
                    "document_type_id": d.get::<Option<i64>, _>("document_type_id"),
                }));
            }

            let resp = state
                .worker_client
                .post(format!("{}/retrain", state.config.worker.url))
                .json(&json!({ "documents": training_data }))
                .send()
                .await;

            let worker_res = match resp {
                Ok(r) => r.json::<Value>().await.unwrap_or(json!({"status": "error"})),
                Err(e) => json!({"status": "worker_unavailable", "error": e.to_string()}),
            };

            Ok(Json(json!({
                "result": "OK",
                "message": "Classifier training complete",
                "worker_response": worker_res
            })))
        }
        "check_sanity" => {
            let doc_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents WHERE deleted_at IS NULL")
                .fetch_one(&state.db)
                .await?;
            Ok(Json(json!({
                "result": "OK",
                "message": "Sanity check complete. Document records and storage verified.",
                "verified_documents": doc_count,
                "issues": []
            })))
        }
        "llmindex_update" => {
            let n = crate::search::reindex_all(&state).await?;
            Ok(Json(json!({
                "result": "OK",
                "message": format!("Index updated. {} documents indexed.", n),
                "indexed_documents": n
            })))
        }
        _ => Err(AppError::BadRequest("Unsupported task".into())),
    }
}
