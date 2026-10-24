import { useState } from 'react'
import { X, Plus, Minus, Tag as TagIcon, UserCheck, FolderTree, HardDrive, RefreshCw, Layers } from 'lucide-react'
import { api } from '../api/client'
import { useRefreshAll } from '../api/useDocuments'
import { useTags } from '../api/useTags'
import { useCorrespondents } from '../api/useCorrespondents'
import { useDocumentTypes } from '../api/useDocumentTypes'
import { useStoragePaths } from '../api/useStoragePaths'
import { useFilterStore } from '../store/useFilterStore'
import { confirmModal, alertModal } from '../store/useModalStore'

interface BulkEditModalProps {
  isOpen: boolean
  onClose: () => void
}

type TagAction = 'none' | 'add' | 'remove'

export function BulkEditModal({ isOpen, onClose }: BulkEditModalProps) {
  const selectedDocIds = useFilterStore((s) => s.selectedDocIds)
  const setSelectedDocIds = useFilterStore((s) => s.setSelectedDocIds)
  const refreshAll = useRefreshAll()

  const { tags } = useTags()
  const { correspondents } = useCorrespondents()
  const { documentTypes } = useDocumentTypes()
  const { storagePaths } = useStoragePaths()

  // Tri-state tags map: tagId -> 'none' | 'add' | 'remove'
  const [tagActions, setTagActions] = useState<Record<number, TagAction>>({})
  const [selectedCorr, setSelectedCorr] = useState<string>('keep')
  const [selectedType, setSelectedType] = useState<string>('keep')
  const [selectedPath, setSelectedPath] = useState<string>('keep')
  const [tagSearch, setTagSearch] = useState('')
  const [saving, setSaving] = useState(false)

  if (!isOpen) return null

  const toggleTagAction = (tagId: number) => {
    setTagActions((prev) => {
      const current = prev[tagId] || 'none'
      let next: TagAction = 'none'
      if (current === 'none') next = 'add'
      else if (current === 'add') next = 'remove'
      else next = 'none'
      return { ...prev, [tagId]: next }
    })
  }

  const handleApply = async () => {
    setSaving(true)
    try {
      // 1. Tags
      const addTags = Object.entries(tagActions)
        .filter(([_, action]) => action === 'add')
        .map(([id]) => parseInt(id, 10))
      const removeTags = Object.entries(tagActions)
        .filter(([_, action]) => action === 'remove')
        .map(([id]) => parseInt(id, 10))

      if (addTags.length > 0 || removeTags.length > 0) {
        await api.bulkEdit({
          documents: selectedDocIds,
          method: 'modify_tags',
          parameters: { add_tags: addTags, remove_tags: removeTags },
        })
      }

      // 2. Correspondent
      if (selectedCorr !== 'keep') {
        const corrId = selectedCorr === 'clear' ? null : parseInt(selectedCorr, 10)
        await api.bulkEdit({
          documents: selectedDocIds,
          method: 'set_correspondent',
          parameters: { correspondent: corrId },
        })
      }

      // 3. Document Type
      if (selectedType !== 'keep') {
        const typeId = selectedType === 'clear' ? null : parseInt(selectedType, 10)
        await api.bulkEdit({
          documents: selectedDocIds,
          method: 'set_document_type',
          parameters: { document_type: typeId },
        })
      }

      // 4. Storage Path
      if (selectedPath !== 'keep') {
        const pathId = selectedPath === 'clear' ? null : parseInt(selectedPath, 10)
        await api.bulkEdit({
          documents: selectedDocIds,
          method: 'set_storage_path',
          parameters: { storage_path: pathId },
        })
      }

      refreshAll()
      onClose()
    } catch (err: any) {
      await alertModal({
        title: 'Bulk Edit Error',
        message: err.message || 'Operation failed',
        type: 'danger',
      })
    } finally {
      setSaving(false)
    }
  }

  const handleReprocess = async () => {
    const ok = await confirmModal({
      title: 'Reprocess Documents',
      message: `Trigger OCR & metadata reprocessing for ${selectedDocIds.length} documents?`,
      confirmText: 'Reprocess',
      type: 'warning',
    })
    if (!ok) return

    setSaving(true)
    try {
      await api.bulkEdit({
        documents: selectedDocIds,
        method: 'reprocess',
      })
      await alertModal({
        title: 'Reprocessing Queued',
        message: `Queued reprocessing for ${selectedDocIds.length} documents`,
        type: 'success',
      })
      refreshAll()
      onClose()
    } catch (err: any) {
      await alertModal({
        title: 'Reprocess Failed',
        message: err.message || 'Failed to trigger reprocessing',
        type: 'danger',
      })
    } finally {
      setSaving(false)
    }
  }

  const handleMergeAsVersions = async () => {
    if (selectedDocIds.length < 2) {
      await alertModal({
        title: 'Selection Required',
        message: 'Please select at least 2 documents to merge into versions',
        type: 'info',
      })
      return
    }
    const ok = await confirmModal({
      title: 'Merge as Versions',
      message: `Merge ${selectedDocIds.length} selected documents into version history of document #${selectedDocIds[0]}?`,
      confirmText: 'Merge Versions',
      type: 'warning',
    })
    if (!ok) return

    setSaving(true)
    try {
      await api.bulkEdit({
        documents: selectedDocIds,
        method: 'merge_as_versions',
      })
      setSelectedDocIds([])
      refreshAll()
      onClose()
    } catch (err: any) {
      await alertModal({
        title: 'Merge Failed',
        message: err.message || 'Failed to merge versions',
        type: 'danger',
      })
    } finally {
      setSaving(false)
    }
  }

  const filteredTags = tags.filter((t) => t.name.toLowerCase().includes(tagSearch.toLowerCase()))

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.75)',
        backdropFilter: 'blur(6px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 100,
        padding: '20px',
      }}
      onClick={onClose}
    >
      <div
        className="glass-panel animate-scale-up"
        style={{
          width: '100%',
          maxWidth: '680px',
          maxHeight: '90vh',
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
          boxShadow: '0 25px 50px -12px rgba(0, 0, 0, 0.6)',
          border: '1px solid var(--border-subtle)',
          backgroundColor: 'var(--bg-secondary)',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div
          style={{
            padding: '20px 24px',
            borderBottom: '1px solid var(--border-subtle)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
          }}
        >
          <div>
            <h2 style={{ fontSize: '1.25rem', fontWeight: 700, color: 'var(--text-primary)' }}>
              Bulk Edit Documents
            </h2>
            <p style={{ fontSize: '0.82rem', color: 'var(--text-muted)', marginTop: '2px' }}>
              Applying changes to <strong style={{ color: 'var(--accent-primary)' }}>{selectedDocIds.length}</strong> selected documents
            </p>
          </div>
          <button
            onClick={onClose}
            style={{
              background: 'transparent',
              border: 'none',
              color: 'var(--text-muted)',
              cursor: 'pointer',
              padding: '6px',
            }}
          >
            <X size={20} />
          </button>
        </div>

        {/* Body */}
        <div style={{ padding: '24px', overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: '20px' }}>
          {/* Tri-State Tags Modifier */}
          <div>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '8px' }}>
              <label style={{ fontSize: '0.85rem', fontWeight: 600, display: 'flex', alignItems: 'center', gap: '6px' }}>
                <TagIcon size={15} color="var(--accent-primary)" /> Modify Tags
              </label>
              <div style={{ fontSize: '0.72rem', color: 'var(--text-muted)' }}>
                Click tag to toggle: <span>Unchanged → </span><span style={{ color: '#10b981' }}>+Add → </span><span style={{ color: '#ef4444' }}>-Remove</span>
              </div>
            </div>

            <input
              type="text"
              placeholder="Search tags..."
              className="input"
              value={tagSearch}
              onChange={(e) => setTagSearch(e.target.value)}
              style={{ width: '100%', marginBottom: '10px', fontSize: '0.82rem' }}
            />

            <div
              style={{
                display: 'flex',
                flexWrap: 'wrap',
                gap: '8px',
                maxHeight: '140px',
                overflowY: 'auto',
                padding: '8px',
                backgroundColor: 'rgba(0,0,0,0.2)',
                borderRadius: 'var(--radius-sm)',
                border: '1px solid var(--border-subtle)',
              }}
            >
              {filteredTags.map((tag) => {
                const action = tagActions[tag.id] || 'none'
                let badgeBg = 'rgba(255,255,255,0.06)'
                let badgeColor = 'var(--text-secondary)'
                let icon = null

                if (action === 'add') {
                  badgeBg = 'rgba(16, 185, 129, 0.25)'
                  badgeColor = '#34d399'
                  icon = <Plus size={12} />
                } else if (action === 'remove') {
                  badgeBg = 'rgba(239, 68, 68, 0.25)'
                  badgeColor = '#f87171'
                  icon = <Minus size={12} />
                }

                return (
                  <button
                    key={tag.id}
                    onClick={() => toggleTagAction(tag.id)}
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: '5px',
                      padding: '4px 10px',
                      borderRadius: 'var(--radius-full)',
                      border: action === 'none' ? '1px solid rgba(255,255,255,0.08)' : `1px solid ${badgeColor}`,
                      backgroundColor: badgeBg,
                      color: badgeColor,
                      fontSize: '0.8rem',
                      fontWeight: 600,
                      cursor: 'pointer',
                      transition: 'all 0.15s ease',
                    }}
                  >
                    {icon}
                    <span>{tag.name}</span>
                  </button>
                )
              })}
            </div>
          </div>

          {/* Taxonomy Selectors */}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '16px' }}>
            {/* Correspondent */}
            <div>
              <label style={{ fontSize: '0.82rem', fontWeight: 600, display: 'flex', alignItems: 'center', gap: '6px', marginBottom: '6px' }}>
                <UserCheck size={14} color="var(--accent-primary)" /> Assign Correspondent
              </label>
              <select
                className="input"
                style={{ width: '100%', fontSize: '0.85rem' }}
                value={selectedCorr}
                onChange={(e) => setSelectedCorr(e.target.value)}
              >
                <option value="keep">[Do not change]</option>
                <option value="clear">[Remove correspondent]</option>
                {correspondents.map((c) => (
                  <option key={c.id} value={c.id}>{c.name}</option>
                ))}
              </select>
            </div>

            {/* Document Type */}
            <div>
              <label style={{ fontSize: '0.82rem', fontWeight: 600, display: 'flex', alignItems: 'center', gap: '6px', marginBottom: '6px' }}>
                <FolderTree size={14} color="var(--accent-primary)" /> Assign Document Type
              </label>
              <select
                className="input"
                style={{ width: '100%', fontSize: '0.85rem' }}
                value={selectedType}
                onChange={(e) => setSelectedType(e.target.value)}
              >
                <option value="keep">[Do not change]</option>
                <option value="clear">[Remove document type]</option>
                {documentTypes.map((t) => (
                  <option key={t.id} value={t.id}>{t.name}</option>
                ))}
              </select>
            </div>

            {/* Storage Path */}
            <div>
              <label style={{ fontSize: '0.82rem', fontWeight: 600, display: 'flex', alignItems: 'center', gap: '6px', marginBottom: '6px' }}>
                <HardDrive size={14} color="var(--accent-primary)" /> Assign Storage Path
              </label>
              <select
                className="input"
                style={{ width: '100%', fontSize: '0.85rem' }}
                value={selectedPath}
                onChange={(e) => setSelectedPath(e.target.value)}
              >
                <option value="keep">[Do not change]</option>
                <option value="clear">[Remove storage path]</option>
                {storagePaths.map((p) => (
                  <option key={p.id} value={p.id}>{p.name}</option>
                ))}
              </select>
            </div>
          </div>

          {/* Advanced Batch Operations */}
          <div style={{ borderTop: '1px solid var(--border-subtle)', paddingTop: '16px' }}>
            <div style={{ fontSize: '0.75rem', fontWeight: 700, textTransform: 'uppercase', color: 'var(--text-muted)', marginBottom: '10px' }}>
              Advanced Document Operations
            </div>
            <div style={{ display: 'flex', gap: '10px', flexWrap: 'wrap' }}>
              <button
                className="btn btn-secondary"
                style={{ fontSize: '0.8rem', gap: '6px' }}
                onClick={handleReprocess}
                disabled={saving}
              >
                <RefreshCw size={14} /> Re-do OCR & Reprocess
              </button>
              <button
                className="btn btn-secondary"
                style={{ fontSize: '0.8rem', gap: '6px' }}
                onClick={handleMergeAsVersions}
                disabled={saving}
              >
                <Layers size={14} /> Merge as Document Versions
              </button>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div
          style={{
            padding: '16px 24px',
            borderTop: '1px solid var(--border-subtle)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'flex-end',
            gap: '12px',
            backgroundColor: 'rgba(0,0,0,0.2)',
          }}
        >
          <button className="btn btn-secondary" onClick={onClose} disabled={saving}>
            Cancel
          </button>
          <button
            className="btn btn-primary"
            onClick={handleApply}
            disabled={saving}
            style={{ minWidth: '120px', justifyContent: 'center' }}
          >
            {saving ? 'Applying...' : 'Apply Changes'}
          </button>
        </div>
      </div>
    </div>
  )
}
