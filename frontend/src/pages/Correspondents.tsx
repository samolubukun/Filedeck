import { useState } from 'react'
import { Plus } from 'lucide-react'
import {
  useCorrespondents,
  useCreateCorrespondent,
  useDeleteCorrespondent,
} from '../api/useCorrespondents'
import { AddCard, DeleteIconButton, PageHeading, fieldLabelStyle } from '../components/TaxonomyManager'
import { confirmModal } from '../store/useModalStore'

export function Correspondents() {
  const { correspondents } = useCorrespondents()
  const createCorr = useCreateCorrespondent()
  const deleteCorr = useDeleteCorrespondent()
  const [newName, setNewName] = useState('')

  const handleCreate = async () => {
    if (!newName.trim()) return
    await createCorr.mutateAsync({ name: newName.trim() })
    setNewName('')
  }

  const handleDelete = async (corr: { id: number; name: string }) => {
    const ok = await confirmModal({
      title: 'Delete Correspondent',
      message: `Are you sure you want to delete correspondent "${corr.name}"?`,
      confirmText: 'Delete Correspondent',
      type: 'danger',
    })
    if (ok) {
      deleteCorr.mutate(corr.id)
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      <PageHeading
        title="Manage Correspondents"
        subtitle="Entities, institutions, companies, or people associated with documents."
      />

      <AddCard>
        <div style={{ flex: 1 }}>
          <label style={fieldLabelStyle}>Correspondent Name</label>
          <input
            type="text"
            className="input"
            placeholder="e.g. Bank of America, Amazon, Health Clinic"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
          />
        </div>
        <button className="btn btn-primary" onClick={handleCreate}>
          <Plus size={16} /> Add Correspondent
        </button>
      </AddCard>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(260px, 1fr))', gap: '16px' }}>
        {correspondents.map((c) => (
          <div
            key={c.id}
            className="glass-panel"
            style={{ padding: '16px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}
          >
            <div>
              <div style={{ fontWeight: 700 }}>{c.name}</div>
              <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                {c.document_count || 0} document{c.document_count === 1 ? '' : 's'}
              </div>
            </div>
            <DeleteIconButton
              onClick={() => handleDelete(c)}
            />
          </div>
        ))}
      </div>
    </div>
  )
}
