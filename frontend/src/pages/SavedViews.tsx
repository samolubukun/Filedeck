import React, { useState, useEffect } from 'react'
import { Eye, Plus, Trash2, Edit3, ArrowRight, X, Filter } from 'lucide-react'
import { api } from '../api/client'
import { useUiStore } from '../store/useUiStore'
import { useFilterStore } from '../store/useFilterStore'
import { useTags } from '../api/useTags'
import { useCorrespondents } from '../api/useCorrespondents'
import { useDocumentTypes } from '../api/useDocumentTypes'
import { confirmModal } from '../store/useModalStore'

interface FilterRule {
  rule_type: number
  value: string
}

interface SavedViewItem {
  id: number
  name: string
  icon: string
  sort_field?: string | null
  sort_reverse: boolean
  page_size?: number | null
  display_mode?: string | null
  display_fields?: any
  filter_rules: FilterRule[]
}

const RULE_TYPE_LABELS: Record<number, string> = {
  0: 'Title contains',
  1: 'Content contains',
  2: 'ASN is',
  3: 'Created before',
  4: 'Created after',
  12: 'Does not have tag',
  13: 'Has tag',
  16: 'Does not have correspondent',
  17: 'Has correspondent',
  18: 'Does not have document type',
  19: 'Has document type',
  22: 'Title or content contains',
}

