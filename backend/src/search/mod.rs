pub mod filters;

use crate::error::AppError;
use crate::state::AppState;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::{MoreLikeThisQuery, Query, QueryParser, TermQuery};
use tantivy::schema::*;
use tantivy::snippet::SnippetGenerator;
use tantivy::{
    DateTime, DocAddress, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term,
};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct SearchFields {
    pub id: Field,
    pub title: Field,
    pub content: Field,
    pub correspondent: Field,
    pub document_type: Field,
    pub storage_path: Field,
    pub tags: Field,
    pub notes: Field,
    pub custom_fields: Field,
    pub created: Field,
    pub added: Field,
    pub modified: Field,
    pub asn: Field,
    pub page_count: Field,
    pub correspondent_id: Field,
    pub document_type_id: Field,
    pub storage_path_id: Field,
    pub tag_id: Field,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SearchMode {
    /// Free text over title, content, correspondent, type, tags, notes.
    Text,
    /// Title only.
    Title,
    /// Advanced Tantivy query syntax (field:value, AND/OR, ranges ...).
    Query,
}

pub struct IndexBundle {
    pub index: Arc<Index>,
    pub writer: Arc<Mutex<IndexWriter>>,
    pub reader: IndexReader,
    pub schema: Schema,
    pub fields: SearchFields,
    /// True if the on-disk index was (re)created and must be repopulated.
    pub needs_rebuild: bool,
}

fn build_schema() -> (Schema, SearchFields) {
    let mut b = Schema::builder();
    let id = b.add_u64_field("id", INDEXED | STORED | FAST);
    let title = b.add_text_field("title", TEXT | STORED);
    let content = b.add_text_field("content", TEXT | STORED);
    let correspondent = b.add_text_field("correspondent", TEXT | STORED);
    let document_type = b.add_text_field("type", TEXT | STORED);
    let storage_path = b.add_text_field("path", TEXT | STORED);
    let tags = b.add_text_field("tag", TEXT | STORED);
    let notes = b.add_text_field("note", TEXT | STORED);
    let custom_fields = b.add_text_field("custom_fields", TEXT | STORED);
    let created = b.add_date_field("created", INDEXED | FAST | STORED);
    let added = b.add_date_field("added", INDEXED | FAST | STORED);
    let modified = b.add_date_field("modified", INDEXED | FAST | STORED);
    let asn = b.add_u64_field("asn", INDEXED | FAST | STORED);
    let page_count = b.add_u64_field("page_count", INDEXED | FAST | STORED);
    let correspondent_id = b.add_u64_field("correspondent_id", INDEXED);
    let document_type_id = b.add_u64_field("document_type_id", INDEXED);
    let storage_path_id = b.add_u64_field("storage_path_id", INDEXED);
    let tag_id = b.add_u64_field("tag_id", INDEXED);
    let schema = b.build();
    (
        schema,
        SearchFields {
            id,
            title,
            content,
            correspondent,
            document_type,
            storage_path,
            tags,
            notes,
            custom_fields,
            created,
            added,
            modified,
            asn,
            page_count,
            correspondent_id,
            document_type_id,
            storage_path_id,
            tag_id,
        },
    )
}

pub fn init_search_index(index_path: &Path) -> anyhow::Result<IndexBundle> {
    std::fs::create_dir_all(index_path)?;
    let (schema, fields) = build_schema();
    let wanted = serde_json::to_string(&schema)?;

    let meta_json = index_path.join("meta.json");
    let mut needs_rebuild = false;
    let index = if meta_json.exists() {
        match Index::open_in_dir(index_path) {
            Ok(existing) if serde_json::to_string(&existing.schema())? == wanted => existing,
            _ => {
                tracing::info!("Search index schema changed, rebuilding index");
                std::fs::remove_dir_all(index_path)?;
                std::fs::create_dir_all(index_path)?;
                needs_rebuild = true;
                Index::create_in_dir(index_path, schema.clone())?
            }
        }
    } else {
        needs_rebuild = true;
        Index::create_in_dir(index_path, schema.clone())?
    };

    let writer = index.writer(50_000_000)?;
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::Manual)
        .try_into()?;

    Ok(IndexBundle {
        index: Arc::new(index),
        writer: Arc::new(Mutex::new(writer)),
        reader,
        schema,
        fields,
        needs_rebuild,
    })
}

// ---------------------------------------------------------------------------
// Indexing
// ---------------------------------------------------------------------------

