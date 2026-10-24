import { useState } from 'react'
import { Download, Trash2, Edit3, X } from 'lucide-react'
import { useFilterStore } from '../store/useFilterStore'
import { useBulkActions } from '../hooks/useBulkActions'
import { BulkEditModal } from './BulkEditModal'

export function BulkActionBar() {
  const [isEditModalOpen, setIsEditModalOpen] = useState(false)
  const count = useFilterStore((s) => s.selectedDocIds.length)
  const setSelectedDocIds = useFilterStore((s) => s.setSelectedDocIds)
  const { bulkDelete, bulkDownload } = useBulkActions()

  if (count === 0) return null

  return (
    <>
      <div
        className="card-panel animate-fade-in"
        style={{
          padding: '10px 22px',
          display: 'flex',
          flexWrap: 'wrap',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: '12px',
          backgroundColor: 'var(--bg-surface)',
          borderColor: 'var(--accent-primary)',
          boxShadow: 'var(--shadow-md)',
          borderRadius: 'var(--radius-full)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <span style={{ fontWeight: 600, fontSize: '0.875rem', color: 'var(--text-primary)' }}>
            {count} document{count > 1 ? 's' : ''} selected
          </span>

          <button
            onClick={() => setSelectedDocIds([])}
            style={{
              background: 'transparent',
              border: 'none',
              color: 'var(--text-muted)',
              fontSize: '0.78rem',
              cursor: 'pointer',
              display: 'inline-flex',
              alignItems: 'center',
              gap: '4px',
              padding: '2px 6px',
            }}
          >
            <X size={13} /> Deselect all
          </button>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexWrap: 'wrap' }}>
          <button
            className="btn btn-primary"
            style={{ padding: '6px 12px', fontSize: '0.82rem', gap: '6px' }}
            onClick={() => setIsEditModalOpen(true)}
          >
            <Edit3 size={14} /> Bulk Edit
          </button>

          <button
            className="btn btn-secondary"
            style={{ padding: '6px 12px', fontSize: '0.82rem', gap: '6px' }}
            onClick={bulkDownload}
          >
            <Download size={14} /> Download ZIP
          </button>

          <button
            className="btn btn-danger"
            style={{ padding: '6px 12px', fontSize: '0.82rem', gap: '6px' }}
            onClick={bulkDelete}
          >
            <Trash2 size={14} /> Delete
          </button>
        </div>
      </div>

      <BulkEditModal isOpen={isEditModalOpen} onClose={() => setIsEditModalOpen(false)} />
    </>
  )
}
