-- Document versions (root_document_id/version_index/version_label live in 0001) and audit log.
CREATE INDEX IF NOT EXISTS idx_documents_root ON documents(root_document_id);

CREATE TABLE IF NOT EXISTS document_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_id INTEGER NOT NULL,
    timestamp TEXT NOT NULL,
    action TEXT NOT NULL DEFAULT 'update',
    changes TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_document_history_doc ON document_history(document_id, timestamp);
