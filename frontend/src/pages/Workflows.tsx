import { useState, useEffect } from 'react'
import {
  GitBranch,
  Plus,
  Trash2,
  Edit2,
  X,
  ChevronDown,
  ChevronUp,
} from 'lucide-react'
import { api } from '../api/client'
import { useCorrespondents } from '../api/useCorrespondents'
import { useDocumentTypes } from '../api/useDocumentTypes'
import { confirmModal, alertModal } from '../store/useModalStore'

interface WorkflowTrigger {
  id?: number
  type: number // 1: Consumption, 2: Added, 3: Updated
  filter_filename?: string
  sources?: string
  filter_path?: string
  filter_has_tags?: string
}

interface WorkflowAction {
  id?: number
  type: number // 1: Assignment, 2: Removal, 3: Webhook
  assign_title?: string
  assign_tags?: string
  assign_document_type_id?: number
  assign_correspondent_id?: number
  assign_storage_path_id?: number
}

interface WorkflowItem {
  id: number
  name: string
  order: number
  enabled: boolean
  triggers?: WorkflowTrigger[]
  actions?: WorkflowAction[]
}

export function Workflows() {
  const [workflows, setWorkflows] = useState<WorkflowItem[]>([])
  const [loading, setLoading] = useState(true)
  const [editingWf, setEditingWf] = useState<WorkflowItem | null>(null)
  const [isModalOpen, setIsModalOpen] = useState(false)
  const [triggersOpen, setTriggersOpen] = useState(true)
  const [actionsOpen, setActionsOpen] = useState(true)

  const { correspondents } = useCorrespondents()
  const { documentTypes } = useDocumentTypes()

  const fetchWorkflows = async () => {
    setLoading(true)
    try {
      const res = await api.getWorkflows()
      setWorkflows(res.results || [])
    } catch (err) {
      console.error('Failed to load workflows', err)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchWorkflows()
  }, [])

  const handleCreateNew = () => {
    setEditingWf({
      id: 0,
      name: '',
      order: (workflows.length + 1),
      enabled: true,
      triggers: [
        {
          type: 1,
          filter_filename: '',
          sources: 'consume,api,web,mail',
          filter_path: '',
        },
      ],
      actions: [
        {
          type: 1,
          assign_title: '',
          assign_tags: '[]',
        },
      ],
    })
    setIsModalOpen(true)
  }

  const handleEdit = (wf: WorkflowItem) => {
    setEditingWf({
      ...wf,
      triggers: wf.triggers && wf.triggers.length > 0 ? wf.triggers : [
        { type: 1, filter_filename: '', sources: 'consume,api,web,mail' }
      ],
      actions: wf.actions && wf.actions.length > 0 ? wf.actions : [
        { type: 1, assign_title: '' }
      ],
    })
    setIsModalOpen(true)
  }

  const handleDelete = async (id: number) => {
    const ok = await confirmModal({
      title: 'Delete Workflow',
      message: 'Are you sure you want to delete this workflow?',
      confirmText: 'Delete Workflow',
      type: 'danger',
    })
    if (!ok) return

    try {
      await api.deleteWorkflow(id)
      fetchWorkflows()
    } catch (err: any) {
      await alertModal({
        title: 'Delete Failed',
        message: err.message || 'Failed to delete workflow',
        type: 'danger',
      })
    }
  }

  const handleToggleEnabled = async (wf: WorkflowItem) => {
    try {
      await api.updateWorkflow(wf.id, { enabled: !wf.enabled })
      fetchWorkflows()
    } catch (err: any) {
      await alertModal({
        title: 'Toggle Failed',
        message: err.message || 'Failed to update workflow status',
        type: 'danger',
      })
    }
  }

  const handleSaveModal = async () => {
    if (!editingWf) return
    if (!editingWf.name.trim()) {
      await alertModal({
        title: 'Validation',
        message: 'Please provide a workflow name',
        type: 'warning',
      })
      return
    }

    try {
      if (editingWf.id === 0) {
        await api.createWorkflow({
          name: editingWf.name,
          order: editingWf.order,
          enabled: editingWf.enabled,
        })
      } else {
        await api.updateWorkflow(editingWf.id, {
          name: editingWf.name,
          order: editingWf.order,
          enabled: editingWf.enabled,
        })
      }
      setIsModalOpen(false)
      fetchWorkflows()
    } catch (err: any) {
      await alertModal({
        title: 'Save Failed',
        message: err.message || 'Failed to save workflow',
        type: 'danger',
      })
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      {/* Page Header */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div>
          <h2 style={{ fontSize: '1.5rem', fontWeight: 700, color: 'var(--text-primary)', display: 'flex', alignItems: 'center', gap: '10px' }}>
            <GitBranch size={24} color="var(--accent-primary)" /> Workflows
          </h2>
          <p style={{ fontSize: '0.85rem', color: 'var(--text-muted)', marginTop: '4px' }}>
            Automated processing rules executed on document ingestion and lifecycle events
          </p>
        </div>

        <button
          className="btn btn-primary"
          style={{ gap: '6px', padding: '9px 16px', fontSize: '0.875rem' }}
          onClick={handleCreateNew}
        >
          <Plus size={16} /> Add Workflow
        </button>
      </div>

      {/* Workflows List */}
      {loading ? (
        <div className="glass-panel" style={{ padding: '36px', textAlign: 'center', color: 'var(--text-muted)' }}>
          Loading workflows...
        </div>
      ) : workflows.length === 0 ? (
        <div
          className="card-panel"
          style={{
            padding: '48px 24px',
            textAlign: 'center',
            color: 'var(--text-muted)',
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            gap: '12px',
            borderRadius: 'var(--radius-xl)',
          }}
        >
          <GitBranch size={42} style={{ opacity: 0.3, color: 'var(--accent-primary)' }} />
          <div style={{ fontSize: '1.1rem', fontWeight: 700, color: 'var(--text-primary)' }}>
            No workflows configured
          </div>
          <div style={{ fontSize: '0.85rem', maxWidth: '420px', lineHeight: 1.5, color: 'var(--text-secondary)' }}>
            Create automated rules to auto-assign tags, correspondents, and document types when documents are ingested.
          </div>
          <button className="btn btn-primary" onClick={handleCreateNew} style={{ marginTop: '8px' }}>
            <Plus size={15} /> Create your first workflow
          </button>
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
          {workflows.map((wf) => (
            <div
              key={wf.id}
              className="card-panel"
              style={{
                padding: '20px 24px',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                gap: '16px',
                borderRadius: 'var(--radius-lg)',
                borderLeft: wf.enabled ? '4px solid var(--accent-primary)' : '4px solid var(--border-subtle)',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
                <span
                  style={{
                    fontSize: '0.75rem',
                    fontWeight: 700,
                    color: 'var(--text-muted)',
                    backgroundColor: 'rgba(255,255,255,0.06)',
                    padding: '3px 8px',
                    borderRadius: '4px',
                  }}
                  title="Execution Order"
                >
                  #{wf.order}
                </span>

                <div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                    <span style={{ fontSize: '1.05rem', fontWeight: 700, color: 'var(--text-primary)' }}>
                      {wf.name}
                    </span>
                    <span
                      style={{
                        fontSize: '0.68rem',
                        fontWeight: 700,
                        padding: '1px 6px',
                        borderRadius: '4px',
                        backgroundColor: wf.enabled ? 'rgba(16, 185, 129, 0.2)' : 'rgba(255,255,255,0.08)',
                        color: wf.enabled ? '#34d399' : 'var(--text-muted)',
                      }}
                    >
                      {wf.enabled ? 'ENABLED' : 'DISABLED'}
                    </span>
                  </div>
                  <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)', marginTop: '4px' }}>
                    Triggers on ingest & updates · Auto-assigns metadata
                  </div>
                </div>
              </div>

              <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                <button
                  className="btn btn-secondary"
                  style={{ padding: '6px 12px', fontSize: '0.8rem', gap: '6px' }}
                  onClick={() => handleToggleEnabled(wf)}
                >
                  {wf.enabled ? 'Disable' : 'Enable'}
                </button>

                <button
                  className="btn btn-secondary"
                  style={{ padding: '6px 10px' }}
                  onClick={() => handleEdit(wf)}
                  title="Edit workflow"
                >
                  <Edit2 size={15} />
                </button>

                <button
                  className="btn btn-danger"
                  style={{ padding: '6px 10px' }}
                  onClick={() => handleDelete(wf.id)}
                  title="Delete workflow"
                >
                  <Trash2 size={15} />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Edit / Create Workflow Modal */}
      {isModalOpen && editingWf && (
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
          onClick={() => setIsModalOpen(false)}
        >
          <div
            className="card-panel animate-scale-up"
            style={{
              width: '100%',
              maxWidth: '720px',
              maxHeight: '90vh',
              display: 'flex',
              flexDirection: 'column',
              overflow: 'hidden',
              backgroundColor: 'var(--bg-secondary)',
              border: '1px solid var(--border-subtle)',
              borderRadius: 'var(--radius-xl)',
              boxShadow: 'var(--shadow-lg)',
            }}
            onClick={(e) => e.stopPropagation()}
          >
            {/* Modal Header */}
            <div
              style={{
                padding: '20px 24px',
                borderBottom: '1px solid var(--border-subtle)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                <h3 style={{ fontSize: '1.2rem', fontWeight: 700, color: 'var(--text-primary)' }}>
                  {editingWf.id === 0 ? 'Create Workflow' : `Edit Workflow #${editingWf.id}`}
                </h3>
              </div>
              <button
                onClick={() => setIsModalOpen(false)}
                style={{ background: 'transparent', border: 'none', color: 'var(--text-muted)', cursor: 'pointer' }}
              >
                <X size={20} />
              </button>
            </div>

            {/* Modal Form */}
            <div style={{ padding: '24px', overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: '20px' }}>
              {/* Basic Info */}
              <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr 1fr', gap: '16px', alignItems: 'flex-end' }}>
                <div>
                  <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: 'var(--text-secondary)', marginBottom: '6px' }}>
                    Workflow Name
                  </label>
                  <input
                    type="text"
                    className="input"
                    placeholder="e.g. Tag utility bills"
                    value={editingWf.name}
                    onChange={(e) => setEditingWf({ ...editingWf, name: e.target.value })}
                  />
                </div>

                <div>
                  <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: 'var(--text-secondary)', marginBottom: '6px' }}>
                    Sort Order
                  </label>
                  <input
                    type="number"
                    className="input"
                    value={editingWf.order}
                    onChange={(e) => setEditingWf({ ...editingWf, order: parseInt(e.target.value, 10) || 1 })}
                  />
                </div>

                <div style={{ paddingBottom: '8px' }}>
                  <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer', fontSize: '0.85rem', fontWeight: 600 }}>
                    <input
                      type="checkbox"
                      checked={editingWf.enabled}
                      onChange={(e) => setEditingWf({ ...editingWf, enabled: e.target.checked })}
                      style={{ width: '16px', height: '16px' }}
                    />
                    <span>Enabled</span>
                  </label>
                </div>
              </div>

              {/* Triggers Section */}
              <div
                style={{
                  border: '1px solid var(--border-subtle)',
                  borderRadius: 'var(--radius-sm)',
                  backgroundColor: 'rgba(0,0,0,0.15)',
                  overflow: 'hidden',
                }}
              >
                <div
                  style={{
                    padding: '12px 18px',
                    backgroundColor: 'rgba(255,255,255,0.03)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    cursor: 'pointer',
                  }}
                  onClick={() => setTriggersOpen(!triggersOpen)}
                >
                  <span style={{ fontWeight: 700, fontSize: '0.85rem', color: 'var(--text-primary)' }}>
                    Triggers (When to run)
                  </span>
                  {triggersOpen ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
                </div>

                {triggersOpen && (
                  <div style={{ padding: '16px', display: 'flex', flexDirection: 'column', gap: '14px' }}>
                    {editingWf.triggers?.map((tr, idx) => (
                      <div
                        key={idx}
                        style={{
                          padding: '14px',
                          borderRadius: 'var(--radius-sm)',
                          backgroundColor: 'rgba(255,255,255,0.02)',
                          border: '1px solid rgba(255,255,255,0.06)',
                          display: 'flex',
                          flexDirection: 'column',
                          gap: '12px',
                        }}
                      >
                        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
                          <div>
                            <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: 600, color: 'var(--text-muted)', marginBottom: '4px' }}>
                              Trigger Type
                            </label>
                            <select
                              className="input"
                              value={tr.type}
                              onChange={(e) => {
                                const next = [...(editingWf.triggers || [])]
                                next[idx].type = parseInt(e.target.value, 10)
                                setEditingWf({ ...editingWf, triggers: next })
                              }}
                            >
                              <option value={1}>Consumption Started</option>
                              <option value={2}>Document Added</option>
                              <option value={3}>Document Updated</option>
                            </select>
                          </div>

                          <div>
                            <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: 600, color: 'var(--text-muted)', marginBottom: '4px' }}>
                              Filter Filename (Wildcard e.g. *bill*)
                            </label>
                            <input
                              type="text"
                              className="input"
                              placeholder="*bill*, invoice_*.pdf"
                              value={tr.filter_filename || ''}
                              onChange={(e) => {
                                const next = [...(editingWf.triggers || [])]
                                next[idx].filter_filename = e.target.value
                                setEditingWf({ ...editingWf, triggers: next })
                              }}
                            />
                          </div>
                        </div>

                        <div>
                          <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: 600, color: 'var(--text-muted)', marginBottom: '4px' }}>
                            Filter Sources
                          </label>
                          <input
                            type="text"
                            className="input"
                            placeholder="consume, api, web, mail"
                            value={tr.sources || ''}
                            onChange={(e) => {
                              const next = [...(editingWf.triggers || [])]
                              next[idx].sources = e.target.value
                              setEditingWf({ ...editingWf, triggers: next })
                            }}
                          />
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {/* Actions Section */}
              <div
                style={{
                  border: '1px solid var(--border-subtle)',
                  borderRadius: 'var(--radius-sm)',
                  backgroundColor: 'rgba(0,0,0,0.15)',
                  overflow: 'hidden',
                }}
              >
                <div
                  style={{
                    padding: '12px 18px',
                    backgroundColor: 'rgba(255,255,255,0.03)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    cursor: 'pointer',
                  }}
                  onClick={() => setActionsOpen(!actionsOpen)}
                >
                  <span style={{ fontWeight: 700, fontSize: '0.85rem', color: 'var(--text-primary)' }}>
                    Actions (What to do)
                  </span>
                  {actionsOpen ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
                </div>

                {actionsOpen && (
                  <div style={{ padding: '16px', display: 'flex', flexDirection: 'column', gap: '14px' }}>
                    {editingWf.actions?.map((act, idx) => (
                      <div
                        key={idx}
                        style={{
                          padding: '14px',
                          borderRadius: 'var(--radius-sm)',
                          backgroundColor: 'rgba(255,255,255,0.02)',
                          border: '1px solid rgba(255,255,255,0.06)',
                          display: 'flex',
                          flexDirection: 'column',
                          gap: '12px',
                        }}
                      >
                        <div>
                          <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: 600, color: 'var(--text-muted)', marginBottom: '4px' }}>
                            Action Type
                          </label>
                          <select
                            className="input"
                            value={act.type}
                            onChange={(e) => {
                              const next = [...(editingWf.actions || [])]
                              next[idx].type = parseInt(e.target.value, 10)
                              setEditingWf({ ...editingWf, actions: next })
                            }}
                          >
                            <option value={1}>Assignment (Set Metadata)</option>
                            <option value={2}>Removal (Remove Tags)</option>
                            <option value={3}>Webhook Notification</option>
                          </select>
                        </div>

                        <div>
                          <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: 600, color: 'var(--text-muted)', marginBottom: '4px' }}>
                            Assign Title Template (e.g. {`Invoice_{correspondent}_{created_year}`})
                          </label>
                          <input
                            type="text"
                            className="input"
                            placeholder="Leave empty to keep original title"
                            value={act.assign_title || ''}
                            onChange={(e) => {
                              const next = [...(editingWf.actions || [])]
                              next[idx].assign_title = e.target.value
                              setEditingWf({ ...editingWf, actions: next })
                            }}
                          />
                        </div>

                        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
                          <div>
                            <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: 600, color: 'var(--text-muted)', marginBottom: '4px' }}>
                              Assign Correspondent
                            </label>
                            <select
                              className="input"
                              value={act.assign_correspondent_id || ''}
                              onChange={(e) => {
                                const next = [...(editingWf.actions || [])]
                                next[idx].assign_correspondent_id = e.target.value ? parseInt(e.target.value, 10) : undefined
                                setEditingWf({ ...editingWf, actions: next })
                              }}
                            >
                              <option value="">(None)</option>
                              {correspondents.map((c: any) => (
                                <option key={c.id} value={c.id}>{c.name}</option>
                              ))}
                            </select>
                          </div>

                          <div>
                            <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: 600, color: 'var(--text-muted)', marginBottom: '4px' }}>
                              Assign Document Type
                            </label>
                            <select
                              className="input"
                              value={act.assign_document_type_id || ''}
                              onChange={(e) => {
                                const next = [...(editingWf.actions || [])]
                                next[idx].assign_document_type_id = e.target.value ? parseInt(e.target.value, 10) : undefined
                                setEditingWf({ ...editingWf, actions: next })
                              }}
                            >
                              <option value="">(None)</option>
                              {documentTypes.map((t: any) => (
                                <option key={t.id} value={t.id}>{t.name}</option>
                              ))}
                            </select>
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>

            {/* Modal Footer */}
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
              <button className="btn btn-secondary" onClick={() => setIsModalOpen(false)}>
                Cancel
              </button>
              <button className="btn btn-primary" onClick={handleSaveModal} style={{ minWidth: '100px', justifyContent: 'center' }}>
                Save Workflow
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
