//! AI endpoints: Document Chat with SSE streaming, Global Semantic Chat,
//! and PDF Table Extraction.

use crate::ai;
use crate::error::AppError;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    Json,
};
use futures_util::StreamExt;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use std::convert::Infallible;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: Option<String>,
    pub query: Option<String>,
    pub history: Option<Vec<ChatMessage>>,
    pub stream: Option<bool>,
}

/// POST /api/documents/:id/chat/
/// Chat with a single document via streaming SSE or standard JSON.
pub async fn document_chat(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<ChatRequest>,
) -> Result<Response, AppError> {
    let user_prompt = payload
        .message
        .or(payload.query)
        .unwrap_or_default()
        .trim()
        .to_string();

    if user_prompt.is_empty() {
        return Err(AppError::BadRequest("Message prompt cannot be empty".into()));
    }

    // Fetch target document
    let doc = sqlx::query(
        "SELECT d.id, d.title, d.content, d.created, c.name as correspondent_name, dt.name as type_name
         FROM documents d
         LEFT JOIN correspondents c ON c.id = d.correspondent_id
         LEFT JOIN document_types dt ON dt.id = d.document_type_id
         WHERE d.id = $1 AND d.deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Document {id} not found")))?;

    let title: String = doc.get("title");
    let content: String = doc.get("content");
    let created: String = doc.get("created");
    let correspondent: Option<String> = doc.get("correspondent_name");
    let doc_type: Option<String> = doc.get("type_name");

    // Gather most relevant excerpts
    let mut excerpts = String::new();
    let mut sources = Vec::new();

    if state.config.ai.enabled {
        if let Ok(hits) = ai::search_chunks(&state, &user_prompt, 4, Some(id)).await {
            for hit in hits {
                excerpts.push_str(&format!("\n[Chunk {}]:\n{}\n", hit.chunk_index, hit.text));
                sources.push(json!({
                    "document_id": hit.document_id,
                    "chunk_index": hit.chunk_index,
                    "score": hit.score,
                }));
            }
        }
    }

    if excerpts.is_empty() {
        // Fallback to top of document content
        let snippet: String = content.chars().take(3500).collect();
        excerpts = format!("\n[Document Content Excerpt]:\n{snippet}\n");
    }

    let system_prompt = format!(
        "You are an expert document assistant in FileDeck. Answer questions accurately, concisely, and helpfully based strictly on the document provided.\n\n\
         Document Information:\n\
         - Title: {title}\n\
         - Date: {created}\n\
         - Correspondent: {}\n\
         - Type: {}\n\n\
         Document Excerpts:\n\
         {excerpts}\n\n\
         If the requested information is not present in the document excerpts, state that clearly and courteously.",
        correspondent.as_deref().unwrap_or("Unknown"),
        doc_type.as_deref().unwrap_or("Unspecified")
    );

    let mut messages = vec![json!({
        "role": "system",
        "content": system_prompt
    })];

    if let Some(hist) = payload.history {
        for msg in hist {
            if msg.role == "user" || msg.role == "assistant" {
                messages.push(json!({
                    "role": msg.role,
                    "content": msg.content
                }));
            }
        }
    }

    messages.push(json!({
        "role": "user",
        "content": user_prompt
    }));

    let wants_stream = payload.stream.unwrap_or(true);
    let llm_url = format!("{}/chat/completions", state.config.ai.base_url.trim_end_matches('/'));
    let llm_model = state.config.ai.llm_model.clone();
    let api_key = state.config.ai.api_key.clone();

    if wants_stream {
        let stream = async_stream::stream! {
            let mut req = state.worker_client
                .post(&llm_url)
                .json(&json!({
                    "model": llm_model,
                    "messages": messages,
                    "stream": true,
                    "temperature": 0.2
                }));

            if !api_key.is_empty() {
                req = req.bearer_auth(&api_key);
            }

            match req.send().await {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        let err_msg = format!("LLM server returned HTTP {}", resp.status());
                        yield Ok::<Event, Infallible>(Event::default().data(json!({"error": err_msg}).to_string()));
                        yield Ok(Event::default().data("[DONE]"));
                        return;
                    }

                    let mut byte_stream = resp.bytes_stream();
                    let mut buffer = String::new();

                    while let Some(chunk_res) = byte_stream.next().await {
                        if let Ok(chunk) = chunk_res {
                            buffer.push_str(&String::from_utf8_lossy(&chunk));
                            while let Some(pos) = buffer.find('\n') {
                                let line = buffer[..pos].trim().to_string();
                                buffer.drain(..=pos);
                                if line.starts_with("data: ") {
                                    let data_str = &line["data: ".len()..];
                                    if data_str == "[DONE]" {
                                        yield Ok(Event::default().data("[DONE]"));
                                        return;
                                    }
                                    if let Ok(parsed) = serde_json::from_str::<Value>(data_str) {
                                        if let Some(delta) = parsed["choices"][0]["delta"]["content"].as_str() {
                                            yield Ok(Event::default().data(json!({"delta": delta}).to_string()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    yield Ok(Event::default().data("[DONE]"));
                }
                Err(err) => {
                    let fallback_msg = format!(
                        "Note: AI engine at {} is currently unreachable ({err}). You can configure your local model runner (such as Ollama or llama.cpp) in filedeck.toml [ai].",
                        llm_url
                    );
                    yield Ok(Event::default().data(json!({"delta": fallback_msg}).to_string()));
                    yield Ok(Event::default().data("[DONE]"));
                }
            }
        };

        Ok(Sse::new(stream).keep_alive(KeepAlive::default()).into_response())
    } else {
        let mut req = state.worker_client
            .post(&llm_url)
            .json(&json!({
                "model": llm_model,
                "messages": messages,
                "stream": false,
                "temperature": 0.2
            }));

        if !api_key.is_empty() {
            req = req.bearer_auth(&api_key);
        }

        match req.send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    return Ok(Json(json!({
                        "error": format!("LLM server returned HTTP {}", resp.status()),
                        "sources": sources
                    })).into_response());
                }
                let body: Value = resp.json().await.unwrap_or(json!({}));
                let answer = body["choices"][0]["message"]["content"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                Ok(Json(json!({
                    "answer": answer,
                    "sources": sources
                })).into_response())
            }
            Err(e) => {
                Ok(Json(json!({
                    "answer": format!("AI service is offline: {e}"),
                    "sources": sources
                })).into_response())
            }
        }
    }
}

/// POST /api/chat/
/// Global RAG chat across all documents in FileDeck.
pub async fn global_chat(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Result<Response, AppError> {
    let user_prompt = payload
        .message
        .or(payload.query)
        .unwrap_or_default()
        .trim()
        .to_string();

    if user_prompt.is_empty() {
        return Err(AppError::BadRequest("Message prompt cannot be empty".into()));
    }

    let mut context_blocks = Vec::new();
    let mut sources = Vec::new();

    if state.config.ai.enabled {
        if let Ok(hits) = ai::search_chunks(&state, &user_prompt, 6, None).await {
            for hit in hits {
                // Fetch document title for each hit
                let title: Option<String> = sqlx::query_scalar("SELECT title FROM documents WHERE id = $1")
                    .bind(hit.document_id)
                    .fetch_optional(&state.db)
                    .await
                    .unwrap_or(None);

                let doc_title = title.unwrap_or_else(|| format!("Doc #{}", hit.document_id));
                context_blocks.push(format!("[Source: {} (ID: {})]:\n{}", doc_title, hit.document_id, hit.text));
                sources.push(json!({
                    "document_id": hit.document_id,
                    "document_title": doc_title,
                    "chunk_index": hit.chunk_index,
                    "score": hit.score,
                }));
            }
        }
    }

    let context_text = if context_blocks.is_empty() {
        "No matching indexed document excerpts were found for this query.".to_string()
    } else {
        context_blocks.join("\n\n---\n\n")
    };

    let system_prompt = format!(
        "You are FileDeck's global document intelligence assistant. Answer the user's questions based on the retrieved document excerpts below. Cite source documents when stating facts.\n\n\
         Retrieved Document Excerpts:\n\
         {}\n\n\
         If the excerpts do not contain enough information to answer, state that accurately.",
        context_text
    );

    let mut messages = vec![json!({
        "role": "system",
        "content": system_prompt
    })];

    if let Some(hist) = payload.history {
        for msg in hist {
            if msg.role == "user" || msg.role == "assistant" {
                messages.push(json!({
                    "role": msg.role,
                    "content": msg.content
                }));
            }
        }
    }

    messages.push(json!({
        "role": "user",
        "content": user_prompt
    }));

    let wants_stream = payload.stream.unwrap_or(true);
    let llm_url = format!("{}/chat/completions", state.config.ai.base_url.trim_end_matches('/'));
    let llm_model = state.config.ai.llm_model.clone();
    let api_key = state.config.ai.api_key.clone();

    if wants_stream {
        let stream = async_stream::stream! {
            let mut req = state.worker_client
                .post(&llm_url)
                .json(&json!({
                    "model": llm_model,
                    "messages": messages,
                    "stream": true,
                    "temperature": 0.2
                }));

            if !api_key.is_empty() {
                req = req.bearer_auth(&api_key);
            }

            match req.send().await {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        let err_msg = format!("LLM server returned HTTP {}", resp.status());
                        yield Ok::<Event, Infallible>(Event::default().data(json!({"error": err_msg}).to_string()));
                        yield Ok(Event::default().data("[DONE]"));
                        return;
                    }

                    let mut byte_stream = resp.bytes_stream();
                    let mut buffer = String::new();

                    while let Some(chunk_res) = byte_stream.next().await {
                        if let Ok(chunk) = chunk_res {
                            buffer.push_str(&String::from_utf8_lossy(&chunk));
                            while let Some(pos) = buffer.find('\n') {
                                let line = buffer[..pos].trim().to_string();
                                buffer.drain(..=pos);
                                if line.starts_with("data: ") {
                                    let data_str = &line["data: ".len()..];
                                    if data_str == "[DONE]" {
                                        yield Ok(Event::default().data("[DONE]"));
                                        return;
                                    }
                                    if let Ok(parsed) = serde_json::from_str::<Value>(data_str) {
                                        if let Some(delta) = parsed["choices"][0]["delta"]["content"].as_str() {
                                            yield Ok(Event::default().data(json!({"delta": delta}).to_string()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    yield Ok(Event::default().data("[DONE]"));
                }
                Err(err) => {
                    let fallback_msg = format!(
                        "Note: AI engine at {} is currently unreachable ({err}). Ensure your local LLM (e.g., Ollama or llama.cpp) is running.",
                        llm_url
                    );
                    yield Ok(Event::default().data(json!({"delta": fallback_msg}).to_string()));
                    yield Ok(Event::default().data("[DONE]"));
                }
            }
        };

        Ok(Sse::new(stream).keep_alive(KeepAlive::default()).into_response())
    } else {
        let mut req = state.worker_client
            .post(&llm_url)
            .json(&json!({
                "model": llm_model,
                "messages": messages,
                "stream": false,
                "temperature": 0.2
            }));

        if !api_key.is_empty() {
            req = req.bearer_auth(&api_key);
        }

        match req.send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    return Ok(Json(json!({
                        "error": format!("LLM server returned HTTP {}", resp.status()),
                        "sources": sources
                    })).into_response());
                }
                let body: Value = resp.json().await.unwrap_or(json!({}));
                let answer = body["choices"][0]["message"]["content"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                Ok(Json(json!({
                    "answer": answer,
                    "sources": sources
                })).into_response())
            }
            Err(e) => {
                Ok(Json(json!({
                    "answer": format!("AI service is offline: {e}"),
                    "sources": sources
                })).into_response())
            }
        }
    }
}

/// GET /api/documents/:id/tables/
/// Extract all tabular structures from a document's PDF via the worker.
pub async fn get_document_tables(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let doc = sqlx::query(
        "SELECT filename, archive_filename, original_filename FROM documents WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Document {id} not found")))?;

    let filename: Option<String> = doc.get("filename");
    let archive_filename: Option<String> = doc.get("archive_filename");
    let original_filename: Option<String> = doc.get("original_filename");

    let file_to_serve = if let Some(ref arch) = archive_filename {
        state.config.storage.media_root.join("archive").join(arch)
    } else if let Some(ref fname) = filename {
        state
            .config
            .storage
            .media_root
            .join("originals")
            .join(fname)
    } else {
        return Ok(Json(json!({"tables": [], "count": 0})));
    };

    if !file_to_serve.exists() {
        return Ok(Json(json!({"tables": [], "count": 0})));
    }

    let file_bytes = tokio::fs::read(&file_to_serve).await?;
    let file_part = Part::bytes(file_bytes)
        .file_name(original_filename.unwrap_or_else(|| "document.pdf".into()))
        .mime_str("application/pdf")
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let form = Form::new().part("file", file_part);

    let resp = state
        .worker_client
        .post(format!("{}/extract-tables", state.config.worker.url))
        .multipart(form)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Worker table extraction failed: {e}")))?;

    if !resp.status().is_success() {
        return Ok(Json(json!({"tables": [], "count": 0})));
    }

    let tables_res: Value = resp.json().await.unwrap_or(json!({"tables": [], "count": 0}));
    Ok(Json(tables_res))
}

/// GET /api/ai/models/status/
pub async fn get_model_status(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let url = format!("{}/ai/models/status", state.config.worker.url);
    let resp = state
        .worker_client
        .get(&url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Worker unreachable: {e}")))?;
    let val: Value = resp.json().await.unwrap_or(json!({"status": "unknown"}));
    Ok(Json(val))
}

/// POST /api/ai/models/download/
pub async fn download_model(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let url = format!("{}/ai/models/download", state.config.worker.url);
    let resp = state
        .worker_client
        .post(&url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Worker unreachable: {e}")))?;
    let val: Value = resp.json().await.unwrap_or(json!({"status": "started"}));
    Ok(Json(val))
}

/// POST /api/ai/models/cancel/
pub async fn cancel_model_download(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let url = format!("{}/ai/models/cancel", state.config.worker.url);
    let resp = state
        .worker_client
        .post(&url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Worker unreachable: {e}")))?;
    let val: Value = resp.json().await.unwrap_or(json!({"status": "cancelling"}));
    Ok(Json(val))
}

/// DELETE /api/ai/models/
pub async fn delete_model(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let url = format!("{}/ai/models", state.config.worker.url);
    let resp = state
        .worker_client
        .delete(&url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Worker unreachable: {e}")))?;
    let val: Value = resp.json().await.unwrap_or(json!({"status": "deleted"}));
    Ok(Json(val))
}

