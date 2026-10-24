-- Phase 6/7: mail + workflows schema additions

-- mail rules
ALTER TABLE mail_rules ADD COLUMN filter_attachment_filename_exclude TEXT;
ALTER TABLE mail_rules ADD COLUMN pdf_layout INTEGER NOT NULL DEFAULT 0;
ALTER TABLE mail_rules ADD COLUMN stop_processing BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE mail_rules ADD COLUMN assign_owner_from_rule BOOLEAN NOT NULL DEFAULT 1;

-- processed mail
ALTER TABLE processed_mail ADD COLUMN folder TEXT NOT NULL DEFAULT '';
ALTER TABLE processed_mail ADD COLUMN uid_validity TEXT;
ALTER TABLE processed_mail ADD COLUMN status TEXT NOT NULL DEFAULT 'SUCCESS';
ALTER TABLE processed_mail ADD COLUMN error TEXT;

-- workflow triggers: filters stored as JSON id arrays
ALTER TABLE workflow_triggers ADD COLUMN filter_has_tags TEXT;
ALTER TABLE workflow_triggers ADD COLUMN filter_has_all_tags TEXT;
ALTER TABLE workflow_triggers ADD COLUMN filter_has_not_tags TEXT;
ALTER TABLE workflow_triggers ADD COLUMN filter_has_any_correspondents TEXT;
ALTER TABLE workflow_triggers ADD COLUMN filter_has_not_correspondents TEXT;
ALTER TABLE workflow_triggers ADD COLUMN filter_has_any_document_types TEXT;
ALTER TABLE workflow_triggers ADD COLUMN filter_has_not_document_types TEXT;
ALTER TABLE workflow_triggers ADD COLUMN filter_has_any_storage_paths TEXT;
ALTER TABLE workflow_triggers ADD COLUMN filter_has_not_storage_paths TEXT;
ALTER TABLE workflow_triggers ADD COLUMN filter_has_correspondent_id INTEGER;
ALTER TABLE workflow_triggers ADD COLUMN filter_has_document_type_id INTEGER;
ALTER TABLE workflow_triggers ADD COLUMN filter_has_storage_path_id INTEGER;
ALTER TABLE workflow_triggers ADD COLUMN filter_custom_field_query TEXT;

-- workflow actions
ALTER TABLE workflow_actions ADD COLUMN assign_custom_fields_values TEXT;
ALTER TABLE workflow_actions ADD COLUMN remove_correspondents TEXT;
ALTER TABLE workflow_actions ADD COLUMN remove_all_correspondents BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE workflow_actions ADD COLUMN remove_document_types TEXT;
ALTER TABLE workflow_actions ADD COLUMN remove_all_document_types BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE workflow_actions ADD COLUMN remove_storage_paths TEXT;
ALTER TABLE workflow_actions ADD COLUMN remove_all_storage_paths BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE workflow_actions ADD COLUMN email_include_document BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE workflow_actions ADD COLUMN webhook_use_params BOOLEAN NOT NULL DEFAULT 1;
ALTER TABLE workflow_actions ADD COLUMN webhook_as_json BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE workflow_actions ADD COLUMN webhook_params TEXT;
ALTER TABLE workflow_actions ADD COLUMN webhook_body TEXT;
ALTER TABLE workflow_actions ADD COLUMN webhook_headers TEXT;
ALTER TABLE workflow_actions ADD COLUMN passwords TEXT;
ALTER TABLE workflow_actions ADD COLUMN ai_suggestion_fields TEXT;
ALTER TABLE workflow_actions ADD COLUMN ai_create_missing BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE workflow_actions ADD COLUMN ai_overwrite_existing BOOLEAN NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS workflow_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workflow_id INTEGER NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    type INTEGER,
    document_id INTEGER REFERENCES documents(id) ON DELETE CASCADE,
    run_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_workflow_runs_doc ON workflow_runs(document_id, workflow_id, type);
