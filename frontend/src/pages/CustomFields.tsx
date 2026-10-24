import { useState, useEffect } from 'react'
import { Hash, Plus, Calendar, Link, CheckSquare, DollarSign, Type } from 'lucide-react'
import { api } from '../api/client'
import { AddCard, DeleteIconButton, PageHeading, fieldLabelStyle } from '../components/TaxonomyManager'
import { confirmModal, alertModal } from '../store/useModalStore'
import type { CustomFieldDefinition } from '../types'

const DATA_TYPES = [
  { value: 'string', label: 'String (Text)' },
  { value: 'url', label: 'URL / Link' },
  { value: 'date', label: 'Date' },
  { value: 'boolean', label: 'Boolean (Yes/No)' },
  { value: 'integer', label: 'Integer' },
  { value: 'float', label: 'Float / Decimal' },
  { value: 'monetary', label: 'Monetary Currency' },
  { value: 'documentlink', label: 'Document Link' },
]

export function CustomFields() {
  const [fields, setFields] = useState<CustomFieldDefinition[]>([])
  const [loading, setLoading] = useState(true)
  const [name, setName] = useState('')
  const [dataType, setDataType] = useState('string')
  const [isSubmitting, setIsSubmitting] = useState(false)

  const loadFields = async () => {
    setLoading(true)
    try {
      const data = await api.getCustomFields()
      setFields(data.results || [])
    } catch (err) {
      console.error(err)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadFields()
  }, [])

  const handleCreate = async () => {
    if (!name.trim()) return
    setIsSubmitting(true)
    try {
      await api.createCustomField({
        name: name.trim(),
        data_type: dataType,
      })
      setName('')
      setDataType('string')
      await loadFields()
    } catch (err: any) {
      await alertModal({
        title: 'Creation Failed',
        message: err.message || 'Failed to create custom field',
        type: 'danger',
      })
    } finally {
      setIsSubmitting(false)
    }
  }

  const handleDelete = async (id: number) => {
    const ok = await confirmModal({
      title: 'Delete Custom Field',
      message: 'Are you sure you want to delete this custom field? Values on existing documents will be removed.',
      confirmText: 'Delete Field',
      type: 'danger',
    })
    if (!ok) return

    try {
      await api.deleteCustomField(id)
      await loadFields()
    } catch (err: any) {
      await alertModal({
        title: 'Delete Failed',
        message: err.message || 'Failed to delete custom field',
        type: 'danger',
      })
    }
  }

  const getDataTypeIcon = (type: string) => {
    switch (type.toLowerCase()) {
      case 'date':
        return <Calendar size={14} />
      case 'url':
        return <Link size={14} />
      case 'boolean':
        return <CheckSquare size={14} />
      case 'monetary':
        return <DollarSign size={14} />
      case 'integer':
      case 'float':
        return <Hash size={14} />
      default:
        return <Type size={14} />
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      <PageHeading
        title="Custom Fields"
        subtitle="Attach structured metadata fields to documents like Invoice Totals, Account IDs, or Expiry Dates."
      />

      <AddCard>
        <div style={{ flex: 1 }}>
          <label style={fieldLabelStyle}>Field Name</label>
          <input
            type="text"
            className="input"
            placeholder="e.g. Invoice Amount, VAT ID, Policy Number"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </div>
        <div style={{ width: '220px' }}>
          <label style={fieldLabelStyle}>Field Data Type</label>
          <select
            className="input"
            value={dataType}
            onChange={(e) => setDataType(e.target.value)}
          >
            {DATA_TYPES.map((dt) => (
              <option key={dt.value} value={dt.value}>
                {dt.label}
              </option>
            ))}
          </select>
        </div>
        <button
          className="btn btn-primary"
          onClick={handleCreate}
          disabled={isSubmitting || !name.trim()}
          style={{ height: '38px', alignSelf: 'flex-end' }}
        >
          <Plus size={16} /> Add Field
        </button>
      </AddCard>

      {loading ? (
        <div style={{ padding: '40px', textAlign: 'center', color: 'var(--text-muted)' }}>
          Loading custom fields...
        </div>
      ) : fields.length === 0 ? (
        <div
          className="glass-panel"
          style={{ padding: '48px', textAlign: 'center', color: 'var(--text-muted)' }}
        >
          No custom fields configured yet. Create fields above to attach custom metadata to documents.
        </div>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(260px, 1fr))', gap: '16px' }}>
          {fields.map((field) => (
            <div
              key={field.id}
              className="glass-panel"
              style={{
                padding: '16px 18px',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
              }}
            >
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <span style={{ fontWeight: 700, fontSize: '0.95rem', color: 'var(--text-primary)' }}>
                  {field.name}
                </span>
                <span
                  style={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: '4px',
                    fontSize: '0.75rem',
                    color: 'var(--accent-secondary, #38bdf8)',
                    textTransform: 'uppercase',
                    fontWeight: 600,
                  }}
                >
                  {getDataTypeIcon(field.data_type)}
                  {field.data_type}
                </span>
              </div>
              <DeleteIconButton onClick={() => handleDelete(field.id)} />
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