fn parse_ts(s: &str) -> i64 {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return dt.timestamp();
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(&s[..s.len().min(10)], "%Y-%m-%d") {
        return d
            .and_hms_opt(0, 0, 0)
            .map(|n| n.and_utc().timestamp())
            .unwrap_or(0);
    }
    0
}

async fn build_doc(
    db: &SqlitePool,
    f: &SearchFields,
    id: i64,
) -> Result<Option<TantivyDocument>, AppError> {
    let row = sqlx::query(
        r#"SELECT d.title, d.content, d.created, d.added, d.modified, d.archive_serial_number,
                  d.page_count, d.correspondent_id, d.document_type_id, d.storage_path_id,
                  c.name AS correspondent, t.name AS doc_type, s.name AS storage_path
           FROM documents d
           LEFT JOIN correspondents c ON c.id = d.correspondent_id
           LEFT JOIN document_types t ON t.id = d.document_type_id
           LEFT JOIN storage_paths s ON s.id = d.storage_path_id
           WHERE d.id = $1 AND d.deleted_at IS NULL"#,
    )
    .bind(id)
    .fetch_optional(db)
    .await?;
    let Some(row) = row else { return Ok(None) };

    let mut doc = TantivyDocument::default();
    doc.add_u64(f.id, id as u64);
    doc.add_text(f.title, row.get::<String, _>("title"));
    doc.add_text(f.content, row.get::<String, _>("content"));
    if let Some(v) = row.get::<Option<String>, _>("correspondent") {
        doc.add_text(f.correspondent, v);
    }
    if let Some(v) = row.get::<Option<String>, _>("doc_type") {
        doc.add_text(f.document_type, v);
    }
    if let Some(v) = row.get::<Option<String>, _>("storage_path") {
        doc.add_text(f.storage_path, v);
    }
    doc.add_date(
        f.created,
        DateTime::from_timestamp_secs(parse_ts(&row.get::<String, _>("created"))),
    );
    doc.add_date(
        f.added,
        DateTime::from_timestamp_secs(parse_ts(&row.get::<String, _>("added"))),
    );
    doc.add_date(
        f.modified,
        DateTime::from_timestamp_secs(parse_ts(&row.get::<String, _>("modified"))),
    );
    if let Some(v) = row.get::<Option<i64>, _>("archive_serial_number") {
        doc.add_u64(f.asn, v as u64);
    }
    if let Some(v) = row.get::<Option<i64>, _>("page_count") {
        doc.add_u64(f.page_count, v as u64);
    }
    if let Some(v) = row.get::<Option<i64>, _>("correspondent_id") {
        doc.add_u64(f.correspondent_id, v as u64);
    }
    if let Some(v) = row.get::<Option<i64>, _>("document_type_id") {
        doc.add_u64(f.document_type_id, v as u64);
    }
    if let Some(v) = row.get::<Option<i64>, _>("storage_path_id") {
        doc.add_u64(f.storage_path_id, v as u64);
    }

    let tag_rows = sqlx::query(
        "SELECT t.id, t.name FROM document_tags dt JOIN tags t ON t.id = dt.tag_id WHERE dt.document_id = $1",
    )
    .bind(id)
    .fetch_all(db)
    .await?;
    for t in tag_rows {
        doc.add_u64(f.tag_id, t.get::<i64, _>("id") as u64);
        doc.add_text(f.tags, t.get::<String, _>("name"));
    }

    let notes = sqlx::query("SELECT note FROM notes WHERE document_id = $1 AND deleted_at IS NULL")
        .bind(id)
        .fetch_all(db)
        .await?;
    for n in notes {
        doc.add_text(f.notes, n.get::<String, _>("note"));
    }

    let cfs = sqlx::query(
        r#"SELECT cf.name, i.value_text, i.value_url, i.value_long_text, i.value_select,
                  i.value_int, i.value_float, i.value_date, i.value_monetary
           FROM custom_field_instances i JOIN custom_fields cf ON cf.id = i.field_id
           WHERE i.document_id = $1 AND i.deleted_at IS NULL"#,
    )
    .bind(id)
    .fetch_all(db)
    .await?;
    for c in cfs {
        let mut parts = vec![c.get::<String, _>("name")];
        for col in [
            "value_text",
            "value_url",
            "value_long_text",
            "value_select",
            "value_date",
            "value_monetary",
        ] {
            if let Some(v) = c.get::<Option<String>, _>(col) {
                parts.push(v);
            }
        }
        if let Some(v) = c.get::<Option<i64>, _>("value_int") {
            parts.push(v.to_string());
        }
        if let Some(v) = c.get::<Option<f64>, _>("value_float") {
            parts.push(v.to_string());
        }
        doc.add_text(f.custom_fields, parts.join(" "));
    }

    Ok(Some(doc))
}