export const SavedViews: React.FC = () => {
  const [views, setViews] = useState<SavedViewItem[]>([])
  const [loading, setLoading] = useState(false)
  const [isModalOpen, setIsModalOpen] = useState(false)
  const [editingId, setEditingId] = useState<number | null>(null)

  // Form states
  const [name, setName] = useState('')
  const [icon, setIcon] = useState('eye')
  const [sortField, setSortField] = useState('created')
  const [sortReverse, setSortReverse] = useState(true)
  const [displayMode, setDisplayMode] = useState('table')
  const [rules, setRules] = useState<FilterRule[]>([])

  const { tags } = useTags()
  const { correspondents } = useCorrespondents()
  const { documentTypes } = useDocumentTypes()
  const setCurrentNav = useUiStore((s) => s.setCurrentNav)
  const applySavedView = useFilterStore((s) => s.applySavedView)

  const fetchViews = async () => {
    setLoading(true)
    try {
      const res = await api.getSavedViews()
      setViews(res.results || [])
    } catch (err) {
      console.error(err)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchViews()
  }, [])

  const handleOpenCreate = () => {
    setEditingId(null)
    setName('')
    setIcon('eye')
    setSortField('created')
    setSortReverse(true)
    setDisplayMode('table')
    setRules([])
    setIsModalOpen(true)
  }

  const handleOpenEdit = (v: SavedViewItem) => {
    setEditingId(v.id)
    setName(v.name)
    setIcon(v.icon || 'eye')
    setSortField(v.sort_field || 'created')
    setSortReverse(v.sort_reverse)
    setDisplayMode(v.display_mode || 'table')
    setRules(v.filter_rules || [])
    setIsModalOpen(true)
  }

  const handleAddRule = () => {
    setRules([...rules, { rule_type: 0, value: '' }])
  }

  const handleRemoveRule = (index: number) => {
    setRules(rules.filter((_, i) => i !== index))
  }

  const handleRuleChange = (index: number, field: keyof FilterRule, val: any) => {
    setRules(
      rules.map((r, i) => {
        if (i === index) {
          return { ...r, [field]: val }
        }
        return r
      })
    )
  }

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!name.trim()) return

    const payload = {
      name: name.trim(),
      icon,
      sort_field: sortField,
      sort_reverse: sortReverse,
      display_mode: displayMode,
      filter_rules: rules,
    }

    try {
      if (editingId) {
        await api.updateSavedView(editingId, payload)
      } else {
        await api.createSavedView(payload)
      }
      setIsModalOpen(false)
      fetchViews()
    } catch (err) {
      console.error(err)
    }
  }

  const handleDelete = async (id: number) => {
    const ok = await confirmModal({
      title: 'Delete Saved View',
      message: 'Are you sure you want to delete this saved view?',
      confirmText: 'Delete View',
      type: 'danger',
    })
    if (!ok) return

    try {
      await api.deleteSavedView(id)
      fetchViews()
    } catch (err) {
      console.error(err)
    }
  }

  const handleApplyView = (v: SavedViewItem) => {
    applySavedView(v)
    setCurrentNav('documents')
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      {/* Header */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <h2 style={{ fontSize: '1.4rem', fontWeight: 700, display: 'flex', alignItems: 'center', gap: '10px' }}>
            <Eye size={22} style={{ color: 'var(--accent-primary)' }} />
            Saved Views
          </h2>
          <p style={{ color: 'var(--text-muted)', fontSize: '0.85rem', marginTop: '4px' }}>
            Configurable document queries with automated filter rules and display settings.
          </p>
        </div>
        <button className="btn btn-primary" onClick={handleOpenCreate}>
          <Plus size={16} /> New Saved View
        </button>
      </div>

      {/* Views Grid */}
      {loading ? (
        <div style={{ padding: '40px', textAlign: 'center', color: 'var(--text-muted)' }}>Loading saved views...</div>
      ) : views.length === 0 ? (
        <div
          className="card-panel"
          style={{
            padding: '48px',
            textAlign: 'center',
            color: 'var(--text-muted)',
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            gap: '12px',
            borderRadius: 'var(--radius-xl)',
          }}
        >
          <Filter size={32} style={{ opacity: 0.4 }} />
          <div style={{ color: 'var(--text-primary)', fontWeight: 600 }}>No saved views configured yet.</div>
          <button className="btn btn-secondary" onClick={handleOpenCreate}>
            Create your first view
          </button>
        </div>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(320px, 1fr))', gap: '18px' }}>
          {views.map((v) => (
            <div
              key={v.id}
              className="card-panel card-panel-interactive"
              style={{
                padding: '20px',
                borderRadius: 'var(--radius-lg)',
                display: 'flex',
                flexDirection: 'column',
                gap: '14px',
              }}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                  <div
                    style={{
                      width: '32px',
                      height: '32px',
                      borderRadius: 'var(--radius-sm)',
                      backgroundColor: 'var(--accent-soft)',
                      border: '1px solid var(--accent-border)',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      color: 'var(--accent-primary)',
                    }}
                  >
                    <Eye size={16} />
                  </div>
                  <div>
                    <h3 style={{ fontSize: '1rem', fontWeight: 600 }}>{v.name}</h3>
                    <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                      Sort: {v.sort_field || 'created'} ({v.sort_reverse ? 'Desc' : 'Asc'})
                    </span>
                  </div>
                </div>
                <div style={{ display: 'flex', gap: '6px' }}>
                  <button
                    onClick={() => handleOpenEdit(v)}
                    style={{
                      background: 'transparent',
                      border: 'none',
                      color: 'var(--text-muted)',
                      cursor: 'pointer',
                      padding: '4px',
                    }}
                  >
                    <Edit3 size={15} />
                  </button>
                  <button
                    onClick={() => handleDelete(v.id)}
                    style={{
                      background: 'transparent',
                      border: 'none',
                      color: 'var(--text-muted)',
                      cursor: 'pointer',
                      padding: '4px',
                    }}
                  >
                    <Trash2 size={15} />
                  </button>
                </div>
              </div>

              {/* Filter Rules List */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                <div style={{ fontSize: '0.72rem', fontWeight: 700, textTransform: 'uppercase', color: 'var(--text-muted)' }}>
                  Active Rules ({v.filter_rules.length})
                </div>
                {v.filter_rules.length === 0 ? (
                  <span style={{ fontSize: '0.8rem', color: 'var(--text-muted)', fontStyle: 'italic' }}>
                    Matches all documents
                  </span>
                ) : (
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px' }}>
                    {v.filter_rules.map((rule, idx) => (
                      <span
                        key={idx}
                        style={{
                          fontSize: '0.75rem',
                          padding: '3px 8px',
                          borderRadius: 'var(--radius-sm)',
                          backgroundColor: 'rgba(255,255,255,0.04)',
                          border: '1px solid var(--border-subtle)',
                        }}
                      >
                        {RULE_TYPE_LABELS[rule.rule_type] || `Rule ${rule.rule_type}`}: <strong>{rule.value || 'None'}</strong>
                      </span>
                    ))}
                  </div>
                )}
              </div>

              {/* Apply Button */}
              <button
                className="btn btn-secondary"
                onClick={() => handleApplyView(v)}
                style={{
                  marginTop: 'auto',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: '8px',
                  fontSize: '0.82rem',
                }}
              >
                Apply View <ArrowRight size={14} />
              </button>
            </div>
          ))}
        </div>
      )}

      {/* Create / Edit Modal */}
      {isModalOpen && (
        <div
          style={{
            position: 'fixed',
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            backgroundColor: 'rgba(0,0,0,0.7)',
            backdropFilter: 'blur(6px)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            zIndex: 9999,
          }}
          onClick={() => setIsModalOpen(false)}
        >
          <div
            className="card-panel"
            style={{
              width: '560px',
              maxWidth: '92vw',
              maxHeight: '85vh',
              overflowY: 'auto',
              padding: '28px',
              borderRadius: 'var(--radius-xl)',
              boxShadow: 'var(--shadow-lg)',
              display: 'flex',
              flexDirection: 'column',
              gap: '18px',
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <h3 style={{ fontSize: '1.2rem', fontWeight: 700 }}>
                {editingId ? 'Edit Saved View' : 'Create Saved View'}
              </h3>
              <button
                onClick={() => setIsModalOpen(false)}
                style={{ background: 'transparent', border: 'none', color: 'var(--text-muted)', cursor: 'pointer' }}
              >
                <X size={18} />
              </button>
            </div>

            <form onSubmit={handleSave} style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
              <div>
                <label style={{ fontSize: '0.85rem', fontWeight: 600, display: 'block', marginBottom: '6px' }}>
                  View Name
                </label>
                <input
                  type="text"
                  className="input"
                  required
                  placeholder="e.g. Invoices 2026, Contracts, Unprocessed"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                />
              </div>

              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
                <div>
                  <label style={{ fontSize: '0.85rem', fontWeight: 600, display: 'block', marginBottom: '6px' }}>
                    Sort Field
                  </label>
                  <select className="input" value={sortField} onChange={(e) => setSortField(e.target.value)}>
                    <option value="created">Date Created</option>
                    <option value="added">Date Ingested</option>
                    <option value="modified">Date Modified</option>
                    <option value="title">Title</option>
                    <option value="archive_serial_number">ASN</option>
                  </select>
                </div>
                <div>
                  <label style={{ fontSize: '0.85rem', fontWeight: 600, display: 'block', marginBottom: '6px' }}>
                    Sort Direction
                  </label>
                  <select
                    className="input"
                    value={sortReverse ? 'desc' : 'asc'}
                    onChange={(e) => setSortReverse(e.target.value === 'desc')}
                  >
                    <option value="desc">Descending (Newest first)</option>
                    <option value="asc">Ascending (Oldest first)</option>
                  </select>
                </div>
              </div>

              {/* Filter Rules Builder */}
              <div>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                  <label style={{ fontSize: '0.85rem', fontWeight: 600 }}>Filter Rules</label>
                  <button
                    type="button"
                    onClick={handleAddRule}
                    className="btn btn-secondary"
                    style={{ fontSize: '0.75rem', padding: '4px 8px', display: 'flex', alignItems: 'center', gap: '4px' }}
                  >
                    <Plus size={12} /> Add Rule
                  </button>
                </div>

                <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                  {rules.length === 0 ? (
                    <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)', fontStyle: 'italic', padding: '8px 0' }}>
                      No rules added yet. Matches all documents.
                    </div>
                  ) : (
                    rules.map((rule, idx) => (
                      <div key={idx} style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
                        <select
                          className="input"
                          style={{ width: '190px' }}
                          value={rule.rule_type}
                          onChange={(e) => handleRuleChange(idx, 'rule_type', parseInt(e.target.value, 10))}
                        >
                          <option value={0}>Title contains</option>
                          <option value={1}>Content contains</option>
                          <option value={22}>Title or content</option>
                          <option value={13}>Has tag</option>
                          <option value={12}>Does not have tag</option>
                          <option value={17}>Has correspondent</option>
                          <option value={16}>Does not have correspondent</option>
                          <option value={19}>Has document type</option>
                          <option value={18}>Does not have document type</option>
                          <option value={2}>ASN is</option>
                          <option value={4}>Created after</option>
                          <option value={3}>Created before</option>
                        </select>

                        {/* Value Input depends on rule type */}
                        {rule.rule_type === 13 || rule.rule_type === 12 ? (
                          <select
                            className="input"
                            style={{ flex: 1 }}
                            value={rule.value}
                            onChange={(e) => handleRuleChange(idx, 'value', e.target.value)}
                          >
                            <option value="">Select Tag...</option>
                            {tags.map((t) => (
                              <option key={t.id} value={t.id}>{t.name}</option>
                            ))}
                          </select>
                        ) : rule.rule_type === 17 || rule.rule_type === 16 ? (
                          <select
                            className="input"
                            style={{ flex: 1 }}
                            value={rule.value}
                            onChange={(e) => handleRuleChange(idx, 'value', e.target.value)}
                          >
                            <option value="">Select Correspondent...</option>
                            {correspondents.map((c) => (
                              <option key={c.id} value={c.id}>{c.name}</option>
                            ))}
                          </select>
                        ) : rule.rule_type === 19 || rule.rule_type === 18 ? (
                          <select
                            className="input"
                            style={{ flex: 1 }}
                            value={rule.value}
                            onChange={(e) => handleRuleChange(idx, 'value', e.target.value)}
                          >
                            <option value="">Select Document Type...</option>
                            {documentTypes.map((dt) => (
                              <option key={dt.id} value={dt.id}>{dt.name}</option>
                            ))}
                          </select>
                        ) : rule.rule_type === 3 || rule.rule_type === 4 ? (
                          <input
                            type="date"
                            className="input"
                            style={{ flex: 1 }}
                            value={rule.value}
                            onChange={(e) => handleRuleChange(idx, 'value', e.target.value)}
                          />
                        ) : (
                          <input
                            type="text"
                            className="input"
                            style={{ flex: 1 }}
                            placeholder="Value..."
                            value={rule.value}
                            onChange={(e) => handleRuleChange(idx, 'value', e.target.value)}
                          />
                        )}

                        <button
                          type="button"
                          onClick={() => handleRemoveRule(idx)}
                          style={{
                            background: 'transparent',
                            border: 'none',
                            color: 'var(--text-muted)',
                            cursor: 'pointer',
                            padding: '6px',
                          }}
                        >
                          <Trash2 size={16} />
                        </button>
                      </div>
                    ))
                  )}
                </div>
              </div>

              <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '10px', marginTop: '12px' }}>
                <button type="button" className="btn btn-secondary" onClick={() => setIsModalOpen(false)}>
                  Cancel
                </button>
                <button type="submit" className="btn btn-primary">
                  {editingId ? 'Save Changes' : 'Create View'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  )
}
