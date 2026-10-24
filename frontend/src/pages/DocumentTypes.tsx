import { useState } from 'react'
import { Plus } from 'lucide-react'
import {
  useDocumentTypes,
  useCreateDocumentType,
  useDeleteDocumentType,
} from '../api/useDocumentTypes'
import { AddCard, DeleteIconButton, PageHeading, fieldLabelStyle } from '../components/TaxonomyManager'
import { confirmModal } from '../store/useModalStore'

export function DocumentTypes() {
  const { documentTypes } = useDocumentTypes()
  const createType = useCreateDocumentType()
  const deleteType = useDeleteDocumentType()
  const [newName, setNewName] = useState('')

  const handleCreate = async () => {
    if (!newName.trim()) return
    await createType.mutateAsync({ name: newName.trim() })
    setNewName('')
  }

  const handleDelete = async (t: { id: number; name: string }) => {
    const ok = await confirmModal({
      title: 'Delete Document Type',
      message: `Are you sure you want to delete document type "${t.name}"?`,
      confirmText: 'Delete Type',
      type: 'danger',
    })
    if (ok) {
      deleteType.mutate(t.id)
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      <PageHeading
        title="Manage Document Types"
        subtitle="Classes of documents such as Invoice, Statement, Receipt, Medical Record."
      />

      <AddCard>
        <div style={{ flex: 1 }}>
          <label style={fieldLabelStyle}>Document Type Name</label>
          <input
            type="text"
            className="input"
            placeholder="e.g. Tax Form, Warranty, Invoice"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
          />
        </div>
        <button className="btn btn-primary" onClick={handleCreate}>
          <Plus size={16} /> Add Type
        </button>
      </AddCard>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(260px, 1fr))', gap: '16px' }}>
        {documentTypes.map((t) => (
          <div
            key={t.id}
            className="glass-panel"
            style={{ padding: '16px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}
          >
            <div>
              <div style={{ fontWeight: 700 }}>{t.name}</div>
              <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                {t.document_count || 0} document{t.document_count === 1 ? '' : 's'}
              </div>
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
