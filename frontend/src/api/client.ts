import type { DocumentItem, DocumentListResponse, TaxonomyItem, SystemStats } from '../types'

export type { DocumentItem, DocumentListResponse, TaxonomyItem, SystemStats }

const API_BASE = '/api'

export async function apiFetch<T>(endpoint: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${endpoint}`, {
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    ...options,
  })
  if (!res.ok) {
    const errorBody = await res.json().catch(() => ({ detail: res.statusText }))
    throw new Error(errorBody.detail || 'API request failed')
  }
  if (res.status === 204) return {} as T
  return res.json()
}

export const api = {
  // Documents
  getDocuments: (params?: Record<string, any>) => {
    const search = new URLSearchParams()
    if (params) {
      Object.entries(params).forEach(([k, v]) => {
        if (v !== undefined && v !== null && v !== '') search.append(k, String(v))
      })
    }
    const queryStr = search.toString() ? `?${search.toString()}` : ''
    return apiFetch<DocumentListResponse>(`/documents/${queryStr}`)
  },
  getDocument: (id: number) => apiFetch<DocumentItem>(`/documents/${id}/`),
  updateDocument: (id: number, data: any) =>
    apiFetch<DocumentItem>(`/documents/${id}/`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    }),
  deleteDocument: (id: number) =>
    apiFetch<void>(`/documents/${id}/`, { method: 'DELETE' }),
  postDocument: async (formData: FormData) => {
    const res = await fetch(`${API_BASE}/documents/post_document/`, {
      method: 'POST',
      body: formData,
    })
    if (!res.ok) throw new Error('Upload failed')
    return res.json()
  },
  bulkEdit: (data: { documents: number[]; method: string; parameters?: any }) =>
    apiFetch<{ result: string; affected: number }>(`/documents/bulk_edit/`, {
      method: 'POST',
      body: JSON.stringify(data),
    }),
  addNote: (docId: number, note: string) =>
    apiFetch<any>(`/documents/${docId}/notes/`, {
      method: 'POST',
      body: JSON.stringify({ note }),
    }),
  deleteNote: (docId: number, noteId: number) =>
    apiFetch<void>(`/documents/${docId}/notes/${noteId}/`, { method: 'DELETE' }),

  getDocumentHistory: (id: number) =>
    apiFetch<any[]>(`/documents/${id}/history/`),
  updateVersionLabel: (id: number, version_label: string) =>
    apiFetch<any>(`/documents/${id}/update_version_label/`, {
      method: 'PATCH',
      body: JSON.stringify({ version_label }),
    }),
  uploadDocumentVersion: async (id: number, formData: FormData) => {
    const res = await fetch(`${API_BASE}/documents/${id}/update_version/`, {
      method: 'POST',
      body: formData,
    })
    if (!res.ok) throw new Error('Version upload failed')
    return res.json()
  },

  // Taxonomy
  getTags: () => apiFetch<{ count: number; results: TaxonomyItem[] }>('/tags/'),
  createTag: (data: { name: string; color?: string }) =>
    apiFetch<TaxonomyItem>('/tags/', { method: 'POST', body: JSON.stringify(data) }),
  deleteTag: (id: number) => apiFetch<void>(`/tags/${id}/`, { method: 'DELETE' }),

  getCorrespondents: () =>
    apiFetch<{ count: number; results: TaxonomyItem[] }>('/correspondents/'),
  createCorrespondent: (data: { name: string }) =>
    apiFetch<TaxonomyItem>('/correspondents/', { method: 'POST', body: JSON.stringify(data) }),
  deleteCorrespondent: (id: number) =>
    apiFetch<void>(`/correspondents/${id}/`, { method: 'DELETE' }),

  getDocumentTypes: () =>
    apiFetch<{ count: number; results: TaxonomyItem[] }>('/document_types/'),
  createDocumentType: (data: { name: string }) =>
    apiFetch<TaxonomyItem>('/document_types/', { method: 'POST', body: JSON.stringify(data) }),
  deleteDocumentType: (id: number) =>
    apiFetch<void>(`/document_types/${id}/`, { method: 'DELETE' }),

  getStoragePaths: () =>
    apiFetch<{ count: number; results: TaxonomyItem[] }>('/storage_paths/'),
  createStoragePath: (data: { name: string; path: string; match?: string; matching_algorithm?: number }) =>
    apiFetch<TaxonomyItem>('/storage_paths/', { method: 'POST', body: JSON.stringify(data) }),
  deleteStoragePath: (id: number) =>
    apiFetch<void>(`/storage_paths/${id}/`, { method: 'DELETE' }),

  getCustomFields: () =>
    apiFetch<{ count: number; results: any[] }>('/custom_fields/'),
  createCustomField: (data: { name: string; data_type: string }) =>
    apiFetch<any>('/custom_fields/', { method: 'POST', body: JSON.stringify(data) }),
  deleteCustomField: (id: number) =>
    apiFetch<void>(`/custom_fields/${id}/`, { method: 'DELETE' }),

  getSavedViews: () =>
    apiFetch<{ count: number; results: any[] }>('/saved_views/'),
  createSavedView: (data: any) =>
    apiFetch<any>('/saved_views/', { method: 'POST', body: JSON.stringify(data) }),
  updateSavedView: (id: number, data: any) =>
    apiFetch<any>(`/saved_views/${id}/`, { method: 'PATCH', body: JSON.stringify(data) }),
  deleteSavedView: (id: number) =>
    apiFetch<void>(`/saved_views/${id}/`, { method: 'DELETE' }),

  // Trash
  getTrash: () =>
    apiFetch<{ count: number; results: any[]; all: number[] }>('/trash/'),
  restoreTrash: (documents: number[]) =>
    apiFetch<any>('/trash/', {
      method: 'POST',
      body: JSON.stringify({ action: 'restore', documents }),
    }),
  emptyTrash: (documents?: number[]) =>
    apiFetch<any>('/trash/', {
      method: 'POST',
      body: JSON.stringify({ action: 'empty', documents }),
    }),

  // System & Tasks
  getTasks: (status?: string) =>
    apiFetch<any[]>(`/tasks/${status ? `?status=${status}` : ''}`),
  acknowledgeTasks: (tasks: number[]) =>
    apiFetch<any>('/tasks/acknowledge/', {
      method: 'POST',
      body: JSON.stringify({ tasks }),
    }),
  getLogs: () => apiFetch<any[]>('/logs/'),
  getSanityCheck: () =>
    apiFetch<{
      passed: boolean
      total_documents: number
      checked_files: number
      messages: Array<{
        doc_id: number | null
        type: 'error' | 'warning' | 'info'
        message: string
      }>
    }>('/sanity_check/'),
  getStats: () => apiFetch<SystemStats>('/statistics/'),
  getStatus: () => apiFetch<any>('/status/'),
  getConfig: () => apiFetch<any>('/config/'),
  updateConfig: (data: any) =>
    apiFetch<any>('/config/', { method: 'PATCH', body: JSON.stringify(data) }),
  getUiSettings: () => apiFetch<any>('/ui_settings/'),
  updateUiSettings: (settings: any) =>
    apiFetch<any>('/ui_settings/', { method: 'PATCH', body: JSON.stringify(settings) }),
  getAutocomplete: (term: string) =>
    apiFetch<string[]>(`/search/autocomplete/?term=${encodeURIComponent(term)}`),
  globalSearch: (query: string) =>
    apiFetch<any>(`/search/?query=${encodeURIComponent(query)}`),

  // AI & Document Intelligence
  getDocumentTables: (id: number) =>
    apiFetch<{ tables: Array<{ page: number; index: number; rows: string[][] }>; count: number }>(
      `/documents/${id}/tables/`
    ),

  chatDocumentStream: async (
    id: number,
    message: string,
    history: Array<{ role: string; content: string }>,
    onDelta: (chunk: string) => void
  ): Promise<string> => {
    const res = await fetch(`${API_BASE}/documents/${id}/chat/`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ message, history, stream: true }),
    })
    if (!res.ok) throw new Error('Chat request failed')
    const reader = res.body?.getReader()
    if (!reader) throw new Error('No readable stream')
    const decoder = new TextDecoder()
    let fullText = ''
    let buffer = ''
    while (true) {
      const { done, value } = await reader.read()
      if (done) break
      buffer += decoder.decode(value, { stream: true })
      const lines = buffer.split('\n')
      buffer = lines.pop() || ''
      for (const line of lines) {
        const trimmed = line.trim()
        if (trimmed.startsWith('data: ')) {
          const dataStr = trimmed.slice(6)
          if (dataStr === '[DONE]') break
          try {
            const parsed = JSON.parse(dataStr)
            if (parsed.delta) {
              fullText += parsed.delta
              onDelta(parsed.delta)
            }
          } catch {
            // ignore non-json
          }
        }
      }
    }
    return fullText
  },

  chatGlobalStream: async (
    message: string,
    history: Array<{ role: string; content: string }>,
    onDelta: (chunk: string) => void
  ): Promise<string> => {
    const res = await fetch(`${API_BASE}/chat/`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ message, history, stream: true }),
    })
    if (!res.ok) throw new Error('Global chat request failed')
    const reader = res.body?.getReader()
    if (!reader) throw new Error('No readable stream')
    const decoder = new TextDecoder()
    let fullText = ''
    let buffer = ''
    while (true) {
      const { done, value } = await reader.read()
      if (done) break
      buffer += decoder.decode(value, { stream: true })
      const lines = buffer.split('\n')
      buffer = lines.pop() || ''
      for (const line of lines) {
        const trimmed = line.trim()
        if (trimmed.startsWith('data: ')) {
          const dataStr = trimmed.slice(6)
          if (dataStr === '[DONE]') break
          try {
            const parsed = JSON.parse(dataStr)
            if (parsed.delta) {
              fullText += parsed.delta
              onDelta(parsed.delta)
            }
          } catch {
            // ignore
          }
        }
      }
    }
    return fullText
  },

  // In-App Local AI Models
  getModelStatus: () => apiFetch<ModelStatus>('/ai/models/status/'),
  downloadModel: () => apiFetch<{ status: string }>('/ai/models/download/', { method: 'POST' }),
  cancelModelDownload: () => apiFetch<{ status: string }>('/ai/models/cancel/', { method: 'POST' }),
  deleteModel: () => apiFetch<{ status: string }>('/ai/models/', { method: 'DELETE' }),

  // Next ASN
  getNextAsn: () => apiFetch<{ next_asn: number }>('/documents/next_asn/'),

  // Workflows
  getWorkflows: () => apiFetch<{ count: number; results: any[] }>('/workflows/'),
  createWorkflow: (data: any) =>
    apiFetch<any>('/workflows/', { method: 'POST', body: JSON.stringify(data) }),
  updateWorkflow: (id: number, data: any) =>
    apiFetch<any>(`/workflows/${id}/`, { method: 'PATCH', body: JSON.stringify(data) }),
  deleteWorkflow: (id: number) =>
    apiFetch<void>(`/workflows/${id}/`, { method: 'DELETE' }),

  getWorkflowTriggers: () =>
    apiFetch<{ count: number; results: any[] }>('/workflow_triggers/'),
  createWorkflowTrigger: (data: any) =>
    apiFetch<any>('/workflow_triggers/', { method: 'POST', body: JSON.stringify(data) }),
  deleteWorkflowTrigger: (id: number) =>
    apiFetch<void>(`/workflow_triggers/${id}/`, { method: 'DELETE' }),

  getWorkflowActions: () =>
    apiFetch<{ count: number; results: any[] }>('/workflow_actions/'),
  createWorkflowAction: (data: any) =>
    apiFetch<any>('/workflow_actions/', { method: 'POST', body: JSON.stringify(data) }),
  deleteWorkflowAction: (id: number) =>
    apiFetch<void>(`/workflow_actions/${id}/`, { method: 'DELETE' }),

  // Mail
  getMailAccounts: () =>
    apiFetch<{ count: number; results: any[] }>('/mail_accounts/'),
  createMailAccount: (data: any) =>
    apiFetch<any>('/mail_accounts/', { method: 'POST', body: JSON.stringify(data) }),
  updateMailAccount: (id: number, data: any) =>
    apiFetch<any>(`/mail_accounts/${id}/`, { method: 'PATCH', body: JSON.stringify(data) }),
  deleteMailAccount: (id: number) =>
    apiFetch<void>(`/mail_accounts/${id}/`, { method: 'DELETE' }),
  testMailAccount: (data: any) =>
    apiFetch<any>('/mail_accounts/test/', { method: 'POST', body: JSON.stringify(data) }),

  getMailRules: () =>
    apiFetch<{ count: number; results: any[] }>('/mail_rules/'),
  createMailRule: (data: any) =>
    apiFetch<any>('/mail_rules/', { method: 'POST', body: JSON.stringify(data) }),
  updateMailRule: (id: number, data: any) =>
    apiFetch<any>(`/mail_rules/${id}/`, { method: 'PATCH', body: JSON.stringify(data) }),
  deleteMailRule: (id: number) =>
    apiFetch<void>(`/mail_rules/${id}/`, { method: 'DELETE' }),
  processMailRule: (id: number) =>
    apiFetch<any>(`/mail_rules/${id}/process/`, { method: 'POST' }),
  getProcessedMail: () =>
    apiFetch<{ count: number; results: any[] }>('/processed_mail/'),

  // PDF Studio
  getPdfDeps: () => apiFetch<any>('/pdf/deps'),
  uploadPdfFiles: async (files: File[]) => {
    const formData = new FormData()
    for (const f of files) {
      formData.append('files', f)
    }
    const res = await fetch('/api/pdf/upload', {
      method: 'POST',
      body: formData,
    })
    if (!res.ok) throw new Error(await res.text())
    return res.json()
  },
  loadVaultDocumentToPdfStudio: (docId: number) =>
    apiFetch<any>(`/documents/${docId}/send-to-pdf-studio/`, { method: 'POST' }),
  getPdfThumbnailUrl: (fid: string, page = 0, w = 200, password = '') =>
    `/api/pdf/thumb/${encodeURIComponent(fid)}?page=${page}&w=${w}&password=${encodeURIComponent(password)}`,
  getPdfPageInfo: (fid: string, page = 0) =>
    apiFetch<any>(`/pdf/pageinfo/${encodeURIComponent(fid)}?page=${page}`),
  getPdfMetadata: (fid: string) =>
    apiFetch<any>(`/pdf/metadata/${encodeURIComponent(fid)}`),
  previewPdfTool: async (tool: string, payload: { files: string[]; options: any }, asset?: File) => {
    const formData = new FormData()
    formData.append('payload', JSON.stringify(payload))
    if (asset) {
      formData.append('asset', asset)
    }
    const res = await fetch(`/api/pdf/preview/${tool}`, {
      method: 'POST',
      body: formData,
    })
    if (!res.ok) {
      const errData = await res.json().catch(() => ({ error: 'Preview failed' }))
      throw new Error(errData.error || 'Preview failed')
    }
    return res.json()
  },
  runPdfTool: async (tool: string, payload: { files: string[]; options: any }, asset?: File) => {
    const formData = new FormData()
    formData.append('payload', JSON.stringify(payload))
    if (asset) {
      formData.append('asset', asset)
    }
    const res = await fetch(`/api/pdf/run/${tool}`, {
      method: 'POST',
      body: formData,
    })
    if (!res.ok) {
      const errData = await res.json().catch(() => ({ error: 'Execution failed' }))
      throw new Error(errData.error || 'Execution failed')
    }
    return res.json()
  },
  getPdfDownloadUrl: (fid: string) => `/api/pdf/download/${encodeURIComponent(fid)}`,
  savePdfAsNewDocument: (data: {
    output_id: string
    title?: string
    correspondent?: number
    document_type?: number
    storage_path?: number
    tags?: number[]
  }) => apiFetch<any>('/pdf/save-as-new-document/', { method: 'POST', body: JSON.stringify(data) }),
  savePdfAsVersion: (docId: number, data: { output_id: string; version_label?: string }) =>
    apiFetch<any>(`/documents/${docId}/save-pdf-version/`, { method: 'POST', body: JSON.stringify(data) }),
}

export interface ModelStatus {
  model_id: string
  name: string
  filename: string
  status: 'not_downloaded' | 'downloading' | 'ready' | 'error'
  is_ready: boolean
  is_loaded: boolean
  size_bytes: number
  size_formatted: string
  disk_size_bytes: number
  downloaded_bytes: number
  percent: number
  speed_mbps: number
  error: string | null
  description: string
  models_dir: string
}
