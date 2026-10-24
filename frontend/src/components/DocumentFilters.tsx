import { useState, useEffect } from 'react'
import { Filter, Grid, List as ListIcon, LayoutList } from 'lucide-react'
import { api } from '../api/client'
import { useFilterStore } from '../store/useFilterStore'
import { useTags } from '../api/useTags'
import { useCorrespondents } from '../api/useCorrespondents'
import { useDocumentTypes } from '../api/useDocumentTypes'
import type { ViewMode } from '../types'

const toId = (v: string): number | '' => (v ? parseInt(v, 10) : '')

function ViewModeTab({
  mode,
  label,
  icon: Icon,
}: {
  mode: ViewMode
  label: string
  icon: React.ComponentType<{ size?: number; style?: React.CSSProperties }>
}) {
  const viewMode = useFilterStore((s) => s.viewMode)
  const setViewMode = useFilterStore((s) => s.setViewMode)
  const active = viewMode === mode

  return (
    <button
      onClick={() => setViewMode(mode)}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: '6px',
        padding: '6px 14px',
        borderRadius: 'var(--radius-full)',
        border: 'none',
        background: active ? 'var(--bg-surface)' : 'transparent',
        color: active ? 'var(--accent-primary)' : 'var(--text-muted)',
        fontWeight: active ? 700 : 500,
        fontSize: '0.82rem',
        cursor: 'pointer',
        boxShadow: active ? 'var(--shadow-sm)' : 'none',
        transition: 'all 0.18s ease',
      }}
    >
      <Icon size={14} style={{ color: active ? 'var(--accent-primary)' : 'currentColor' }} />
      <span>{label}</span>
    </button>
  )
}

export function DocumentFilters() {
  const { tags } = useTags()
  const { correspondents } = useCorrespondents()
  const { documentTypes } = useDocumentTypes()
  const [savedViews, setSavedViews] = useState<any[]>([])
  const selectedSavedViewId = useFilterStore((s) => s.selectedSavedViewId)
  const applySavedView = useFilterStore((s) => s.applySavedView)
  const resetFilters = useFilterStore((s) => s.resetFilters)
  const tagFilter = useFilterStore((s) => s.tagFilter)
  const corrFilter = useFilterStore((s) => s.corrFilter)
  const typeFilter = useFilterStore((s) => s.typeFilter)
  const inboxOnly = useFilterStore((s) => s.inboxOnly)
  const setTagFilter = useFilterStore((s) => s.setTagFilter)
  const setCorrFilter = useFilterStore((s) => s.setCorrFilter)
  const setTypeFilter = useFilterStore((s) => s.setTypeFilter)
  const setInboxOnly = useFilterStore((s) => s.setInboxOnly)

  useEffect(() => {
    api
      .getSavedViews()
      .then((res) => setSavedViews(res.results || []))
      .catch(console.error)
  }, [])

  const handleApplySavedView = (viewIdStr: string) => {
    if (!viewIdStr) {
      resetFilters()
      return
    }
    const view = savedViews.find((v) => String(v.id) === viewIdStr)
    if (view) {
      applySavedView(view)
    }
  }

  return (
    <div
      className="card-panel"
      style={{
        padding: '16px 20px',
        display: 'flex',
        flexWrap: 'wrap',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: '14px',
        borderRadius: 'var(--radius-lg)',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: '10px', flexWrap: 'wrap' }}>
        <div
          style={{
            width: '32px',
            height: '32px',
            borderRadius: 'var(--radius-full)',
            background: 'var(--accent-soft)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: 'var(--accent-primary)',
          }}
        >
          <Filter size={15} />
        </div>

        {savedViews.length > 0 && (
          <select
            className="input"
            style={{ width: '160px', borderRadius: 'var(--radius-full)' }}
            value={selectedSavedViewId ? String(selectedSavedViewId) : ''}
            onChange={(e) => handleApplySavedView(e.target.value)}
          >
            <option value="">Saved Views...</option>
            {savedViews.map((sv) => (
              <option key={sv.id} value={sv.id}>
                👁 {sv.name}
              </option>
            ))}
          </select>
        )}

        <select
          className="input"
          style={{ width: '150px', borderRadius: 'var(--radius-full)' }}
          value={tagFilter}
          onChange={(e) => setTagFilter(toId(e.target.value))}
        >
          <option value="">All Tags</option>
          {tags.map((t) => (
            <option key={t.id} value={t.id}>
              {t.name}
            </option>
          ))}
        </select>

        <select
          className="input"
          style={{ width: '160px', borderRadius: 'var(--radius-full)' }}
          value={corrFilter}
          onChange={(e) => setCorrFilter(toId(e.target.value))}
        >
          <option value="">All Correspondents</option>
          {correspondents.map((c) => (
            <option key={c.id} value={c.id}>
              {c.name}
            </option>
          ))}
        </select>

        <select
          className="input"
          style={{ width: '150px', borderRadius: 'var(--radius-full)' }}
          value={typeFilter}
          onChange={(e) => setTypeFilter(toId(e.target.value))}
        >
          <option value="">All Types</option>
          {documentTypes.map((t) => (
            <option key={t.id} value={t.id}>
              {t.name}
            </option>
          ))}
        </select>

        <button
          className={`btn ${inboxOnly ? 'btn-primary' : 'btn-secondary'}`}
          style={{ padding: '7px 14px', fontSize: '0.8rem', borderRadius: 'var(--radius-full)' }}
          onClick={() => setInboxOnly(!inboxOnly)}
        >
          Inbox Only
        </button>

        {(tagFilter !== '' || corrFilter !== '' || typeFilter !== '' || inboxOnly) && (
          <button
            className="btn btn-secondary"
            style={{ padding: '7px 12px', fontSize: '0.78rem', color: 'var(--text-muted)', borderRadius: 'var(--radius-full)' }}
            onClick={() => {
              resetFilters()
            }}
          >
            Reset
          </button>
        )}
      </div>

      {/* Segmented Pill View Mode Switcher (Matching Reference) */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '2px',
          backgroundColor: 'var(--bg-tertiary)',
          padding: '4px',
          borderRadius: 'var(--radius-full)',
          border: '1px solid var(--border-subtle)',
        }}
      >
        <ViewModeTab mode="table" label="List View" icon={ListIcon} />
        <ViewModeTab mode="cards" label="Card View" icon={Grid} />
        <ViewModeTab mode="large_cards" label="Large Cards" icon={LayoutList} />
      </div>
    </div>
  )
}
