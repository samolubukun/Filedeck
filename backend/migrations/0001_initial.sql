-- FileDeck Initial Schema Migration (Single-user, local-first)

CREATE TABLE IF NOT EXISTS correspondents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    match TEXT NOT NULL DEFAULT '',
    matching_algorithm INTEGER NOT NULL DEFAULT 1,
    is_insensitive BOOLEAN NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL DEFAULT '#a6cee3',
    is_inbox_tag BOOLEAN NOT NULL DEFAULT 0,
    parent_id INTEGER REFERENCES tags(id) ON DELETE SET NULL,
    match TEXT NOT NULL DEFAULT '',
    matching_algorithm INTEGER NOT NULL DEFAULT 1,
    is_insensitive BOOLEAN NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS document_types (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    match TEXT NOT NULL DEFAULT '',
    matching_algorithm INTEGER NOT NULL DEFAULT 1,
    is_insensitive BOOLEAN NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS storage_paths (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    path TEXT NOT NULL DEFAULT '',
    match TEXT NOT NULL DEFAULT '',
    matching_algorithm INTEGER NOT NULL DEFAULT 1,
    is_insensitive BOOLEAN NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL DEFAULT '',
    content_length INTEGER NOT NULL DEFAULT 0,
    mime_type TEXT NOT NULL DEFAULT 'application/pdf',
    checksum TEXT NOT NULL DEFAULT '',
    archive_checksum TEXT,
    page_count INTEGER,
    created TEXT NOT NULL,
    modified TEXT NOT NULL,
    added TEXT NOT NULL,
    filename TEXT,
    archive_filename TEXT,
    original_filename TEXT,
    archive_serial_number INTEGER UNIQUE,
    root_document_id INTEGER REFERENCES documents(id) ON DELETE CASCADE,
    version_index INTEGER,
    version_label TEXT,
    correspondent_id INTEGER REFERENCES correspondents(id) ON DELETE SET NULL,
    document_type_id INTEGER REFERENCES document_types(id) ON DELETE SET NULL,
    storage_path_id INTEGER REFERENCES storage_paths(id) ON DELETE SET NULL,
    deleted_at TEXT,
    transaction_id TEXT
);

CREATE INDEX IF NOT EXISTS idx_documents_created ON documents(created);
CREATE INDEX IF NOT EXISTS idx_documents_added ON documents(added);
CREATE INDEX IF NOT EXISTS idx_documents_checksum ON documents(checksum);
CREATE INDEX IF NOT EXISTS idx_documents_deleted_at ON documents(deleted_at);

CREATE TABLE IF NOT EXISTS document_tags (
    document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (document_id, tag_id)
);

CREATE TABLE IF NOT EXISTS notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    note TEXT NOT NULL DEFAULT '',
    created TEXT NOT NULL,
    deleted_at TEXT
);

CREATE TABLE IF NOT EXISTS document_barcodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    page INTEGER NOT NULL,
    value TEXT NOT NULL,
    format TEXT NOT NULL DEFAULT 'QRCode'
);

CREATE TABLE IF NOT EXISTS custom_fields (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    data_type TEXT NOT NULL,
    extra_data TEXT,
    created TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS custom_field_instances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    field_id INTEGER NOT NULL REFERENCES custom_fields(id) ON DELETE CASCADE,
    created TEXT NOT NULL,
    value_text TEXT,
    value_bool BOOLEAN,
    value_url TEXT,
    value_date TEXT,
    value_int INTEGER,
    value_float REAL,
    value_monetary TEXT,
    value_document_ids TEXT,
    value_select TEXT,
    value_long_text TEXT,
    deleted_at TEXT
);

CREATE TABLE IF NOT EXISTS saved_views (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    icon TEXT NOT NULL DEFAULT 'funnel',
    sort_field TEXT,
    sort_reverse BOOLEAN NOT NULL DEFAULT 0,
    page_size INTEGER,
    display_mode TEXT,
    display_fields TEXT
);

CREATE TABLE IF NOT EXISTS saved_view_filter_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    saved_view_id INTEGER NOT NULL REFERENCES saved_views(id) ON DELETE CASCADE,
    rule_type INTEGER NOT NULL,
    value TEXT
);

CREATE TABLE IF NOT EXISTS share_links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    slug TEXT NOT NULL UNIQUE,
    expiration TEXT,
    file_version TEXT NOT NULL DEFAULT 'archive',
    created TEXT NOT NULL,
    deleted_at TEXT
);

CREATE TABLE IF NOT EXISTS share_link_bundles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    slug TEXT NOT NULL UNIQUE,
    expiration TEXT,
    file_version TEXT NOT NULL DEFAULT 'archive',
    status TEXT NOT NULL DEFAULT 'pending',
    size_bytes INTEGER,
    last_error TEXT,
    file_path TEXT,
    built_at TEXT
);

CREATE TABLE IF NOT EXISTS share_link_bundle_documents (
    bundle_id INTEGER NOT NULL REFERENCES share_link_bundles(id) ON DELETE CASCADE,
    document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    PRIMARY KEY (bundle_id, document_id)
);

CREATE TABLE IF NOT EXISTS workflow_triggers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    type INTEGER NOT NULL,
    sources TEXT NOT NULL DEFAULT '',
    filter_path TEXT,
    filter_filename TEXT,
    filter_mailrule_id INTEGER,
    match TEXT,
    matching_algorithm INTEGER NOT NULL DEFAULT 1,
    is_insensitive BOOLEAN NOT NULL DEFAULT 1,
    schedule_date_field TEXT,
    schedule_offset_days INTEGER,
    schedule_is_recurring BOOLEAN NOT NULL DEFAULT 0,
    schedule_recurring_interval_days INTEGER,
    schedule_custom_field_id INTEGER
);

