//! Local-first AI: embeddings (worker), semantic search (cosine in Rust), LLM client
//! (any OpenAI-compatible endpoint, e.g. Ollama / llama.cpp) and maintenance tasks.
//!
//! Everything degrades gracefully: when `ai.enabled` is false nothing is indexed and AI
//! endpoints answer 400 JSON; when the worker or LLM is unreachable errors are JSON, never panics.

use crate::error::AppError;
use crate::state::AppState;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Row;

pub const CHUNK_CHARS: usize = 1000;
pub const MAX_CHUNKS: usize = 24;

pub fn require_enabled(state: &AppState) -> Result<(), AppError> {
    if state.config.ai.enabled {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "AI is required for this feature. Enable it with [ai] enabled = true in filedeck.toml \
             (or FILEDECK_AI_ENABLED=true) and point ai.base_url at a local OpenAI-compatible server."
                .into(),
        ))
    }
}

/// Split text into ~CHUNK_CHARS pieces on whitespace boundaries.
pub fn chunk_text(title: &str, content: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut cur = String::new();
    if !title.trim().is_empty() {
        cur.push_str(title.trim());
        cur.push('\n');
    }
    for word in content.split_whitespace() {
        if cur.len() + word.len() + 1 > CHUNK_CHARS && !cur.trim().is_empty() {
            chunks.push(std::mem::take(&mut cur).trim().to_string());
            if chunks.len() >= MAX_CHUNKS {
                return chunks;
            }
        }
        if !cur.is_empty() && !cur.ends_with('\n') {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.trim().is_empty() {
        chunks.push(cur.trim().to_string());
    }
    chunks
}

pub struct Embedded {
    pub vectors: Vec<Vec<f32>>,
    pub backend: String,
    pub model: String,
}

pub async fn embed_texts(state: &AppState, texts: &[String]) -> Result<Embedded, AppError> {
    let resp = state
        .worker_client
        .post(format!("{}/embed", state.config.worker.url))
        .json(&json!({ "texts": texts }))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Embedding worker unreachable: {e}")))?;
    if !resp.status().is_success() {
        return Err(AppError::Internal(format!(
            "Embedding worker returned {}",
            resp.status()
        )));
    }
    let v: Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Bad embedding response: {e}")))?;
    let vectors: Vec<Vec<f32>> = v["embeddings"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|row| {
                    row.as_array()
                        .map(|r| r.iter().filter_map(|x| x.as_f64()).map(|x| x as f32).collect())
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default();
    if vectors.len() != texts.len() {
        return Err(AppError::Internal("Embedding count mismatch".into()));
    }
    Ok(Embedded {
        vectors,
        backend: v["backend"].as_str().unwrap_or("unknown").to_string(),
        model: v["model"].as_str().unwrap_or("").to_string(),
    })
}

fn to_blob(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn from_blob(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0f32, 0f32, 0f32);
    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na.sqrt() * nb.sqrt())
    }
}

/// (Re)compute and store the embeddings of one document. Skips when content is unchanged.
/// Returns the number of chunks stored (0 = nothing to do / skipped).
pub async fn index_document(state: &AppState, id: i64) -> Result<usize, AppError> {
    require_enabled(state)?;
    let row = sqlx::query("SELECT title, content FROM documents WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    let Some(row) = row else {
        remove_document(state, id).await?;
        return Ok(0);
    };
    let title: String = row.get("title");
    let content: String = row.get("content");
    let hash = hex::encode(Sha256::digest(format!("{title}\n{content}").as_bytes()));
    let prev: Option<String> =
        sqlx::query_scalar("SELECT content_hash FROM ai_index_state WHERE document_id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
    if prev.as_deref() == Some(hash.as_str()) {
        return Ok(0);
    }
    let chunks = chunk_text(&title, &content);
    if chunks.is_empty() {
        remove_document(state, id).await?;
        return Ok(0);
    }
    let emb = embed_texts(state, &chunks).await?;
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM ai_chunks WHERE document_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    for (i, (text, vec)) in chunks.iter().zip(&emb.vectors).enumerate() {
        sqlx::query(
            "INSERT INTO ai_chunks (document_id, chunk_index, text, backend, model, dim, vector)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(id)
        .bind(i as i64)
        .bind(text)
        .bind(&emb.backend)
        .bind(&emb.model)
        .bind(vec.len() as i64)
        .bind(to_blob(vec))
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query(
        "INSERT INTO ai_index_state (document_id, content_hash, backend, chunk_count, indexed_at)
         VALUES ($1,$2,$3,$4,$5)
         ON CONFLICT(document_id) DO UPDATE SET content_hash=excluded.content_hash,
           backend=excluded.backend, chunk_count=excluded.chunk_count, indexed_at=excluded.indexed_at",
    )
    .bind(id)
    .bind(&hash)
    .bind(&emb.backend)
    .bind(chunks.len() as i64)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(chunks.len())
}

pub async fn remove_document(state: &AppState, id: i64) -> Result<(), AppError> {
    sqlx::query("DELETE FROM ai_chunks WHERE document_id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    sqlx::query("DELETE FROM ai_index_state WHERE document_id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(())
}

/// Fire-and-forget indexing hook for ingest/update paths. No-op when AI is disabled.
pub fn spawn_index(state: &AppState, id: i64) {
    if !state.config.ai.enabled {
        return;
    }
    let st = state.clone();
    tokio::spawn(async move {
        if let Err(e) = index_document(&st, id).await {
            tracing::warn!("AI indexing of document {id} failed: {e}");
        }
    });
}

#[derive(Debug, Clone)]
pub struct Hit {
    pub document_id: i64,
    pub chunk_index: i64,
    pub text: String,
    pub score: f32,
}

/// Top-k chunks most similar to `query` across non-deleted documents (optionally filtered by document_id).
pub async fn search_chunks(
    state: &AppState,
    query: &str,
    limit: usize,
    doc_id: Option<i64>,
) -> Result<Vec<Hit>, AppError> {
    require_enabled(state)?;
    let emb = embed_texts(state, &[query.to_string()]).await?;
    let query_vec = match emb.vectors.first() {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };

    let rows = if let Some(id) = doc_id {
        sqlx::query(
            "SELECT c.document_id, c.chunk_index, c.text, c.vector
             FROM ai_chunks c
             JOIN documents d ON d.id = c.document_id
             WHERE c.document_id = $1 AND d.deleted_at IS NULL",
        )
        .bind(id)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query(
            "SELECT c.document_id, c.chunk_index, c.text, c.vector
             FROM ai_chunks c
             JOIN documents d ON d.id = c.document_id
             WHERE d.deleted_at IS NULL",
        )
        .fetch_all(&state.db)
        .await?
    };

    let mut hits: Vec<Hit> = rows
        .into_iter()
        .map(|r| {
            let document_id: i64 = r.get("document_id");
            let chunk_index: i64 = r.get("chunk_index");
            let text: String = r.get("text");
            let vector_blob: Vec<u8> = r.get("vector");
            let vec = from_blob(&vector_blob);
            let score = cosine(query_vec, &vec);
            Hit {
                document_id,
                chunk_index,
                text,
                score,
            }
        })
        .collect();

    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    hits.truncate(limit);
    Ok(hits)
}

pub async fn reindex_all(state: &AppState) -> Result<usize, AppError> {
    require_enabled(state)?;
    let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM documents WHERE deleted_at IS NULL")
        .fetch_all(&state.db)
        .await?;
    let mut count = 0;
    for id in ids {
        if let Ok(n) = index_document(state, id).await {
            count += n;
        }
    }
    Ok(count)
}