/// (Re)index the given documents. Documents that no longer exist or are trashed are removed.
pub async fn reindex_documents(state: &AppState, ids: &[i64]) -> Result<(), AppError> {
    if ids.is_empty() {
        return Ok(());
    }
    let f = &state.search_fields;
    // Take the writer lock *before* reading the database so concurrent reindexes are serialized
    // and a slower, older snapshot can never overwrite a newer one.
    let mut writer = state.index_writer.lock().await;
    for id in ids {
        let doc = build_doc(&state.db, f, *id).await?;
        writer.delete_term(Term::from_field_u64(f.id, *id as u64));
        if let Some(d) = doc {
            writer.add_document(d)?;
        }
    }
    writer.commit()?;
    drop(writer);
    state.index_reader.reload()?;
    Ok(())
}

pub async fn reindex_document(state: &AppState, id: i64) -> Result<(), AppError> {
    reindex_documents(state, &[id]).await
}

/// Drop everything and rebuild from the database.
pub async fn reindex_all(state: &AppState) -> Result<usize, AppError> {
    let f = &state.search_fields;
    let mut writer = state.index_writer.lock().await;
    let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM documents WHERE deleted_at IS NULL")
        .fetch_all(&state.db)
        .await?;
    let mut docs = Vec::with_capacity(ids.len());
    for id in &ids {
        if let Some(d) = build_doc(&state.db, f, *id).await? {
            docs.push(d);
        }
    }
    let n = docs.len();
    writer.delete_all_documents()?;
    for d in docs {
        writer.add_document(d)?;
    }
    writer.commit()?;
    drop(writer);
    state.index_reader.reload()?;
    Ok(n)
}

