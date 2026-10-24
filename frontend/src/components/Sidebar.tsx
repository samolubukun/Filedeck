import { useState, useEffect, type ReactNode } from 'react'
import {
  Files,
  LayoutDashboard,
  Tag,
  UserCheck,
  FolderTree,
  Settings,
  HardDrive,
  Wrench,
  Hash,
  ListTodo,
  Trash2,
  Terminal,
  Sparkles,
  Eye,
  GitBranch,
  Mail,
  ChevronLeft,
  ChevronRight,
  ChevronDown,
  Filter,
  FileText,
  X,
} from 'lucide-react'
import { useUiStore } from '../store/useUiStore'
import { useFilterStore } from '../store/useFilterStore'
import { useDocuments } from '../api/useDocuments'
import { api } from '../api/client'
import { LogoBadge, FileDeckText } from './Logo'
import { hashFromNav } from '../lib/routing'
import type { NavView } from '../types'

interface NavButtonProps {
  view: NavView
  primary?: boolean
  title?: string
  children: ReactNode
}

function NavButton({ view, primary, title, children }: NavButtonProps) {
  const currentNav = useUiStore((s) => s.currentNav)
  const setCurrentNav = useUiStore((s) => s.setCurrentNav)
  const sidebarCollapsed = useUiStore((s) => s.sidebarCollapsed)
  const active = currentNav === view

  return (
    <a
      href={hashFromNav(view)}
      onClick={(e) => {
        if (!e.ctrlKey && !e.metaKey && !e.shiftKey) {
          e.preventDefault()
          setCurrentNav(view)
        }
      }}
      title={sidebarCollapsed ? title : undefined}
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: sidebarCollapsed ? 'center' : 'flex-start',
        gap: sidebarCollapsed ? '0' : '11px',
        padding: sidebarCollapsed ? '10px 0' : primary ? '10px 14px' : '9px 14px',
        borderRadius: '10px',
        border: active ? '1px solid var(--accent-border)' : '1px solid transparent',
        background: active ? 'var(--accent-soft)' : 'transparent',
        color: active ? 'var(--accent-primary)' : 'var(--text-secondary)',
        fontWeight: active ? 700 : primary ? 600 : 500,
        fontSize: '0.86rem',
        cursor: 'pointer',
        textAlign: 'left',
        width: '100%',
        textDecoration: 'none',
        boxSizing: 'border-box',
        transition: 'all 0.18s ease',
        position: 'relative',
      }}
      onMouseEnter={(e) => {
        if (!active) {
          e.currentTarget.style.backgroundColor = 'var(--bg-tertiary)'
          e.currentTarget.style.color = 'var(--text-primary)'
        }
      }}
      onMouseLeave={(e) => {
        if (!active) {
          e.currentTarget.style.backgroundColor = 'transparent'
          e.currentTarget.style.color = 'var(--text-secondary)'
        }
      }}
    >
      {children}
    </a>
  )
}

function SectionLabel({ children }: { children: ReactNode }) {
  const sidebarCollapsed = useUiStore((s) => s.sidebarCollapsed)
  if (sidebarCollapsed) {
    return (
      <div
        style={{
          height: '1px',
          backgroundColor: 'var(--border-subtle)',
          margin: '12px 10px',
          opacity: 0.7,
        }}
      />
    )
  }

  return (
    <div
      style={{
        fontSize: '0.68rem',
        fontWeight: 700,
        textTransform: 'uppercase',
        letterSpacing: '0.08em',
        color: 'var(--text-muted)',
        margin: '18px 14px 6px',
        opacity: 0.9,
      }}
    >
      {children}
    </div>
  )
}

