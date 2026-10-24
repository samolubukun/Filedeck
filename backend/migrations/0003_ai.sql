-- AI: per-document text chunk embeddings (little-endian f32 BLOBs, cosine similarity in Rust).
CREATE TABLE IF NOT EXISTS ai_chunks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    text TEXT NOT NULL,
    backend TEXT NOT NULL,
    model TEXT NOT NULL DEFAULT '',
    dim INTEGER NOT NULL,
    vector BLOB NOT NULL,
    UNIQUE(document_id, chunk_index)
);
CREATE INDEX IF NOT EXISTS idx_ai_chunks_document ON ai_chunks(document_id);

CREATE TABLE IF NOT EXISTS ai_index_state (
    document_id INTEGER PRIMARY KEY REFERENCES documents(id) ON DELETE CASCADE,
    content_hash TEXT NOT NULL,
    backend TEXT NOT NULL,
    chunk_count INTEGER NOT NULL DEFAULT 0,
    indexed_at TEXT NOT NULL
);
