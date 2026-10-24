use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use base64::Engine;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use tracing::{info, warn};

use crate::error::AppError;
use crate::ingest::ConsumeRequest;
use crate::state::AppState;

#[derive(Serialize, Deserialize, Default)]
pub struct MailAccountPayload {
    pub name: Option<String>,
    pub imap_server: Option<String>,
    pub imap_port: Option<i64>,
    pub imap_security: Option<i64>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub is_token: Option<bool>,
    pub character_set: Option<String>,
    pub account_type: Option<i64>,
}

pub async fn list_mail_accounts(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query(
        "SELECT id, name, imap_server, imap_port, imap_security, username, is_token, character_set, account_type
         FROM mail_accounts ORDER BY name ASC"
    )
    .fetch_all(&state.db)
    .await?;

    let results: Vec<Value> = rows.into_iter().map(|r| {
        let id: i64 = r.get("id");
        let name: String = r.get("name");
        let imap_server: String = r.get("imap_server");
        let imap_port: Option<i64> = r.get("imap_port");
        let imap_security: i64 = r.get("imap_security");
        let username: String = r.get("username");
        let is_token: bool = r.get("is_token");
        let character_set: String = r.get("character_set");
        let account_type: i64 = r.get("account_type");

        json!({
            "id": id,
            "name": name,
            "imap_server": imap_server,
            "imap_port": imap_port,
            "imap_security": imap_security,
            "username": username,
            "is_token": is_token,
            "character_set": character_set,
            "account_type": account_type
        })
    }).collect();

    Ok(Json(json!({
        "count": results.len(),
        "results": results
    })))
}

pub async fn create_mail_account(
    State(state): State<AppState>,
    Json(payload): Json<MailAccountPayload>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let name = payload.name.unwrap_or_default();
    let imap_server = payload.imap_server.unwrap_or_default();
    let port = payload.imap_port.unwrap_or(993);
    let security = payload.imap_security.unwrap_or(2);
    let username = payload.username.unwrap_or_default();
    let pass = payload.password.unwrap_or_default();
    let is_tok = payload.is_token.unwrap_or(false);
    let charset = payload.character_set.unwrap_or_else(|| "UTF-8".to_string());
    let acc_type = payload.account_type.unwrap_or(1);

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO mail_accounts (name, imap_server, imap_port, imap_security, username, password,
                                   is_token, character_set, account_type)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id"
    )
    .bind(&name)
    .bind(&imap_server)
    .bind(port)
    .bind(security)
    .bind(&username)
    .bind(&pass)
    .bind(is_tok)
    .bind(&charset)
    .bind(acc_type)
    .fetch_one(&state.db)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": id,
            "name": name,
            "imap_server": imap_server,
            "imap_port": port,
            "imap_security": security,
            "username": username,
            "is_token": is_tok,
            "character_set": charset,
            "account_type": acc_type
        })),
    ))
}

