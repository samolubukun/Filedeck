import React, { useState, useEffect, useRef } from 'react'
import { Search, X, FileText, Tag as TagIcon, User, Layers, Eye } from 'lucide-react'
import { api } from '../api/client'
import { useUiStore } from '../store/useUiStore'
import { useFilterStore } from '../store/useFilterStore'

interface GlobalSearchResult {
  documents?: Array<{ id: number; title: string; created: string }>
  tags?: Array<{ id: number; name: string; color?: string }>
  correspondents?: Array<{ id: number; name: string }>
  document_types?: Array<{ id: number; name: string }>
  saved_views?: Array<{ id: number; name: string }>
}

interface GlobalSearchModalProps {
  isOpen: boolean
  onClose: () => void
}

export const GlobalSearchModal: React.FC<GlobalSearchModalProps> = ({ isOpen, onClose }) => {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<GlobalSearchResult | null>(null)
  const [loading, setLoading] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)

  const setActiveDocId = useUiStore((s) => s.setActiveDocId)
  const setCurrentNav = useUiStore((s) => s.setCurrentNav)
  const setTagFilter = useFilterStore((s) => s.setTagFilter)
  const setCorrFilter = useFilterStore((s) => s.setCorrFilter)
  const setTypeFilter = useFilterStore((s) => s.setTypeFilter)

  useEffect(() => {
    if (isOpen) {
      setTimeout(() => inputRef.current?.focus(), 50)
    } else {
      setQuery('')
      setResults(null)
    }
  }, [isOpen])

  // Global Ctrl+K / Cmd+K handler
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault()
        if (isOpen) {
          onClose()
        } else {
          // Open handled by caller or state
        }
      } else if (e.key === 'Escape' && isOpen) {
        onClose()
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [isOpen, onClose])

  useEffect(() => {
    const term = query.trim()
    if (term.length < 2) {
      setResults(null)
      setLoading(false)
      return
    }

    setLoading(true)
    const timeout = setTimeout(async () => {
      try {
        const data = await api.globalSearch(term)
        setResults(data)
      } catch (err) {
        console.error(err)
      } finally {
        setLoading(false)
      }
    }, 200)

    return () => clearTimeout(timeout)
  }, [query])

  if (!isOpen) return null

  const handleSelectDoc = (id: number) => {
    setActiveDocId(id)
    onClose()
  }

  const handleSelectTag = (id: number) => {
    setTagFilter(id)
    setCurrentNav('documents')
    onClose()
  }

  const handleSelectCorr = (id: number) => {
    setCorrFilter(id)
    setCurrentNav('documents')
    onClose()
  }

  const handleSelectType = (id: number) => {
    setTypeFilter(id)
    setCurrentNav('documents')
    onClose()
  }

  const hasResults =
    results &&
    ((results.documents && results.documents.length > 0) ||
      (results.tags && results.tags.length > 0) ||
      (results.correspondents && results.correspondents.length > 0) ||
      (results.document_types && results.document_types.length > 0) ||
      (results.saved_views && results.saved_views.length > 0))

  return (
    <div
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.75)',
        backdropFilter: 'blur(8px)',
        zIndex: 9999,
        display: 'flex',
        alignItems: 'flex-start',
        justifyContent: 'center',
        paddingTop: '10vh',
      }}
      onClick={onClose}
    >
      <div
        className="card-panel"
        style={{
          width: '640px',
          maxWidth: '92vw',
          maxHeight: '75vh',
          display: 'flex',
          flexDirection: 'column',
          borderRadius: 'var(--radius-xl)',
          boxShadow: 'var(--shadow-lg)',
          border: '1px solid var(--border-subtle)',
          backgroundColor: 'var(--bg-card)',
          overflow: 'hidden',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Search Input Bar */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            padding: '16px 20px',
            borderBottom: '1px solid var(--border-subtle)',
            gap: '12px',
          }}
        >
          <Search size={20} style={{ color: 'var(--accent-primary)', flexShrink: 0 }} />
          <input
            ref={inputRef}
            type="text"
            placeholder="Search documents, tags, correspondents, views..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            style={{
              flex: 1,
              background: 'transparent',
              border: 'none',
              outline: 'none',
              fontSize: '1.05rem',
              color: 'var(--text-primary)',
            }}
          />
          {loading && (
            <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>Searching...</span>
          )}
          <button
            onClick={onClose}
            style={{
              background: 'transparent',
              border: 'none',
              color: 'var(--text-muted)',
              cursor: 'pointer',
              padding: '4px',
            }}
          >
            <X size={18} />
          </button>
        </div>

        {/* Results Container */}
        <div style={{ flex: 1, overflowY: 'auto', padding: '16px 20px' }}>
          {query.trim().length < 2 ? (
            <div style={{ padding: '24px 0', textAlign: 'center', color: 'var(--text-muted)', fontSize: '0.9rem' }}>
              Type at least 2 characters to search across everything...
            </div>
          ) : !hasResults && !loading ? (
            <div style={{ padding: '32px 0', textAlign: 'center', color: 'var(--text-muted)' }}>
              No matches found for <strong style={{ color: 'var(--text-primary)' }}>"{query}"</strong>
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
              {/* Documents */}
              {results?.documents && results.documents.length > 0 && (
                <div>
                  <div style={{ fontSize: '0.75rem', fontWeight: 700, textTransform: 'uppercase', color: 'var(--text-muted)', marginBottom: '8px' }}>
                    Documents ({results.documents.length})
                  </div>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                    {results.documents.map((doc) => (
                      <div
                        key={doc.id}
                        onClick={() => handleSelectDoc(doc.id)}
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'space-between',
                          padding: '10px 14px',
                          borderRadius: 'var(--radius-sm)',
                          backgroundColor: 'rgba(255,255,255,0.03)',
                          cursor: 'pointer',
                          transition: 'background 0.15s ease',
                        }}
                        onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = 'var(--accent-soft)')}
                        onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = 'transparent')}
                      >
                        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                          <FileText size={16} style={{ color: 'var(--accent-primary)' }} />
                          <span style={{ fontSize: '0.9rem', fontWeight: 500 }}>{doc.title}</span>
                        </div>
                        <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                          {new Date(doc.created).toLocaleDateString()}
                        </span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Tags */}
              {results?.tags && results.tags.length > 0 && (
                <div>
                  <div style={{ fontSize: '0.75rem', fontWeight: 700, textTransform: 'uppercase', color: 'var(--text-muted)', marginBottom: '8px' }}>
                    Tags
                  </div>
                  <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                    {results.tags.map((t) => (
                      <button
                        key={t.id}
                        onClick={() => handleSelectTag(t.id)}
                        className="btn btn-secondary"
                        style={{ fontSize: '0.8rem', padding: '6px 12px', display: 'flex', alignItems: 'center', gap: '6px' }}
                      >
                        <TagIcon size={12} style={{ color: t.color || 'var(--accent-primary)' }} />
                        {t.name}
                      </button>
                    ))}
                  </div>
                </div>
              )}

              {/* Correspondents */}
              {results?.correspondents && results.correspondents.length > 0 && (
                <div>
                  <div style={{ fontSize: '0.75rem', fontWeight: 700, textTransform: 'uppercase', color: 'var(--text-muted)', marginBottom: '8px' }}>
                    Correspondents
                  </div>
                  <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                    {results.correspondents.map((c) => (
                      <button
                        key={c.id}
                        onClick={() => handleSelectCorr(c.id)}
                        className="btn btn-secondary"
                        style={{ fontSize: '0.8rem', padding: '6px 12px', display: 'flex', alignItems: 'center', gap: '6px' }}
                      >
                        <User size={12} style={{ color: 'var(--accent-primary)' }} />
                        {c.name}
                      </button>
                    ))}
                  </div>
                </div>
              )}

              {/* Document Types */}
              {results?.document_types && results.document_types.length > 0 && (
                <div>
                  <div style={{ fontSize: '0.75rem', fontWeight: 700, textTransform: 'uppercase', color: 'var(--text-muted)', marginBottom: '8px' }}>
                    Document Types
                  </div>
                  <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                    {results.document_types.map((dt) => (
                      <button
                        key={dt.id}
                        onClick={() => handleSelectType(dt.id)}
                        className="btn btn-secondary"
                        style={{ fontSize: '0.8rem', padding: '6px 12px', display: 'flex', alignItems: 'center', gap: '6px' }}
                      >
                        <Layers size={12} style={{ color: 'var(--accent-primary)' }} />
                        {dt.name}
                      </button>
                    ))}
                  </div>
                </div>
              )}

              {/* Saved Views */}
              {results?.saved_views && results.saved_views.length > 0 && (
                <div>
                  <div style={{ fontSize: '0.75rem', fontWeight: 700, textTransform: 'uppercase', color: 'var(--text-muted)', marginBottom: '8px' }}>
                    Saved Views
                  </div>
                  <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                    {results.saved_views.map((sv) => (
                      <button
                        key={sv.id}
                        onClick={() => {
                          setCurrentNav('documents')
                          onClose()
                        }}
                        className="btn btn-secondary"
                        style={{ fontSize: '0.8rem', padding: '6px 12px', display: 'flex', alignItems: 'center', gap: '6px' }}
                      >
                        <Eye size={12} style={{ color: 'var(--accent-primary)' }} />
                        {sv.name}
                      </button>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer shortcuts helper */}
        <div
          style={{
            padding: '10px 20px',
            backgroundColor: 'rgba(0,0,0,0.3)',
            borderTop: '1px solid var(--border-subtle)',
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            fontSize: '0.75rem',
            color: 'var(--text-muted)',
          }}
        >
          <span>Use <strong>Esc</strong> to close</span>
          <span>Shortcut: <strong>Ctrl + K</strong> / <strong>⌘K</strong></span>
        </div>
      </div>
    </div>
  )
}
