import { useState } from 'react'
import { Plus } from 'lucide-react'
import { useTags, useCreateTag, useDeleteTag } from '../api/useTags'
import { AddCard, DeleteIconButton, PageHeading, fieldLabelStyle } from '../components/TaxonomyManager'
import { confirmModal } from '../store/useModalStore'

export function Tags() {
  const { tags } = useTags()
  const createTag = useCreateTag()
  const deleteTag = useDeleteTag()
  const [newTagName, setNewTagName] = useState('')
  const [newTagColor, setNewTagColor] = useState('#2563eb')

  const handleCreate = async () => {
    if (!newTagName.trim()) return
    await createTag.mutateAsync({ name: newTagName.trim(), color: newTagColor })
    setNewTagName('')
  }

  const handleDelete = async (tag: { id: number; name: string }) => {
    const ok = await confirmModal({
      title: 'Delete Tag',
      message: `Are you sure you want to delete tag "${tag.name}"? It will be removed from all documents.`,
      confirmText: 'Delete Tag',
      type: 'danger',
    })
    if (ok) {
      deleteTag.mutate(tag.id)
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      <PageHeading title="Manage Tags" subtitle="Organize and categorize documents with tags." />

      <AddCard>
        <div style={{ flex: 1 }}>
          <label style={fieldLabelStyle}>Tag Name</label>
          <input
            type="text"
            className="input"
            placeholder="e.g. Invoices, Receipts, Contracts"
            value={newTagName}
            onChange={(e) => setNewTagName(e.target.value)}
          />
        </div>
        <div>
          <label style={fieldLabelStyle}>Badge Color</label>
          <input
            type="color"
            style={{ height: '38px', width: '60px', borderRadius: 'var(--radius-sm)', border: 'none', cursor: 'pointer', background: 'transparent' }}
            value={newTagColor}
            onChange={(e) => setNewTagColor(e.target.value)}
          />
        </div>
        <button className="btn btn-primary" onClick={handleCreate}>
          <Plus size={16} /> Add Tag
        </button>
      </AddCard>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(220px, 1fr))', gap: '16px' }}>
        {tags.map((t) => (
          <div
            key={t.id}
            className="glass-panel"
            style={{ padding: '16px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <span style={{ width: '12px', height: '12px', borderRadius: '50%', backgroundColor: t.color || 'var(--accent-primary)' }} />
              <span style={{ fontWeight: 600 }}>{t.name}</span>
            </div>
            <DeleteIconButton
              onClick={() => handleDelete(t)}
            />
          </div>
        ))}
      </div>
    </div>
  )
}
