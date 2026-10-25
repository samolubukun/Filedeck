import type { NavView } from '../types'

const HASH_TO_NAV: Record<string, NavView> = {
  '#/': 'dashboard',
  '#': 'dashboard',
  '': 'dashboard',
  '#/documents': 'documents',
  '#/data-entry': 'data_entry',
  '#/data_entry': 'data_entry',
  '#/idp': 'data_entry',
  '#/pdf-studio': 'pdf_studio',
  '#/pdf_studio': 'pdf_studio',
  '#/views': 'saved_views',
  '#/saved_views': 'saved_views',
  '#/chat': 'chat',
  '#/tags': 'tags',
  '#/correspondents': 'correspondents',
  '#/document_types': 'document_types',
  '#/types': 'document_types',
  '#/storage_paths': 'storage_paths',
  '#/paths': 'storage_paths',
  '#/custom_fields': 'custom_fields',
  '#/fields': 'custom_fields',
  '#/workflows': 'workflows',
  '#/mail': 'mail',
  '#/tasks': 'tasks',
  '#/trash': 'trash',
  '#/logs': 'logs',
  '#/settings': 'settings',
}

const NAV_TO_HASH: Record<NavView, string> = {
  dashboard: '#/',
  documents: '#/documents',
  data_entry: '#/data-entry',
  pdf_studio: '#/pdf-studio',
  saved_views: '#/views',
  chat: '#/chat',
  tags: '#/tags',
  correspondents: '#/correspondents',
  document_types: '#/types',
  storage_paths: '#/paths',
  custom_fields: '#/fields',
  workflows: '#/workflows',
  mail: '#/mail',
  tasks: '#/tasks',
  trash: '#/trash',
  logs: '#/logs',
  settings: '#/settings',
}

export function parseHash(): { nav: NavView; docId: number | null } {
  if (typeof window === 'undefined') {
    return { nav: 'dashboard', docId: null }
  }
  const rawHash = window.location.hash || '#/'
  const [routePart, queryPart] = rawHash.split('?')
  const nav = HASH_TO_NAV[routePart] ?? 'dashboard'
  let docId: number | null = null

  if (queryPart) {
    const params = new URLSearchParams(queryPart)
    const docParam = params.get('doc')
    if (docParam) {
      const parsed = parseInt(docParam, 10)
      if (!isNaN(parsed)) {
        docId = parsed
      }
    }
  }

  return { nav, docId }
}

export function hashFromNav(nav: NavView, docId?: number | null): string {
  const base = NAV_TO_HASH[nav] || '#/'
  if (docId != null) {
    return `${base}?doc=${docId}`
  }
  return base
}

export function syncHashToState(nav: NavView, docId: number | null) {
  if (typeof window === 'undefined') return
  const newHash = hashFromNav(nav, docId)
  if (window.location.hash !== newHash) {
    window.location.hash = newHash
  }
}
