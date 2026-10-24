import { useState, useEffect } from 'react'
import { Trash2, RotateCcw, CheckSquare, Square } from 'lucide-react'
import { api } from '../api/client'
import { PageHeading } from '../components/TaxonomyManager'
import { confirmModal, alertModal } from '../store/useModalStore'

interface TrashedDoc {
  id: number
  title: string
  created: string
  deleted_at?: string
  original_file_name?: string
  thumbnail_url: string
}

export function Trash() {
  const [items, setItems] = useState<TrashedDoc[]>([])
  const [loading, setLoading] = useState(true)
  const [selectedIds, setSelectedIds] = useState<number[]>([])
  const [busy, setBusy] = useState(false)

  const loadTrash = async () => {
    setLoading(true)
    try {
      const data = await api.getTrash()
      setItems(data.results || [])
      setSelectedIds([])
    } catch (err) {
      console.error(err)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadTrash()
  }, [])

  const toggleSelect = (id: number) => {
    setSelectedIds((prev) =>
      prev.includes(id) ? prev.filter((i) => i !== id) : [...prev, id]
    )
  }

  const toggleSelectAll = () => {
    if (selectedIds.length === items.length) {
      setSelectedIds([])
    } else {
      setSelectedIds(items.map((i) => i.id))
    }
  }

  const handleRestore = async (ids: number[]) => {
    if (ids.length === 0) return
    setBusy(true)
    try {
      await api.restoreTrash(ids)
      await loadTrash()
    } catch (err: any) {
      await alertModal({
        title: 'Restore Failed',
        message: err.message || 'Failed to restore documents',
        type: 'danger',
      })
    } finally {
      setBusy(false)
    }
  }

  const handleEmpty = async (ids?: number[]) => {
    const isAll = !ids || ids.length === 0 || ids.length === items.length
    const promptMsg = isAll
      ? 'Permanently delete ALL documents in the trash? This cannot be undone.'
      : `Permanently delete ${ids.length} selected document(s)? This cannot be undone.`

    const ok = await confirmModal({
      title: isAll ? 'Empty Trash' : 'Permanently Delete Documents',
      message: promptMsg,
      confirmText: 'Permanently Delete',
      type: 'danger',
    })
    if (!ok) return

    setBusy(true)
    try {
      await api.emptyTrash(isAll ? undefined : ids)
      await loadTrash()
    } catch (err: any) {
      await alertModal({
        title: 'Delete Failed',
        message: err.message || 'Failed to permanently delete items',
        type: 'danger',
      })
    } finally {
      setBusy(false)
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <PageHeading
          title="Trash"
          subtitle="Documents deleted in FileDeck are held here for 30 days before permanent purging."
        />
        {items.length > 0 && (
          <div style={{ display: 'flex', gap: '10px' }}>
            <button
              className="btn btn-secondary"
              onClick={() => handleRestore(selectedIds.length > 0 ? selectedIds : items.map((i) => i.id))}
              disabled={busy}
              style={{ display: 'flex', alignItems: 'center', gap: '6px' }}
            >
              <RotateCcw size={15} />
              {selectedIds.length > 0 ? `Restore Selected (${selectedIds.length})` : 'Restore All'}
            </button>
            <button
              className="btn"
              onClick={() => handleEmpty(selectedIds.length > 0 ? selectedIds : undefined)}
              disabled={busy}
              style={{
                backgroundColor: 'rgba(239, 68, 68, 0.15)',
                color: '#ef4444',
                border: '1px solid rgba(239, 68, 68, 0.3)',
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
              }}
            >
              <Trash2 size={15} />
              {selectedIds.length > 0 ? `Empty Selected (${selectedIds.length})` : 'Empty Entire Trash'}
            </button>
          </div>
        )}
      </div>

      {loading ? (
        <div style={{ padding: '40px', textAlign: 'center', color: 'var(--text-muted)' }}>
          Loading trash...
        </div>
      ) : items.length === 0 ? (
        <div
          className="glass-panel"
          style={{
            padding: '48px',
            textAlign: 'center',
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            gap: '12px',
          }}
        >
          <Trash2 size={40} color="var(--text-muted)" style={{ opacity: 0.5 }} />
          <h4 style={{ fontSize: '1.1rem', fontWeight: 600 }}>Trash is Empty</h4>
          <p style={{ color: 'var(--text-muted)', fontSize: '0.9rem', maxWidth: '400px' }}>
            Documents you delete will show up here. You can restore them anytime or purge them immediately.
          </p>
        </div>
      ) : (
        <div className="glass-panel" style={{ overflow: 'hidden' }}>
          <div
            style={{
              padding: '12px 18px',
              borderBottom: '1px solid var(--border-subtle)',
              display: 'flex',
              alignItems: 'center',
              backgroundColor: 'rgba(0,0,0,0.15)',
              gap: '16px',
            }}
          >
            <button
              onClick={toggleSelectAll}
              style={{ background: 'transparent', border: 'none', cursor: 'pointer', color: 'var(--text-secondary)' }}
            >
              {selectedIds.length === items.length ? <CheckSquare size={18} /> : <Square size={18} />}
            </button>
            <span style={{ fontSize: '0.8rem', fontWeight: 600, color: 'var(--text-muted)', textTransform: 'uppercase' }}>
              {selectedIds.length > 0 ? `${selectedIds.length} of ${items.length} selected` : `${items.length} Trashed Document(s)`}
            </span>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column' }}>
            {items.map((doc) => {
              const isSelected = selectedIds.includes(doc.id)
              return (
                <div
                  key={doc.id}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    padding: '14px 18px',
                    borderBottom: '1px solid var(--border-subtle)',
                    gap: '16px',
                    backgroundColor: isSelected ? 'var(--accent-soft)' : 'transparent',
                    transition: 'background-color 0.15s ease',
                  }}
                >
                  <button
                    onClick={() => toggleSelect(doc.id)}
                    style={{ background: 'transparent', border: 'none', cursor: 'pointer', color: isSelected ? 'var(--accent-primary)' : 'var(--text-muted)' }}
                  >
                    {isSelected ? <CheckSquare size={18} /> : <Square size={18} />}
                  </button>

                  <div
                    style={{
                      width: '42px',
                      height: '52px',
                      borderRadius: '4px',
                      overflow: 'hidden',
                      backgroundColor: 'rgba(0,0,0,0.3)',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      flexShrink: 0,
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
                  </div>

                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ fontWeight: 600, fontSize: '0.95rem', color: 'var(--text-primary)' }}>
                      {doc.title}
                    </div>
                    <div style={{ display: 'flex', gap: '12px', fontSize: '0.75rem', color: 'var(--text-muted)', marginTop: '4px' }}>
                      <span>Original: {doc.original_file_name || 'unknown'}</span>
                      {doc.deleted_at && <span>Deleted: {new Date(doc.deleted_at).toLocaleString()}</span>}
                    </div>
                  </div>

                  <div style={{ display: 'flex', gap: '8px' }}>
                    <button
                      className="btn btn-secondary"
                      onClick={() => handleRestore([doc.id])}
                      title="Restore Document"
                      style={{ padding: '6px 12px', fontSize: '0.8rem' }}
                    >
                      <RotateCcw size={14} /> Restore
                    </button>
                    <button
                      className="btn"
                      onClick={() => handleEmpty([doc.id])}
                      title="Permanently Purge"
                      style={{
                        padding: '6px 12px',
                        fontSize: '0.8rem',
                        backgroundColor: 'rgba(239, 68, 68, 0.1)',
                        color: '#ef4444',
                        border: '1px solid rgba(239, 68, 68, 0.2)',
                      }}
                    >
                      <Trash2 size={14} /> Purge
                    </button>
                  </div>
                </div>
              )
            })}
          </div>
        </div>
      )}
    </div>
  )
}