export function Sidebar() {
  const wsConnected = useUiStore((s) => s.wsConnected)
  const sidebarCollapsed = useUiStore((s) => s.sidebarCollapsed)
  const toggleSidebarCollapsed = useUiStore((s) => s.toggleSidebarCollapsed)
  const setCurrentNav = useUiStore((s) => s.setCurrentNav)
  const currentNav = useUiStore((s) => s.currentNav)
  const activeDocId = useUiStore((s) => s.activeDocId)
  const setActiveDocId = useUiStore((s) => s.setActiveDocId)
  const selectedSavedViewId = useFilterStore((s) => s.selectedSavedViewId)
  const applySavedView = useFilterStore((s) => s.applySavedView)
  const { totalDocs, documents } = useDocuments()
  const openDoc = documents.find((d) => d.id === activeDocId)
  const [savedViews, setSavedViews] = useState<any[]>([])
  const [savedViewsCollapsed, setSavedViewsCollapsed] = useState(() => {
    if (typeof window !== 'undefined') {
      return localStorage.getItem('filedeck_saved_views_collapsed') === 'true'
    }
    return false
  })

  const toggleSavedViewsCollapsed = () => {
    setSavedViewsCollapsed((prev) => {
      const next = !prev
      localStorage.setItem('filedeck_saved_views_collapsed', String(next))
      return next
    })
  }

  useEffect(() => {
    api
      .getSavedViews()
      .then((res) => {
        if (res && res.results) {
          setSavedViews(res.results)
        }
      })
      .catch(() => {})
  }, [currentNav])

  return (
    <aside
      style={{
        width: sidebarCollapsed ? '64px' : '280px',
        minWidth: sidebarCollapsed ? '64px' : '280px',
        flexShrink: 0,
        borderRight: '1px solid var(--border-subtle)',
        backgroundColor: 'var(--bg-secondary)',
        display: 'flex',
        flexDirection: 'column',
        position: 'sticky',
        top: 0,
        height: '100vh',
        maxHeight: '100vh',
        zIndex: 50,
        overflow: 'hidden',
        transition: 'width 0.2s ease, min-width 0.2s ease, background-color 0.25s ease',
      }}
    >
      {/* Brand Header: Always Pinned at Top */}
      <div
        style={{
          padding: sidebarCollapsed ? '18px 12px' : '20px 18px 18px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: sidebarCollapsed ? 'center' : 'flex-start',
          gap: '12px',
          flexShrink: 0,
        }}
      >
        <div
          onClick={() => setCurrentNav('dashboard')}
          title="FileDeck Unified Vault"
          style={{ cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0 }}
        >
          <LogoBadge size={38} iconSize={21} />
        </div>
        {!sidebarCollapsed && (
          <div style={{ minWidth: 0, flex: 1, overflow: 'visible' }}>
            <div style={{ display: 'flex', alignItems: 'center' }}>
              <h1 style={{ margin: 0, padding: 0, lineHeight: 1.15 }}>
                <FileDeckText fontSize="1.25rem" />
              </h1>
            </div>
            <span style={{ fontSize: '0.68rem', color: 'var(--text-muted)', fontWeight: 500, letterSpacing: '-0.02em', display: 'block', whiteSpace: 'nowrap' }}>
              Your documents. One workspace.
            </span>
          </div>
        )}
      </div>


      {/* Nav Menu Items: Dedicated Scrollable Area */}
      <nav
        style={{
          flex: 1,
          minHeight: 0,
          overflowY: 'auto',
          overflowX: 'hidden',
          padding: sidebarCollapsed ? '0 6px' : '0 10px',
          display: 'flex',
          flexDirection: 'column',
          gap: '3px',
        }}
      >
        <NavButton view="dashboard" primary title="Dashboard">
          <LayoutDashboard size={17} />
          {!sidebarCollapsed && <span>Dashboard</span>}
        </NavButton>

        <NavButton view="documents" primary title="Documents">
          <Files size={17} />
          {!sidebarCollapsed && (
            <>
              <span>Documents</span>
              <span
                style={{
                  marginLeft: 'auto',
                  fontSize: '0.72rem',
                  fontWeight: 700,
                  backgroundColor: 'var(--bg-tertiary)',
                  color: 'var(--text-secondary)',
                  padding: '2px 8px',
                  borderRadius: 'var(--radius-full)',
                }}
              >
                {totalDocs}
              </span>
            </>
          )}
        </NavButton>

        <NavButton view="pdf_studio" primary title="PDF Studio">
          <Wrench size={17} style={{ color: '#ec4899' }} />
          {!sidebarCollapsed && (
            <>
              <span>PDF Studio</span>
              <span
                style={{
                  marginLeft: 'auto',
                  fontSize: '0.68rem',
                  fontWeight: 700,
                  backgroundColor: 'rgba(236, 72, 153, 0.15)',
                  color: '#ec4899',
                  padding: '2px 7px',
                  borderRadius: 'var(--radius-full)',
                }}
              >
                30
              </span>
            </>
          )}
        </NavButton>

        <NavButton view="chat" primary title="Ask AI">
          <Sparkles size={17} style={{ color: 'var(--accent-primary)' }} />
          {!sidebarCollapsed && (
            <>
              <span>Ask AI</span>
              <span
                style={{
                  marginLeft: 'auto',
                  fontSize: '0.65rem',
                  fontWeight: 700,
                  letterSpacing: '0.04em',
                  background: 'var(--accent-soft)',
                  color: 'var(--accent-primary)',
                  border: '1px solid var(--accent-border)',
                  padding: '1px 7px',
                  borderRadius: 'var(--radius-full)',
                }}
              >
                ✦ AI
              </span>
            </>
          )}
        </NavButton>

        {/* Collapsible Saved Views */}
        {!sidebarCollapsed && savedViews.length > 0 && (
          <div style={{ marginTop: '8px' }}>
            <div
              onClick={toggleSavedViewsCollapsed}
              role="button"
              tabIndex={0}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault()
                  toggleSavedViewsCollapsed()
                }
              }}
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                padding: '6px 14px',
                cursor: 'pointer',
                userSelect: 'none',
                borderRadius: '8px',
                transition: 'background-color 0.15s ease',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.backgroundColor = 'var(--bg-tertiary)'
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.backgroundColor = 'transparent'
              }}
              title={savedViewsCollapsed ? 'Expand Saved Views' : 'Collapse Saved Views'}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '7px' }}>
                <span
                  style={{
                    fontSize: '0.68rem',
                    fontWeight: 700,
                    textTransform: 'uppercase',
                    letterSpacing: '0.08em',
                    color: 'var(--text-muted)',
                  }}
                >
                  Saved Views
                </span>
                <span
                  style={{
                    fontSize: '0.65rem',
                    fontWeight: 700,
                    backgroundColor: 'var(--bg-tertiary)',
                    color: 'var(--text-muted)',
                    padding: '1px 6px',
                    borderRadius: 'var(--radius-full)',
                  }}
                >
                  {savedViews.length}
                </span>
              </div>
              <span
                style={{
                  color: 'var(--text-muted)',
                  display: 'flex',
                  alignItems: 'center',
                  transition: 'transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                  transform: savedViewsCollapsed ? 'rotate(-90deg)' : 'rotate(0deg)',
                }}
              >
                <ChevronDown size={14} />
              </span>
            </div>

            {!savedViewsCollapsed && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '2px', marginTop: '2px' }}>
                {savedViews.map((sv) => {
                  const isActive = currentNav === 'documents' && selectedSavedViewId === sv.id
                  return (
                    <button
                      key={sv.id}
                      onClick={() => {
                        applySavedView(sv)
                        setCurrentNav('documents')
                      }}
                      title={`Filter documents: ${sv.name}`}
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: '9px',
                        padding: '7px 12px 7px 14px',
                        borderRadius: '10px',
                        border: isActive ? '1px solid var(--accent-border)' : '1px solid transparent',
                        background: isActive ? 'var(--accent-soft)' : 'transparent',
                        color: isActive ? 'var(--accent-primary)' : 'var(--text-secondary)',
                        fontWeight: isActive ? 700 : 500,
                        fontSize: '0.82rem',
                        cursor: 'pointer',
                        textAlign: 'left',
                        width: '100%',
                        transition: 'all 0.15s ease',
                        boxSizing: 'border-box',
                      }}
                      onMouseEnter={(e) => {
                        if (!isActive) {
                          e.currentTarget.style.backgroundColor = 'var(--bg-tertiary)'
                          e.currentTarget.style.color = 'var(--text-primary)'
                        }
                      }}
                      onMouseLeave={(e) => {
                        if (!isActive) {
                          e.currentTarget.style.backgroundColor = 'transparent'
                          e.currentTarget.style.color = 'var(--text-secondary)'
                        }
                      }}
                    >
                      <Filter
                        size={13}
                        style={{
                          opacity: isActive ? 1 : 0.7,
                          flexShrink: 0,
                          color: isActive ? 'var(--accent-primary)' : 'inherit',
                        }}
                      />
                      <span
                        style={{
                          flex: 1,
                          overflow: 'hidden',
                          textOverflow: 'ellipsis',
                          whiteSpace: 'nowrap',
                        }}
                      >
                        {sv.name}
                      </span>
                      {isActive && (
                        <span
                          style={{
                            width: '6px',
                            height: '6px',
                            borderRadius: '50%',
                            backgroundColor: 'var(--accent-primary)',
                            flexShrink: 0,
                          }}
                        />
                      )}
                    </button>
                  )
                })}
              </div>
            )}
          </div>
        )}

        {/* Open Documents (Paperless Parity) */}
        {!sidebarCollapsed && activeDocId && (
          <div style={{ marginTop: '8px' }}>
            <SectionLabel>Open Documents</SectionLabel>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                padding: '8px 12px',
                borderRadius: '10px',
                backgroundColor: 'rgba(16, 185, 129, 0.12)',
                border: '1px solid rgba(16, 185, 129, 0.3)',
                color: '#10b981',
                fontSize: '0.82rem',
                fontWeight: 600,
                gap: '8px',
                margin: '2px 0',
              }}
            >
              <div
                style={{ display: 'flex', alignItems: 'center', gap: '8px', overflow: 'hidden', cursor: 'pointer', flex: 1 }}
                onClick={() => setActiveDocId(activeDocId)}
                title={openDoc?.title || `Document #${activeDocId}`}
              >
                <FileText size={15} style={{ flexShrink: 0 }} />
                <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                  {openDoc?.title || `Document #${activeDocId}`}
                </span>
              </div>
              <button
                onClick={(e) => {
                  e.stopPropagation()
                  setActiveDocId(null)
                }}
                title="Close Document"
                style={{
                  background: 'transparent',
                  border: 'none',
                  color: '#10b981',
                  cursor: 'pointer',
                  padding: '2px',
                  display: 'flex',
                  alignItems: 'center',
                  opacity: 0.8,
                  borderRadius: '4px',
                }}
              >
                <X size={14} />
              </button>
            </div>
          </div>
        )}

        <SectionLabel>Views & Taxonomy</SectionLabel>

        <NavButton view="saved_views" title="Saved Views">
          <Eye size={16} />
          {!sidebarCollapsed && <span>Saved Views</span>}
        </NavButton>
        <NavButton view="tags" title="Tags">
          <Tag size={16} />
          {!sidebarCollapsed && <span>Tags</span>}
        </NavButton>
        <NavButton view="correspondents" title="Correspondents">
          <UserCheck size={16} />
          {!sidebarCollapsed && <span>Correspondents</span>}
        </NavButton>
        <NavButton view="document_types" title="Document Types">
          <FolderTree size={16} />
          {!sidebarCollapsed && <span>Document Types</span>}
        </NavButton>
        <NavButton view="storage_paths" title="Storage Paths">
          <HardDrive size={16} />
          {!sidebarCollapsed && <span>Storage Paths</span>}
        </NavButton>
        <NavButton view="custom_fields" title="Custom Fields">
          <Hash size={16} />
          {!sidebarCollapsed && <span>Custom Fields</span>}
        </NavButton>

        <SectionLabel>Automation & Mail</SectionLabel>

        <NavButton view="workflows" title="Workflows">
          <GitBranch size={16} />
          {!sidebarCollapsed && <span>Workflows</span>}
        </NavButton>
        <NavButton view="mail" title="Mail Fetcher">
          <Mail size={16} />
          {!sidebarCollapsed && <span>Mail Fetcher</span>}
        </NavButton>

        <SectionLabel>System & Operations</SectionLabel>

        <NavButton view="tasks" title="Task Queue">
          <ListTodo size={16} />
          {!sidebarCollapsed && <span>Task Queue</span>}
        </NavButton>
        <NavButton view="trash" title="Trash">
          <Trash2 size={16} />
          {!sidebarCollapsed && <span>Trash</span>}
        </NavButton>
        <NavButton view="logs" title="System Logs">
          <Terminal size={16} />
          {!sidebarCollapsed && <span>System Logs</span>}
        </NavButton>
        <NavButton view="settings" title="Settings & Status">
          <Settings size={16} />
          {!sidebarCollapsed && <span>Settings & Status</span>}
        </NavButton>
      </nav>

      {/* Sync Status & Collapse Toggle Footer: Always Anchored at Bottom */}
      <div
        style={{
          flexShrink: 0,
          marginTop: 'auto',
          padding: sidebarCollapsed ? '12px 8px' : '14px 16px',
          borderTop: '1px solid var(--border-subtle)',
          backgroundColor: 'var(--bg-tertiary)',
          display: 'flex',
          flexDirection: sidebarCollapsed ? 'column' : 'row',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: '8px',
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            fontSize: '0.75rem',
            color: 'var(--text-muted)',
            overflow: 'hidden',
          }}
          title={wsConnected ? 'Sync Stream Active' : 'Polling Feed'}
        >
          <span
            style={{
              width: '8px',
              height: '8px',
              borderRadius: '50%',
              backgroundColor: wsConnected ? '#10b981' : '#f59e0b',
              boxShadow: wsConnected ? '0 0 8px rgba(16, 185, 129, 0.6)' : 'none',
              display: 'inline-block',
              flexShrink: 0,
            }}
          />
          {!sidebarCollapsed && (
            <span style={{ fontWeight: 600, whiteSpace: 'nowrap', color: 'var(--text-secondary)' }}>
              {wsConnected ? 'Live Sync' : 'Polling'}
            </span>
          )}
        </div>

        <button
          onClick={toggleSidebarCollapsed}
          title={sidebarCollapsed ? 'Expand Sidebar' : 'Collapse Sidebar'}
          style={{
            background: 'var(--bg-surface)',
            border: '1px solid var(--border-subtle)',
            color: 'var(--text-secondary)',
            cursor: 'pointer',
            padding: '6px',
            borderRadius: 'var(--radius-sm)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            transition: 'all 0.15s ease',
            boxShadow: 'var(--shadow-sm)',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.color = 'var(--accent-primary)'
            e.currentTarget.style.borderColor = 'var(--accent-primary)'
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.color = 'var(--text-secondary)'
            e.currentTarget.style.borderColor = 'var(--border-subtle)'
          }}
        >
          {sidebarCollapsed ? <ChevronRight size={15} /> : <ChevronLeft size={15} />}
        </button>
      </div>
    </aside>
  )
}
