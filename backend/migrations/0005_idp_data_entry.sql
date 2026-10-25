-- Migration 0005: Intelligent Document Processing & Data Entry Automation

CREATE TABLE IF NOT EXISTS extraction_schemas (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    document_type_id INTEGER REFERENCES document_types(id) ON DELETE SET NULL,
    columns_json TEXT NOT NULL, -- JSON array of column definitions
    validation_rules_json TEXT,  -- JSON array of cross-field checks
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS extraction_batches (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    schema_id INTEGER NOT NULL REFERENCES extraction_schemas(id) ON DELETE RESTRICT,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'queued', -- queued, processing, needs_review, completed, failed
    total_documents INTEGER DEFAULT 0,
    processed_documents INTEGER DEFAULT 0,
    flagged_fields_count INTEGER DEFAULT 0,
    output_filename TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME
);

CREATE TABLE IF NOT EXISTS extraction_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    batch_id INTEGER NOT NULL REFERENCES extraction_batches(id) ON DELETE CASCADE,
    document_id INTEGER REFERENCES documents(id) ON DELETE SET NULL,
    original_filename TEXT NOT NULL,
    file_path TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending', -- pending, extracted, needs_review, approved, error
    confidence_score REAL DEFAULT 0.0,
    error_message TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS extracted_cells (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    record_id INTEGER NOT NULL REFERENCES extraction_records(id) ON DELETE CASCADE,
    column_key TEXT NOT NULL,
    raw_value TEXT,
    normalized_value TEXT,
    data_type TEXT NOT NULL DEFAULT 'string',
    confidence REAL NOT NULL DEFAULT 1.0,
    page_number INTEGER DEFAULT 1,
    bbox_json TEXT, -- [x0, y0, x1, y1] normalized coordinates
    is_reviewed INTEGER NOT NULL DEFAULT 0,
    reviewed_by TEXT,
    validation_error TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_extraction_batches_status ON extraction_batches(status);
CREATE INDEX IF NOT EXISTS idx_extraction_records_batch ON extraction_records(batch_id);
CREATE INDEX IF NOT EXISTS idx_extracted_cells_record ON extracted_cells(record_id);
CREATE INDEX IF NOT EXISTS idx_extracted_cells_column ON extracted_cells(column_key);

-- Insert Default High-Frequency Office Schemas
INSERT OR IGNORE INTO extraction_schemas (id, name, description, columns_json, validation_rules_json)
VALUES (
    1,
    'Vendor Invoices & Bills',
    'Standard procurement & accounts payable invoice data entry',
    '[{"key":"invoice_number","label":"Invoice Number","type":"string","required":true,"hints":["Invoice #","Inv No","Bill No","Invoice Number"]},{"key":"vendor_name","label":"Vendor Name","type":"string","required":true,"hints":["Vendor","Billed From","Company","Supplier"]},{"key":"invoice_date","label":"Invoice Date","type":"date","required":true,"hints":["Date","Invoice Date","Bill Date"]},{"key":"due_date","label":"Due Date","type":"date","required":false,"hints":["Due Date","Payment Due"]},{"key":"subtotal","label":"Subtotal","type":"monetary","required":false,"hints":["Subtotal","Net Amount"]},{"key":"tax_amount","label":"Tax Amount","type":"monetary","required":false,"hints":["Tax","VAT","GST"]},{"key":"total_amount","label":"Total Amount","type":"monetary","required":true,"hints":["Total","Total Amount","Gross Amount","Balance Due"]}]',
    '[{"rule":"sum","components":["subtotal","tax_amount"],"target":"total_amount","description":"Subtotal + Tax must equal Total Amount"}]'
);

INSERT OR IGNORE INTO extraction_schemas (id, name, description, columns_json, validation_rules_json)
VALUES (
    2,
    'Employee & HR Records',
    'Personnel intake forms, registration sheets, and identity records',
    '[{"key":"employee_id","label":"Employee ID","type":"string","required":true,"regex":"^EMP-\\d{4,6}$","hints":["Employee #","Badge ID","Staff ID"]},{"key":"full_name","label":"Full Name","type":"string","required":true,"hints":["Full Name","Employee Name","Staff Name"]},{"key":"hire_date","label":"Hire Date","type":"date","required":true,"hints":["Date of Hire","Joining Date","Start Date"]},{"key":"department","label":"Department","type":"string","required":false,"hints":["Department","Division","Unit"]},{"key":"job_title","label":"Job Title","type":"string","required":false,"hints":["Title","Position","Role"]},{"key":"salary","label":"Base Salary","type":"monetary","required":false,"hints":["Base Salary","Monthly Compensation","Wage"]}]',
    '[]'
);
