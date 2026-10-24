//! Document audit log and change tracking.
//!
//! Entries live in `document_history`. Mutation paths take a [`Snapshot`] before changing a
//! document and call [`record_changes`] afterwards, which diffs and stores one `update` entry
//! (plus one entry per changed custom field).

use serde_json::{json, Map, Value};
use sqlx::{Row, SqlitePool};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default, Clone)]
pub struct Snapshot {
    exists: bool,
    scalars: BTreeMap<&'static str, Option<String>>,
    tags: BTreeSet<String>,
    custom_fields: BTreeMap<String, String>,
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Store one raw entry.
pub async fn record(db: &SqlitePool, doc_id: i64, action: &str, changes: Value) {
    let res = sqlx::query(
        "INSERT INTO document_history (document_id, timestamp, action, changes) VALUES ($1, $2, $3, $4)",
    )
    .bind(doc_id)
    .bind(now())
    .bind(action)
    .bind(changes.to_string())
    .execute(db)
    .await;
    if let Err(e) = res {
        tracing::warn!("Could not write history for document {doc_id}: {e}");
    }
}

/// `{"Note Added": ["None", 3]}`-style single-key update.
pub async fn record_simple(db: &SqlitePool, doc_id: i64, key: &str, old: Value, new: Value) {
    record(db, doc_id, "update", json!({ key: [old, new] })).await;
}

pub async fn snapshot(db: &SqlitePool, id: i64) -> Snapshot {
    snapshot_inner(db, id).await.unwrap_or_default()
}

async fn snapshot_inner(db: &SqlitePool, id: i64) -> Result<Snapshot, sqlx::Error> {
    let row = sqlx::query(
        "SELECT d.title, d.created, d.archive_serial_number, d.filename, d.original_filename,
                c.name AS correspondent, t.name AS document_type, s.name AS storage_path
         FROM documents d
         LEFT JOIN correspondents c ON c.id = d.correspondent_id
         LEFT JOIN document_types t ON t.id = d.document_type_id
         LEFT JOIN storage_paths s ON s.id = d.storage_path_id
         WHERE d.id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await?;
    let Some(row) = row else {
        return Ok(Snapshot::default());
    };
    let mut snap = Snapshot {
        exists: true,
        ..Default::default()
    };
    snap.scalars.insert("title", row.get("title"));
    snap.scalars.insert("created", row.get("created"));
    snap.scalars.insert(
        "archive_serial_number",
        row.get::<Option<i64>, _>("archive_serial_number")
            .map(|v| v.to_string()),
    );
    snap.scalars
        .insert("correspondent", row.get("correspondent"));
    snap.scalars
        .insert("document_type", row.get("document_type"));
    snap.scalars.insert("storage_path", row.get("storage_path"));
    snap.scalars
        .insert("original_filename", row.get("original_filename"));

    snap.tags = sqlx::query_scalar::<_, String>(
        "SELECT t.name FROM document_tags dt JOIN tags t ON t.id = dt.tag_id WHERE dt.document_id = $1",
    )
    .bind(id)
    .fetch_all(db)
    .await?
    .into_iter()
    .collect();

    let rows = sqlx::query(
        "SELECT f.name,
                COALESCE(i.value_text, CAST(i.value_int AS TEXT), CAST(i.value_float AS TEXT),
                         i.value_date, i.value_url, i.value_select, i.value_long_text,
                         i.value_monetary, i.value_document_ids, CAST(i.value_bool AS TEXT), '') AS v
         FROM custom_field_instances i JOIN custom_fields f ON f.id = i.field_id
         WHERE i.document_id = $1 AND i.deleted_at IS NULL",
    )
    .bind(id)
    .fetch_all(db)
    .await?;
    for r in rows {
        snap.custom_fields.insert(r.get("name"), r.get("v"));
    }
    Ok(snap)
}

fn s(v: &Option<String>) -> Value {
    json!(v.clone().unwrap_or_else(|| "None".into()))
}

/// Record the creation of a document with its initial values.
pub async fn record_create(db: &SqlitePool, id: i64) {
    let snap = snapshot(db, id).await;
    let mut changes = Map::new();
    for (k, v) in &snap.scalars {
        if v.is_some() {
            changes.insert((*k).into(), json!(["None", v]));
        }
    }
    if !snap.tags.is_empty() {
        changes.insert(
            "tags".into(),
            json!({"type": "m2m", "operation": "add", "objects": snap.tags}),
        );
    }
    record(db, id, "create", Value::Object(changes)).await;
}

/// Diff `before` against the current state and store what changed.
pub async fn record_changes(db: &SqlitePool, id: i64, before: &Snapshot) {
    if !before.exists {
        return;
    }
    let after = snapshot(db, id).await;
    if !after.exists {
        return;
    }
    let mut changes = Map::new();
    for (k, new) in &after.scalars {
        let old = before.scalars.get(k).cloned().unwrap_or(None);
        if &old != new {
            changes.insert((*k).into(), json!([s(&old), s(new)]));
        }
    }
    let added: Vec<&String> = after.tags.difference(&before.tags).collect();
    let removed: Vec<&String> = before.tags.difference(&after.tags).collect();
    if !added.is_empty() {
        changes.insert(
            "tags".into(),
            json!({"type": "m2m", "operation": "add", "objects": added}),
        );
    }
    if !removed.is_empty() {
        let key = if changes.contains_key("tags") {
            "tags_removed"
        } else {
            "tags"
        };
        changes.insert(
            key.into(),
            json!({"type": "m2m", "operation": "remove", "objects": removed}),
        );
    }
    if !changes.is_empty() {
        record(db, id, "update", Value::Object(changes)).await;
    }

    for (name, new) in &after.custom_fields {
        let action = match before.custom_fields.get(name) {
            None => "create",
            Some(old) if old != new => "update",
            _ => continue,
        };
        record(
            db,
            id,
            action,
            json!({"custom_fields": {"type": "custom_field", "field": name, "value": new}}),
        )
        .await;
    }
    for (name, old) in &before.custom_fields {
        if !after.custom_fields.contains_key(name) {
            record(
                db,
                id,
                "delete",
                json!({"custom_fields": {"type": "custom_field", "field": name, "value": old}}),
            )
            .await;
        }
    }
}

/// `GET /api/documents/{id}/history/` payload, newest first.
pub async fn list(db: &SqlitePool, id: i64) -> Result<Vec<Value>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, timestamp, action, changes FROM document_history
         WHERE document_id = $1 ORDER BY timestamp DESC, id DESC",
    )
    .bind(id)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let changes: String = r.get("changes");
            json!({
                "id": r.get::<i64, _>("id"),
                "timestamp": r.get::<String, _>("timestamp"),
                "action": r.get::<String, _>("action"),
                "changes": serde_json::from_str::<Value>(&changes).unwrap_or(json!({})),
                "actor": Value::Null,
            })
        })
        .collect())
}
