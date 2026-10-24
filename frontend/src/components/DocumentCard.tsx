import { FileText, Calendar } from 'lucide-react'
import type { DocumentItem, TaxonomyItem } from '../types'
import { TagBadges } from './TagBadges'

interface DocumentCardProps {
  doc: DocumentItem
  tags: TaxonomyItem[]
  onOpen: () => void
  selection?: { selected: boolean; onToggle: () => void }
}

export function DocumentCard({ doc, tags, onOpen, selection }: DocumentCardProps) {
  const compact = !selection

  return (
    <div
      className="card-panel card-panel-interactive"
      style={
        compact
          ? {
              padding: '16px',
              cursor: 'pointer',
              display: 'flex',
              flexDirection: 'column',
              gap: '12px',
              borderRadius: 'var(--radius-lg)',
            }
          : {
              padding: '16px',
              display: 'flex',
              flexDirection: 'column',
              gap: '12px',
              position: 'relative',
              borderRadius: 'var(--radius-lg)',
              borderColor: selection.selected ? 'var(--accent-primary)' : 'var(--border-subtle)',
              boxShadow: selection.selected
                ? '0 0 0 2px var(--accent-primary), var(--shadow-card)'
                : 'var(--shadow-card)',
            }
      }
      onClick={onOpen}
    >
      {selection && (
        <div
          style={{ position: 'absolute', top: '14px', left: '14px', zIndex: 10 }}
          onClick={(e) => {
            e.stopPropagation()
            selection.onToggle()
          }}
        >
          <input
            type="checkbox"
            checked={selection.selected}
            onChange={() => {}}
            style={{ cursor: 'pointer', width: '17px', height: '17px', accentColor: 'var(--accent-primary)' }}
          />
        </div>
      )}

      <div
        style={{
          height: compact ? '145px' : '165px',
          background: 'var(--bg-tertiary)',
          border: '1px solid var(--border-subtle)',
          borderRadius: 'var(--radius-md)',
          overflow: 'hidden',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          position: 'relative',
        }}
      >
        <img
          src={doc.thumbnail_url}
          alt={doc.title}
          style={{ width: '100%', height: '100%', objectFit: 'cover' }}
          onError={(e) => {
            e.currentTarget.style.display = 'none'
          }}
        />
        <FileText size={compact ? 36 : 42} style={{ color: 'var(--text-muted)', opacity: 0.35 }} />
      </div>

      <div>
        {compact ? (
          <>
            <div
              style={{
                fontWeight: 700,
                fontSize: '0.92rem',
                marginBottom: '4px',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
                color: 'var(--text-primary)',
              }}
            >
              {doc.title}
            </div>
            <div
              style={{
                fontSize: '0.75rem',
                color: 'var(--text-muted)',
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
              }}
            >
              <Calendar size={12} /> {doc.created}
            </div>
          </>
        ) : (
          <>
            <div style={{ fontWeight: 700, fontSize: '0.92rem', marginBottom: '4px', color: 'var(--text-primary)' }}>
              {doc.title}
            </div>
            <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
              {doc.created} · {doc.original_file_name}
            </div>
          </>
        )}
      </div>

      <div style={{ display: 'flex', flexWrap: 'wrap', gap: '4px', marginTop: 'auto' }}>
        <TagBadges tagIds={doc.tags} tags={tags} />
      </div>
    </div>
  )
}