/// Fire-and-forget full rebuild; used after taxonomy renames/deletes which affect many documents.
///
/// Requests are coalesced: while a rebuild runs, any number of further requests collapse into a
/// single follow-up rebuild, so a burst of edits costs at most two index passes.
pub fn reindex_all_background(state: &AppState) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static REQUESTED: AtomicBool = AtomicBool::new(false);
    static RUNNING: AtomicBool = AtomicBool::new(false);

    REQUESTED.store(true, Ordering::SeqCst);
    if RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    let state = state.clone();
    tokio::spawn(async move {
        loop {
            while REQUESTED.swap(false, Ordering::SeqCst) {
                if let Err(e) = reindex_all(&state).await {
                    tracing::warn!("background reindex failed: {e}");
                }
            }
            RUNNING.store(false, Ordering::SeqCst);
            // A request may have arrived between the last check and clearing RUNNING.
            if !REQUESTED.load(Ordering::SeqCst) || RUNNING.swap(true, Ordering::SeqCst) {
                break;
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Searching
// ---------------------------------------------------------------------------

pub fn build_query(state: &AppState, raw: &str, mode: SearchMode) -> Box<dyn Query> {
    let f = &state.search_fields;
    let default_fields = match mode {
        SearchMode::Title => vec![f.title],
        _ => vec![
            f.title,
            f.content,
            f.correspondent,
            f.document_type,
            f.storage_path,
            f.tags,
            f.notes,
            f.custom_fields,
        ],
    };
    let mut parser = QueryParser::for_index(&state.index, default_fields);
    parser.set_conjunction_by_default();
    parser.set_field_boost(f.title, 2.0);
    let (q, _errors) = parser.parse_query_lenient(raw);
    q
}

pub struct Hit {
    pub id: i64,
    pub score: f32,
}

pub const MAX_HITS: usize = 10_000;

/// Ranked ids (best first) for a query.
pub fn search_ids(
    state: &AppState,
    raw: &str,
    mode: SearchMode,
    limit: usize,
) -> Result<Vec<Hit>, AppError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(vec![]);
    }
    let searcher = state.index_reader.searcher();
    let query = build_query(state, raw, mode);
    let top = searcher.search(&*query, &TopDocs::with_limit(limit.max(1)))?;
    let mut hits = Vec::with_capacity(top.len());
    for (score, addr) in top {
        let doc: TantivyDocument = searcher.doc(addr)?;
        if let Some(id) = doc
            .get_first(state.search_fields.id)
            .and_then(|v| v.as_u64())
        {
            hits.push(Hit {
                id: id as i64,
                score,
            });
        }
    }
    Ok(hits)
}

fn doc_address(state: &AppState, id: i64) -> Result<Option<DocAddress>, AppError> {
    let searcher = state.index_reader.searcher();
    let term = Term::from_field_u64(state.search_fields.id, id as u64);
    let q = TermQuery::new(term, IndexRecordOption::Basic);
    let top = searcher.search(&q, &TopDocs::with_limit(1))?;
    Ok(top.into_iter().next().map(|(_, a)| a))
}

/// Documents similar to `id` (excluding itself), best first.
pub fn more_like_this_ids(state: &AppState, id: i64, limit: usize) -> Result<Vec<Hit>, AppError> {
    let Some(addr) = doc_address(state, id)? else {
        return Ok(vec![]);
    };
    let searcher = state.index_reader.searcher();
    let q = MoreLikeThisQuery::builder()
        .with_min_doc_frequency(1)
        .with_min_term_frequency(1)
        .with_min_word_length(3)
        .with_max_query_terms(25)
        .with_document(addr);
    let top = searcher.search(&q, &TopDocs::with_limit(limit.max(1) + 1))?;
    let mut hits = Vec::new();
    for (score, a) in top {
        let doc: TantivyDocument = searcher.doc(a)?;
        if let Some(hid) = doc
            .get_first(state.search_fields.id)
            .and_then(|v| v.as_u64())
        {
            if hid as i64 != id {
                hits.push(Hit {
                    id: hid as i64,
                    score,
                });
            }
        }
    }
    Ok(hits)
}

pub struct Highlights {
    pub highlights: String,
    pub note_highlights: String,
}

fn snippet_html(
    searcher: &tantivy::Searcher,
    query: &dyn Query,
    field: Field,
    doc: &TantivyDocument,
) -> String {
    match SnippetGenerator::create(searcher, query, field) {
        Ok(mut g) => {
            g.set_max_num_chars(150);
            let s = g.snippet_from_doc(doc);
            if s.is_empty() {
                String::new()
            } else {
                s.to_html()
                    .replace("<b>", "<span class=\"match\">")
                    .replace("</b>", "</span>")
            }
        }
        Err(_) => String::new(),
    }
}

/// Highlight snippets for a set of already-ranked ids (only call with one page worth).
pub fn highlights_for(
    state: &AppState,
    raw: &str,
    mode: SearchMode,
    ids: &[i64],
) -> HashMap<i64, Highlights> {
    let mut out = HashMap::new();
    let searcher = state.index_reader.searcher();
    let query = build_query(state, raw.trim(), mode);
    for id in ids {
        let Ok(Some(addr)) = doc_address(state, *id) else {
            continue;
        };
        let Ok(doc) = searcher.doc::<TantivyDocument>(addr) else {
            continue;
        };
        let mut h = snippet_html(&searcher, &*query, state.search_fields.content, &doc);
        if h.is_empty() {
            h = snippet_html(&searcher, &*query, state.search_fields.title, &doc);
        }
        let n = snippet_html(&searcher, &*query, state.search_fields.notes, &doc);
        out.insert(
            *id,
            Highlights {
                highlights: h,
                note_highlights: n,
            },
        );
    }
    out
}

/// Complete the last word of `term` using the index term dictionary, most frequent first.
pub fn autocomplete(state: &AppState, term: &str, limit: usize) -> Result<Vec<String>, AppError> {
    let last = term.split_whitespace().last().unwrap_or("").to_lowercase();
    if last.is_empty() {
        return Ok(vec![]);
    }
    let searcher = state.index_reader.searcher();
    let f = &state.search_fields;
    let mut freq: HashMap<String, u64> = HashMap::new();
    for field in [f.title, f.content, f.correspondent, f.document_type, f.tags] {
        for seg in searcher.segment_readers() {
            let inv = seg.inverted_index(field)?;
            let dict = inv.terms();
            let mut stream = dict.range().ge(last.as_bytes()).into_stream()?;
            let mut guard = 0;
            while stream.advance() && guard < 2000 {
                guard += 1;
                let key = stream.key();
                if !key.starts_with(last.as_bytes()) {
                    break;
                }
                if let Ok(s) = std::str::from_utf8(key) {
                    *freq.entry(s.to_string()).or_insert(0) += stream.value().doc_freq as u64;
                }
            }
        }
    }
    let mut words: Vec<(String, u64)> = freq.into_iter().collect();
    words.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    Ok(words.into_iter().take(limit).map(|(w, _)| w).collect())
}

/// Ids that exist in the index (used for diagnostics / status).
pub fn indexed_doc_count(state: &AppState) -> u64 {
    state.index_reader.searcher().num_docs()
}