pub async fn update_mail_account(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<MailAccountPayload>,
) -> Result<Json<Value>, AppError> {
    if let Some(name) = &payload.name {
        sqlx::query("UPDATE mail_accounts SET name = $1 WHERE id = $2")
            .bind(name)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(server) = &payload.imap_server {
        sqlx::query("UPDATE mail_accounts SET imap_server = $1 WHERE id = $2")
            .bind(server)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(port) = payload.imap_port {
        sqlx::query("UPDATE mail_accounts SET imap_port = $1 WHERE id = $2")
            .bind(port)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(sec) = payload.imap_security {
        sqlx::query("UPDATE mail_accounts SET imap_security = $1 WHERE id = $2")
            .bind(sec)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(user) = &payload.username {
        sqlx::query("UPDATE mail_accounts SET username = $1 WHERE id = $2")
            .bind(user)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(pass) = &payload.password {
        sqlx::query("UPDATE mail_accounts SET password = $1 WHERE id = $2")
            .bind(pass)
            .bind(id)
            .execute(&state.db)
            .await?;
    }

    Ok(Json(json!({ "id": id, "success": true })))
}

pub async fn delete_mail_account(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    sqlx::query("DELETE FROM mail_accounts WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Test connection to an IMAP mail account.
pub async fn test_mail_account(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let url = format!("{}/mail/test", state.config.worker.url);
    let resp = state
        .worker_client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Worker communication error: {e}")))?;

    let status = resp.status();
    let body: Value = resp.json().await.unwrap_or(json!({
        "success": false,
        "detail": format!("HTTP {}", status)
    }));

    Ok(Json(body))
}

pub async fn test_mail_account_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let row = sqlx::query(
        "SELECT imap_server, imap_port, imap_security, username, password FROM mail_accounts WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    let Some(r) = row else {
        return Err(AppError::NotFound("Mail account not found".into()));
    };

    let payload = json!({
        "imap_server": r.get::<String, _>("imap_server"),
        "imap_port": r.get::<Option<i64>, _>("imap_port"),
        "imap_security": r.get::<i64, _>("imap_security"),
        "username": r.get::<String, _>("username"),
        "password": r.get::<String, _>("password"),
    });

    let url = format!("{}/mail/test", state.config.worker.url);
    let resp = state
        .worker_client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Worker communication error: {e}")))?;

    let body: Value = resp.json().await.unwrap_or(json!({ "success": false }));
    Ok(Json(body))
}

#[derive(Serialize, Deserialize, Default)]
pub struct MailRulePayload {
    pub account: Option<i64>,
    pub name: Option<String>,
    pub order: Option<i64>,
    pub enabled: Option<bool>,
    pub folder: Option<String>,
    pub filter_from: Option<String>,
    pub filter_to: Option<String>,
    pub filter_subject: Option<String>,
    pub filter_body: Option<String>,
    pub filter_attachment_filename: Option<String>,
    pub action: Option<i64>,
    pub assign_tags: Option<Vec<i64>>,
    pub assign_document_type: Option<i64>,
    pub assign_correspondent: Option<i64>,
    pub consumption_scope: Option<i64>,
}

pub async fn list_mail_rules(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query(
        r#"SELECT id, account_id, name, "order", enabled, folder, filter_from, filter_to,
                  filter_subject, filter_body, filter_attachment_filename, action,
                  assign_tags, assign_document_type_id, assign_correspondent_id, consumption_scope
           FROM mail_rules ORDER BY "order" ASC, id ASC"#
    )
    .fetch_all(&state.db)
    .await?;

    let results: Vec<Value> = rows.into_iter().map(|r| {
        let id: i64 = r.get("id");
        let account_id: i64 = r.get("account_id");
        let name: String = r.get("name");
        let order: i64 = r.get("order");
        let enabled: bool = r.get("enabled");
        let folder: String = r.get("folder");
        let filter_from: Option<String> = r.get("filter_from");
        let filter_to: Option<String> = r.get("filter_to");
        let filter_subject: Option<String> = r.get("filter_subject");
        let filter_body: Option<String> = r.get("filter_body");
        let filter_attachment_filename: Option<String> = r.get("filter_attachment_filename");
        let action: i64 = r.get("action");
        let assign_tags_raw: Option<String> = r.get("assign_tags");
        let assign_tags: Vec<i64> = assign_tags_raw
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let assign_document_type: Option<i64> = r.get("assign_document_type_id");
        let assign_correspondent: Option<i64> = r.get("assign_correspondent_id");
        let consumption_scope: i64 = r.get("consumption_scope");

        json!({
            "id": id,
            "account": account_id,
            "name": name,
            "order": order,
            "enabled": enabled,
            "folder": folder,
            "filter_from": filter_from,
            "filter_to": filter_to,
            "filter_subject": filter_subject,
            "filter_body": filter_body,
            "filter_attachment_filename": filter_attachment_filename,
            "action": action,
            "assign_tags": assign_tags,
            "assign_document_type": assign_document_type,
            "assign_correspondent": assign_correspondent,
            "consumption_scope": consumption_scope
        })
    }).collect();

    Ok(Json(json!({
        "count": results.len(),
        "results": results
    })))
}

pub async fn create_mail_rule(
    State(state): State<AppState>,
    Json(payload): Json<MailRulePayload>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let account = payload.account.unwrap_or(1);
    let name = payload.name.unwrap_or_default();
    let order = payload.order.unwrap_or(0);
    let enabled = payload.enabled.unwrap_or(true);
    let folder = payload.folder.unwrap_or_else(|| "INBOX".to_string());
    let action = payload.action.unwrap_or(3);
    let tags_str = payload.assign_tags.as_ref().map(|t| serde_json::to_string(t).unwrap_or_default());
    let scope = payload.consumption_scope.unwrap_or(1);

    let id: i64 = sqlx::query_scalar(
        r#"INSERT INTO mail_rules (account_id, name, "order", enabled, folder, filter_from, filter_to,
                                  filter_subject, filter_body, filter_attachment_filename, action,
                                  assign_tags, assign_document_type_id, assign_correspondent_id, consumption_scope)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15) RETURNING id"#
    )
    .bind(account)
    .bind(&name)
    .bind(order)
    .bind(enabled)
    .bind(&folder)
    .bind(&payload.filter_from)
    .bind(&payload.filter_to)
    .bind(&payload.filter_subject)
    .bind(&payload.filter_body)
    .bind(&payload.filter_attachment_filename)
    .bind(action)
    .bind(tags_str)
    .bind(payload.assign_document_type)
    .bind(payload.assign_correspondent)
    .bind(scope)
    .fetch_one(&state.db)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": id,
            "account": account,
            "name": name,
            "order": order,
            "enabled": enabled,
            "folder": folder,
            "action": action,
            "assign_tags": payload.assign_tags,
            "consumption_scope": scope
        })),
    ))
}