CREATE TABLE IF NOT EXISTS workflow_trigger_tags (
    trigger_id INTEGER NOT NULL REFERENCES workflow_triggers(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    relation_type INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (trigger_id, tag_id)
);

CREATE TABLE IF NOT EXISTS workflow_trigger_doc_types (
    trigger_id INTEGER NOT NULL REFERENCES workflow_triggers(id) ON DELETE CASCADE,
    doc_type_id INTEGER NOT NULL REFERENCES document_types(id) ON DELETE CASCADE,
    relation_type INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (trigger_id, doc_type_id)
);

CREATE TABLE IF NOT EXISTS workflow_trigger_correspondents (
    trigger_id INTEGER NOT NULL REFERENCES workflow_triggers(id) ON DELETE CASCADE,
    correspondent_id INTEGER NOT NULL REFERENCES correspondents(id) ON DELETE CASCADE,
    relation_type INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (trigger_id, correspondent_id)
);

CREATE TABLE IF NOT EXISTS workflow_actions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    type INTEGER NOT NULL,
    "order" INTEGER NOT NULL DEFAULT 0,
    assign_title TEXT,
    assign_tags TEXT,
    assign_correspondent_id INTEGER,
    assign_document_type_id INTEGER,
    assign_storage_path_id INTEGER,
    assign_asn INTEGER,
    assign_custom_fields TEXT,
    remove_tags TEXT,
    remove_all_tags BOOLEAN NOT NULL DEFAULT 0,
    remove_correspondent BOOLEAN NOT NULL DEFAULT 0,
    remove_document_type BOOLEAN NOT NULL DEFAULT 0,
    remove_storage_path BOOLEAN NOT NULL DEFAULT 0,
    remove_custom_fields TEXT,
    remove_all_custom_fields BOOLEAN NOT NULL DEFAULT 0,
    email_to TEXT,
    email_subject TEXT,
    email_body TEXT,
    webhook_url TEXT,
    webhook_include_document BOOLEAN NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS workflows (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    "order" INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS workflow_workflow_triggers (
    workflow_id INTEGER NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    trigger_id INTEGER NOT NULL REFERENCES workflow_triggers(id) ON DELETE CASCADE,
    PRIMARY KEY (workflow_id, trigger_id)
);

CREATE TABLE IF NOT EXISTS workflow_workflow_actions (
    workflow_id INTEGER NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    action_id INTEGER NOT NULL REFERENCES workflow_actions(id) ON DELETE CASCADE,
    PRIMARY KEY (workflow_id, action_id)
);

CREATE TABLE IF NOT EXISTS mail_accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    imap_server TEXT NOT NULL,
    imap_port INTEGER,
    imap_security INTEGER NOT NULL DEFAULT 2,
    username TEXT NOT NULL,
    password TEXT NOT NULL DEFAULT '',
    is_token BOOLEAN NOT NULL DEFAULT 0,
    character_set TEXT NOT NULL DEFAULT 'UTF-8',
    account_type INTEGER NOT NULL DEFAULT 1,
    refresh_token TEXT,
    expiration TEXT
);

CREATE TABLE IF NOT EXISTS mail_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL REFERENCES mail_accounts(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    "order" INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    folder TEXT NOT NULL DEFAULT 'INBOX',
    filter_from TEXT,
    filter_to TEXT,
    filter_subject TEXT,
    filter_body TEXT,
    filter_attachment_filename TEXT,
    maximum_age INTEGER,
    attachment_type INTEGER NOT NULL DEFAULT 1,
    action INTEGER NOT NULL DEFAULT 3,
    action_parameter TEXT,
    assign_title_from INTEGER NOT NULL DEFAULT 1,
    assign_correspondent_from INTEGER NOT NULL DEFAULT 1,
    assign_tags TEXT,
    assign_document_type_id INTEGER,
    assign_correspondent_id INTEGER,
    consumption_scope INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS processed_mail (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_id INTEGER REFERENCES mail_rules(id) ON DELETE SET NULL,
    uid TEXT NOT NULL,
    subject TEXT,
    received TEXT,
    processed TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL UNIQUE,
    task_type TEXT NOT NULL,
    trigger_source TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    date_created TEXT NOT NULL,
    date_started TEXT,
    date_done TEXT,
    duration_seconds REAL,
    wait_time_seconds REAL,
    input_data TEXT NOT NULL DEFAULT '{}',
    result_data TEXT,
    acknowledged BOOLEAN NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_created ON tasks(date_created);

CREATE TABLE IF NOT EXISTS ui_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    settings TEXT NOT NULL DEFAULT '{}'
);

INSERT OR IGNORE INTO ui_settings (id, settings) VALUES (1, '{}');

CREATE TABLE IF NOT EXISTS app_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    app_title TEXT NOT NULL DEFAULT 'FileDeck',
    app_logo TEXT,
    output_type TEXT NOT NULL DEFAULT 'pdfa',
    language TEXT NOT NULL DEFAULT 'eng',
    mode TEXT NOT NULL DEFAULT 'skip_noarchive'
);

INSERT OR IGNORE INTO app_config (id, app_title, app_logo, output_type, language, mode)
VALUES (1, 'FileDeck', NULL, 'pdfa', 'eng', 'skip_noarchive');
