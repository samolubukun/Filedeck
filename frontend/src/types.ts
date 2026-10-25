export interface DocumentVersionItem {
  id: number
  added: string
  version_index: number
  version_label?: string | null
  checksum: string
  is_root: boolean
  is_current: boolean
}

export interface DocumentHistoryItem {
  id: number
  timestamp: string
  actor?: string | { id?: number; username?: string; first_name?: string; last_name?: string } | null
  action: string
  details?: Record<string, any>
  changes?: Record<string, any>
}

export interface DocumentItem {
  id: number
  title: string
  content: string
  correspondent?: number | null
  document_type?: number | null
  storage_path?: number | null
  tags: number[]
  checksum: string
  archive_checksum?: string | null
  mime_type?: string
  page_count?: number
  created: string
  modified: string
  added: string
  original_file_name?: string
  archived_file_name?: string
  download_url: string
  thumbnail_url: string
  archive_serial_number?: number | null
  notes: Array<{ id: number; note: string; created: string }>
  custom_fields: Array<{ field: number; value: any }>
  root_document?: number | null
  versions?: DocumentVersionItem[]
}

export interface DocumentListResponse {
  count: number
  results: DocumentItem[]
  all?: number[]
}

export interface TaxonomyItem {
  id: number
  slug?: string
  name: string
  color?: string
  path?: string
  match?: string
  matching_algorithm?: number
  is_insensitive?: boolean
  document_count?: number
}

export interface CustomFieldDefinition {
  id: number
  name: string
  data_type: 'string' | 'url' | 'date' | 'boolean' | 'integer' | 'float' | 'monetary' | 'documentlink' | string
  extra_data?: any
  created?: string
}

export interface TaskItem {
  id: number
  task_id: string
  task_type: string
  trigger_source: string
  status: 'PENDING' | 'STARTED' | 'SUCCESS' | 'FAILURE' | string
  date_created: string
  date_started?: string | null
  date_done?: string | null
  duration_seconds?: number | null
  wait_time_seconds?: number | null
  input_data?: any
  result_data?: any
  related_document?: number | null
  acknowledged: boolean
}

export type NavView =
  | 'dashboard'
  | 'documents'
  | 'data_entry'
  | 'pdf_studio'
  | 'saved_views'
  | 'chat'
  | 'tags'
  | 'correspondents'
  | 'document_types'
  | 'storage_paths'
  | 'custom_fields'
  | 'workflows'
  | 'mail'
  | 'tasks'
  | 'trash'
  | 'logs'
  | 'settings'

export interface ExtractionColumn {
  key: string
  label: string
  type: 'string' | 'number' | 'date' | 'monetary' | 'boolean'
  required?: boolean
  regex?: string
  hints?: string[]
}

export interface ExtractionSchema {
  id: number
  name: string
  description?: string
  document_type_id?: number | null
  columns: ExtractionColumn[]
  validation_rules?: any[]
  created_at?: string
  updated_at?: string
}

export interface ExtractionBatch {
  id: number
  schema_id: number
  schema_name: string
  title: string
  status: 'queued' | 'processing' | 'needs_review' | 'completed' | 'failed' | string
  total_documents: number
  processed_documents: number
  flagged_fields_count: number
  created_at: string
  completed_at?: string | null
}

export interface ExtractionCell {
  id: number
  raw_value?: string | null
  value?: string | null
  data_type: string
  confidence: number
  page_number: number
  bbox?: [number, number, number, number] | null
  is_reviewed: boolean
  validation_error?: string | null
}

export interface ExtractionRow {
  record_id: number
  document_id?: number | null
  filename: string
  status: 'pending' | 'extracted' | 'needs_review' | 'approved' | 'error' | string
  confidence_score: number
  error_message?: string | null
  cells: Record<string, ExtractionCell>
}

export interface BatchGridData {
  batch_id: number
  title: string
  status: string
  total_documents: number
  processed_documents: number
  flagged_fields_count: number
  columns: ExtractionColumn[]
  validation_rules?: any[]
  rows: ExtractionRow[]
}

export type ViewMode = 'table' | 'cards' | 'large_cards'

export interface FileTypeStat {
  mime_type: string
  count: number
}

export interface SystemStats {
  documents_total: number
  documents_inbox: number
  character_count: number
  tags_total: number
  correspondents_total: number
  document_types_total: number
  storage_paths_total: number
  document_file_type_counts?: FileTypeStat[]
}

export interface MailAccount {
  id: number
  name: string
  imap_server: string
  imap_port?: number | null
  imap_security?: number
  username: string
  password?: string
  is_token?: boolean
  character_set?: string
  account_type?: number
  expiration?: string | null
}

export interface MailRule {
  id: number
  account: number
  name: string
  order?: number
  enabled?: boolean
  folder?: string
  filter_from?: string | null
  filter_to?: string | null
  filter_subject?: string | null
  filter_body?: string | null
  filter_attachment_filename?: string | null
  filter_attachment_filename_include?: string | null
  maximum_age?: number
  attachment_type?: number
  action?: number
  action_parameter?: string | null
  assign_title_from?: number
  assign_correspondent_from?: number
  assign_tags?: number[]
  assign_document_type?: number | null
  assign_correspondent?: number | null
  consumption_scope?: number
}

export interface ProcessedMail {
  id?: number
  rule_id: number
  message_id?: string
  uid?: number
  subject?: string
  processed_at?: string
  status?: string
}