pub async fn update_mail_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<MailRulePayload>,
) -> Result<Json<Value>, AppError> {
    if let Some(name) = &payload.name {
        sqlx::query("UPDATE mail_rules SET name = $1 WHERE id = $2")
            .bind(name)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(folder) = &payload.folder {
        sqlx::query("UPDATE mail_rules SET folder = $1 WHERE id = $2")
            .bind(folder)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(enabled) = payload.enabled {
        sqlx::query("UPDATE mail_rules SET enabled = $1 WHERE id = $2")
            .bind(enabled)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(tags) = &payload.assign_tags {
        let s = serde_json::to_string(tags).unwrap_or_default();
        sqlx::query("UPDATE mail_rules SET assign_tags = $1 WHERE id = $2")
            .bind(s)
            .bind(id)
            .execute(&state.db)
            .await?;
    }

    Ok(Json(json!({ "id": id, "success": true })))
}

pub async fn delete_mail_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    sqlx::query("DELETE FROM mail_rules WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_processed_mail(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query(
        "SELECT id, rule_id, uid, subject, received, processed FROM processed_mail ORDER BY processed DESC LIMIT 100"
    )
    .fetch_all(&state.db)
    .await?;

    let results: Vec<Value> = rows.into_iter().map(|r| {
        let id: i64 = r.get("id");
        let rule_id: Option<i64> = r.get("rule_id");
        let uid: String = r.get("uid");
        let subject: Option<String> = r.get("subject");
        let received: Option<String> = r.get("received");
        let processed: String = r.get("processed");
        json!({
            "id": id,
            "rule": rule_id,
            "uid": uid,
            "subject": subject,
            "received": received,
            "processed": processed
        })
    }).collect();

    Ok(Json(json!({
        "count": results.len(),
        "results": results
    })))
}

pub async fn oauth_callback(
    Json(payload): Json<Value>,
) -> Json<Value> {
    Json(json!({
        "status": "success",
        "message": "OAuth token callback processed",
        "data": payload
    }))
}

// ---------------------------------------------------------------------------
// Processing Engine
// ---------------------------------------------------------------------------

/// Process emails for a single mail rule.
pub async fn process_mail_rule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let rule_row = sqlx::query(
        "SELECT r.id, r.account_id, r.name, r.folder, r.filter_from, r.filter_subject,
                r.assign_tags, r.assign_document_type_id, r.assign_correspondent_id,
                a.imap_server, a.imap_port, a.imap_security, a.username, a.password
         FROM mail_rules r
         JOIN mail_accounts a ON r.account_id = a.id
         WHERE r.id = $1"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    let Some(r) = rule_row else {
        return Err(AppError::NotFound("Mail rule or account not found".into()));
    };

    let count = execute_single_rule(&state, &r).await?;

    Ok(Json(json!({
        "status": "success",
        "rule_id": id,
        "attachments_ingested": count
    })))
}

/// Process all active mail rules for a mail account.
pub async fn process_mail_account(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let rules = sqlx::query(
        "SELECT r.id, r.account_id, r.name, r.folder, r.filter_from, r.filter_subject,
                r.assign_tags, r.assign_document_type_id, r.assign_correspondent_id,
                a.imap_server, a.imap_port, a.imap_security, a.username, a.password
         FROM mail_rules r
         JOIN mail_accounts a ON r.account_id = a.id
         WHERE r.account_id = $1 AND r.enabled = 1
         ORDER BY r.\"order\" ASC, r.id ASC"
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let mut total_ingested = 0;
    for r in &rules {
        if let Ok(c) = execute_single_rule(&state, r).await {
            total_ingested += c;
        }
    }

    Ok(Json(json!({
        "status": "success",
        "account_id": id,
        "rules_executed": rules.len(),
        "attachments_ingested": total_ingested
    })))
}

/// Core function to execute one mail rule.
async fn execute_single_rule(
    state: &AppState,
    row: &sqlx::sqlite::SqliteRow,
) -> Result<usize, AppError> {
    let rule_id: i64 = row.get("id");
    let folder: String = row.get("folder");
    let filter_from: Option<String> = row.get("filter_from");
    let filter_subject: Option<String> = row.get("filter_subject");
    let assign_tags_raw: Option<String> = row.get("assign_tags");
    let assign_corr: Option<i64> = row.get("assign_correspondent_id");
    let assign_type: Option<i64> = row.get("assign_document_type_id");

    let tags: Vec<i64> = assign_tags_raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let payload = json!({
        "imap_server": row.get::<String, _>("imap_server"),
        "imap_port": row.get::<Option<i64>, _>("imap_port"),
        "imap_security": row.get::<i64, _>("imap_security"),
        "username": row.get::<String, _>("username"),
        "password": row.get::<String, _>("password"),
        "folder": folder,
        "filter_from": filter_from,
        "filter_subject": filter_subject,
        "limit": 25,
    });

    let url = format!("{}/mail/fetch", state.config.worker.url);
    let resp = state
        .worker_client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Worker fetch failed: {e}")))?;

    if !resp.status().is_success() {
        return Err(AppError::Internal("Worker returned an error while fetching mail".into()));
    }

    let fetch_result: Value = resp.json().await.map_err(|e| AppError::Internal(e.to_string()))?;
    let attachments = fetch_result
        .get("attachments")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut ingested_count = 0;

    for att in attachments {
        let msg_id = att.get("message_id").and_then(|v| v.as_str()).unwrap_or("");
        let filename = att.get("filename").and_then(|v| v.as_str()).unwrap_or("document.pdf");
        let subject = att.get("subject").and_then(|v| v.as_str()).unwrap_or("");
        let b64 = att.get("data_b64").and_then(|v| v.as_str()).unwrap_or("");

        let uid_key = if !msg_id.is_empty() {
            format!("{}_{}", msg_id, filename)
        } else {
            format!("{}_{}", subject, filename)
        };

        // Check if already processed
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM processed_mail WHERE uid = $1")
            .bind(&uid_key)
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

        if exists > 0 {
            continue;
        }

        // Decode attachment data
        let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(b64) else {
            continue;
        };

        let req = ConsumeRequest {
            bytes,
            filename: filename.to_string(),
            title: if !subject.is_empty() { Some(subject.to_string()) } else { None },
            correspondent: assign_corr,
            document_type: assign_type,
            storage_path: None,
            tags: tags.clone(),
            created: Some(Utc::now().to_rfc3339()),
            asn: None,
            custom_fields: vec![],
            source: "mail_fetch".to_string(),
            root_document_id: None,
            version_index: None,
            version_label: None,
        };

        if let Ok(_task_id) = crate::ingest::submit(state, req).await {
            ingested_count += 1;
            let now_str = Utc::now().to_rfc3339();
            let _ = sqlx::query(
                "INSERT INTO processed_mail (rule_id, uid, subject, received, processed, folder, status)
                 VALUES ($1, $2, $3, $4, $5, $6, 'SUCCESS')"
            )
            .bind(rule_id)
            .bind(&uid_key)
            .bind(subject)
            .bind(&now_str)
            .bind(&now_str)
            .bind(&folder)
            .execute(&state.db)
            .await;
        }
    }

    info!(
        "Mail rule #{} executed: {} attachments ingested",
        rule_id, ingested_count
    );

    Ok(ingested_count)
}

/// Process all active mail accounts and rules across the system.
pub async fn process_all_active_mail(state: &AppState) -> Result<usize, AppError> {
    let active_accounts: Vec<i64> = sqlx::query_scalar("SELECT id FROM mail_accounts")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let mut total = 0;
    for acc_id in active_accounts {
        let rules = sqlx::query(
            "SELECT r.id, r.account_id, r.name, r.folder, r.filter_from, r.filter_subject,
                    r.assign_tags, r.assign_document_type_id, r.assign_correspondent_id,
                    a.imap_server, a.imap_port, a.imap_security, a.username, a.password
             FROM mail_rules r
             JOIN mail_accounts a ON r.account_id = a.id
             WHERE r.account_id = $1 AND r.enabled = 1
             ORDER BY r.\"order\" ASC, r.id ASC"
        )
        .bind(acc_id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        for r in rules {
            if let Ok(c) = execute_single_rule(state, &r).await {
                total += c;
            }
        }
    }

    Ok(total)
}

/// Background poller task spawned at application startup.
pub fn spawn_mail_poller(state: AppState) {
    tokio::spawn(async move {
        // Wait 15 seconds after startup before first check
        tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
        loop {
            if let Err(e) = process_all_active_mail(&state).await {
                warn!("Mail background poller warning: {e}");
            }
            // Poll every 10 minutes (600 seconds)
            tokio::time::sleep(tokio::time::Duration::from_secs(600)).await;
        }
    });
}
