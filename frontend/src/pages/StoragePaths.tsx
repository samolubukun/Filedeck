import { useState, useEffect } from 'react'
import { FolderTree, Plus, Check, Folder, FileText, ChevronRight, Sparkles, Settings2 } from 'lucide-react'
import { api } from '../api/client'
import { DeleteIconButton, PageHeading, fieldLabelStyle } from '../components/TaxonomyManager'
import { confirmModal, alertModal } from '../store/useModalStore'
import type { TaxonomyItem } from '../types'

interface PresetOption {
  id: string
  label: string
  nameSuggestion: string
  template: string
  preview: string
}

const PRESETS: PresetOption[] = [
  {
    id: 'year_sender_title',
    label: 'Year / Sender / Title',
    nameSuggestion: 'Organize by Year & Sender',
    template: '{created_year}/{correspondent}/{title}',
    preview: '2026 / Acme Energy / October Bill.pdf',
  },
  {
    id: 'sender_year_title',
    label: 'Sender / Year / Title',
    nameSuggestion: 'Organize by Sender & Year',
    template: '{correspondent}/{created_year}/{title}',
    preview: 'Acme Energy / 2026 / October Bill.pdf',
  },
  {
    id: 'type_year_title',
    label: 'Doc Type / Year / Title',
    nameSuggestion: 'Organize by Document Type',
    template: '{document_type}/{created_year}/{title}',
    preview: 'Invoices / 2026 / October Bill.pdf',
  },
  {
    id: 'year_title',
    label: 'Year / Title (Simple)',
    nameSuggestion: 'Simple Year Folders',
    template: '{created_year}/{title}',
    preview: '2026 / October Bill.pdf',
  },
]

const TOKENS = [
  { key: '{created_year}', label: 'Year', example: '2026', icon: '📅' },
  { key: '{correspondent}', label: 'Sender / Vendor', example: 'Acme Corp', icon: '🏢' },
  { key: '{document_type}', label: 'Doc Type', example: 'Invoice', icon: '📑' },
  { key: '{created_month}', label: 'Month', example: '10', icon: '🗓️' },
  { key: '{title}', label: 'Doc Title', example: 'Power Bill', icon: '📄' },
  { key: '{asn}', label: 'Archive #', example: '00042', icon: '🔢' },
]

export function StoragePaths() {
  const [paths, setPaths] = useState<TaxonomyItem[]>([])
  const [loading, setLoading] = useState(true)
  const [name, setName] = useState('')
  const [pathTemplate, setPathTemplate] = useState(PRESETS[0].template)
  const [activePreset, setActivePreset] = useState<string>(PRESETS[0].id)
  const [showAdvanced, setShowAdvanced] = useState(false)
  const [matchPattern, setMatchPattern] = useState('')
  const [algorithm, setAlgorithm] = useState<number>(0)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [successMsg, setSuccessMsg] = useState('')

  const loadPaths = async () => {
    setLoading(true)
    try {
      const data = await api.getStoragePaths()
      setPaths(data.results || [])
    } catch (err) {
      console.error(err)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadPaths()
  }, [])

  const handleSelectPreset = (preset: PresetOption) => {
    setActivePreset(preset.id)
    setPathTemplate(preset.template)
    if (!name || PRESETS.some((p) => p.nameSuggestion === name)) {
      setName(preset.nameSuggestion)
    }
  }

  const handleInsertToken = (tokenKey: string) => {
    setActivePreset('custom')
    setPathTemplate((prev) => {
      if (!prev || !prev.trim()) return tokenKey
      if (prev.endsWith('/')) return `${prev}${tokenKey}`
      return `${prev}/${tokenKey}`
    })
  }

  const handleCreate = async (e?: React.FormEvent) => {
    if (e) e.preventDefault()
    const finalName = name.trim() || 'My Storage Path'
    const finalTemplate = pathTemplate.trim() || PRESETS[0].template

    setIsSubmitting(true)
    try {
      await api.createStoragePath({
        name: finalName,
        path: finalTemplate,
        match: matchPattern.trim() || undefined,
        matching_algorithm: algorithm,
      })
      setName('')
      setPathTemplate(PRESETS[0].template)
      setActivePreset(PRESETS[0].id)
      setMatchPattern('')
      setAlgorithm(0)
      setShowAdvanced(false)
      setSuccessMsg('Storage path created successfully!')
      setTimeout(() => setSuccessMsg(''), 3000)
      await loadPaths()
    } catch (err: any) {
      const msg = err?.response?.data?.message || err?.message || 'Failed to create storage path'
      await alertModal({
        title: 'Error Creating Path',
        message: msg,
        type: 'danger',
      })
    } finally {
      setIsSubmitting(false)
    }
  }

  const handleDelete = async (id: number) => {
    const ok = await confirmModal({
      title: 'Delete Storage Path',
      message: 'Are you sure you want to delete this storage path?',
      confirmText: 'Delete Path',
      type: 'danger',
    })
    if (!ok) return

    try {
      await api.deleteStoragePath(id)
      await loadPaths()
    } catch (err: any) {
      await alertModal({
        title: 'Delete Failed',
        message: 'Failed to delete storage path',
        type: 'danger',
      })
    }
  }

  // Generate realistic breadcrumbs preview from template
  const getSimulatedPreview = (templateStr: string) => {
    const segments = templateStr.split('/').filter(Boolean)
    if (segments.length === 0) return ['Documents', 'Sample_Document.pdf']
    return segments.map((seg, idx) => {
      let text = seg
        .replace('{created_year}', '2026')
        .replace('{created_month}', '10')
        .replace('{created}', '2026-10-24')
        .replace('{correspondent}', 'Acme Energy')
        .replace('{document_type}', 'Invoice')
        .replace('{title}', 'October Power Bill')
        .replace('{asn}', '00128')
      if (idx === segments.length - 1 && !text.includes('.')) {
        text += '.pdf'
      }
      return text
    })
  }

  const previewSegments = getSimulatedPreview(pathTemplate)

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      <PageHeading
        title="Storage Paths"
        subtitle="Control how your documents are named and sorted into actual folders on your hard drive."
      />

      {/* Main Creation Builder */}
      <form onSubmit={handleCreate}>
        <div
          className="card-panel"
          style={{
            padding: '24px',
            borderRadius: 'var(--radius-xl)',
            display: 'flex',
            flexDirection: 'column',
            gap: '20px',
            boxShadow: 'var(--shadow-md)',
          }}
        >
          {/* Top Title / Presets Selector */}
          <div>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '12px' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <Sparkles size={18} style={{ color: 'var(--accent-primary)' }} />
                <span style={{ fontSize: '0.95rem', fontWeight: 700, color: 'var(--text-primary)' }}>
                  Choose a Folder Layout
                </span>
              </div>
              <button
                type="button"
                onClick={() => setShowAdvanced(!showAdvanced)}
                style={{
                  background: 'none',
                  border: 'none',
                  color: 'var(--text-muted)',
                  fontSize: '0.8rem',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '5px',
                  cursor: 'pointer',
                  padding: '4px 8px',
                  borderRadius: '6px',
                }}
              >
                <Settings2 size={14} />
                {showAdvanced ? 'Hide Advanced Editor' : 'Advanced Editor'}
              </button>
            </div>

            {/* Quick Preset Buttons */}
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '10px' }}>
              {PRESETS.map((preset) => {
                const isSelected = activePreset === preset.id
                return (
                  <div
                    key={preset.id}
                    onClick={() => handleSelectPreset(preset)}
                    style={{
                      padding: '12px 14px',
                      borderRadius: 'var(--radius-md)',
                      border: isSelected ? '2px solid var(--accent-primary)' : '1px solid var(--border-subtle)',
                      backgroundColor: isSelected ? 'var(--accent-soft)' : 'var(--bg-tertiary)',
                      cursor: 'pointer',
                      transition: 'all 0.15s ease',
                      display: 'flex',
                      flexDirection: 'column',
                      gap: '4px',
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                      <span
                        style={{
                          fontSize: '0.86rem',
                          fontWeight: isSelected ? 700 : 600,
                          color: isSelected ? 'var(--accent-primary)' : 'var(--text-primary)',
                        }}
                      >
                        {preset.label}
                      </span>
                      {isSelected && <Check size={15} style={{ color: 'var(--accent-primary)' }} />}
                    </div>
                    <span style={{ fontSize: '0.74rem', color: 'var(--text-muted)', fontFamily: 'var(--font-mono, monospace)' }}>
                      {preset.preview}
                    </span>
                  </div>
                )
              })}
            </div>
          </div>

          {/* Clickable Building Block Chips */}
          <div>
            <div style={{ ...fieldLabelStyle, marginBottom: '8px' }}>
              Click building blocks to add folders or filename elements:
            </div>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px' }}>
              {TOKENS.map((token) => (
                <button
                  type="button"
                  key={token.key}
                  onClick={() => handleInsertToken(token.key)}
                  title={`Click to add ${token.label} (${token.example})`}
                  style={{
                    backgroundColor: 'var(--bg-surface)',
                    color: 'var(--text-primary)',
                    border: '1px solid var(--border-subtle)',
                    padding: '6px 12px',
                    borderRadius: 'var(--radius-full)',
                    fontSize: '0.8rem',
                    fontWeight: 600,
                    cursor: 'pointer',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '6px',
                    transition: 'all 0.15s ease',
                    boxShadow: 'var(--shadow-sm)',
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.borderColor = 'var(--accent-primary)'
                    e.currentTarget.style.color = 'var(--accent-primary)'
                    e.currentTarget.style.transform = 'translateY(-1px)'
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.borderColor = 'var(--border-subtle)'
                    e.currentTarget.style.color = 'var(--text-primary)'
                    e.currentTarget.style.transform = 'translateY(0)'
                  }}
                >
                  <span>{token.icon}</span>
                  <span>{token.label}</span>
                  <span style={{ fontSize: '0.72rem', color: 'var(--text-muted)' }}>({token.example})</span>
                </button>
              ))}
              {pathTemplate && (
                <button
                  type="button"
                  onClick={() => {
                    setActivePreset('custom')
                    setPathTemplate('')
                  }}
                  style={{
                    backgroundColor: 'transparent',
                    border: 'none',
                    color: 'var(--text-muted)',
                    fontSize: '0.78rem',
                    cursor: 'pointer',
                    padding: '6px 10px',
                    textDecoration: 'underline',
                  }}
                >
                  Clear Path
                </button>
              )}
            </div>
          </div>

          {/* Live Realistic Disk Preview */}
          <div
            style={{
              padding: '14px 18px',
              borderRadius: 'var(--radius-lg)',
              backgroundColor: 'var(--bg-tertiary)',
              border: '1px solid var(--border-subtle)',
              display: 'flex',
              flexDirection: 'column',
              gap: '6px',
            }}
          >
            <div style={{ fontSize: '0.72rem', fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.05em', color: 'var(--text-muted)' }}>
              Live Disk Folder Preview
            </div>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                flexWrap: 'wrap',
                gap: '8px',
                fontSize: '0.86rem',
                fontWeight: 600,
                color: 'var(--text-primary)',
              }}
            >
              <span style={{ color: 'var(--text-muted)' }}>📂 vault/</span>
              {previewSegments.map((seg, i) => {
                const isFile = i === previewSegments.length - 1
                return (
                  <span key={i} style={{ display: 'inline-flex', alignItems: 'center', gap: '6px' }}>
                    {isFile ? (
                      <span
                        style={{
                          display: 'inline-flex',
                          alignItems: 'center',
                          gap: '5px',
                          backgroundColor: 'var(--bg-surface)',
                          padding: '3px 10px',
                          borderRadius: '6px',
                          border: '1px solid var(--border-subtle)',
                          color: 'var(--accent-primary)',
                          fontFamily: 'var(--font-mono, monospace)',
                          fontSize: '0.82rem',
                        }}
                      >
                        <FileText size={14} />
                        {seg}
                      </span>
                    ) : (
                      <span
                        style={{
                          display: 'inline-flex',
                          alignItems: 'center',
                          gap: '5px',
                          backgroundColor: 'var(--bg-surface)',
                          padding: '3px 9px',
                          borderRadius: '6px',
                          border: '1px solid var(--border-subtle)',
                          color: 'var(--text-primary)',
                        }}
                      >
                        <Folder size={14} style={{ opacity: 0.8 }} />
                        {seg}
                      </span>
                    )}
                    {!isFile && <ChevronRight size={13} style={{ color: 'var(--text-muted)' }} />}
                  </span>
                )
              })}
            </div>
          </div>

          {/* Form Fields: Name, Rule, and Advanced Pattern */}
          <div style={{ display: 'flex', flexWrap: 'wrap', alignItems: 'flex-end', gap: '16px' }}>
            <div style={{ flex: '1 1 240px' }}>
              <label style={fieldLabelStyle}>Storage Path Name</label>
              <input
                type="text"
                className="input"
                placeholder="e.g. Invoices by Year"
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
              />
            </div>

            <div style={{ width: '180px' }}>
              <label style={fieldLabelStyle}>Auto-Assign Rule</label>
              <select
                className="input"
                value={algorithm}
                onChange={(e) => setAlgorithm(parseInt(e.target.value, 10))}
              >
                <option value={0}>Manual Only</option>
                <option value={1}>Any Word</option>
                <option value={2}>All Words</option>
                <option value={3}>Exact Match</option>
                <option value={4}>Regular Expression</option>
                <option value={6}>Auto (Classifier)</option>
              </select>
            </div>

            <button
              type="submit"
              className="btn btn-primary"
              disabled={isSubmitting}
              style={{
                height: '40px',
                padding: '0 20px',
                borderRadius: 'var(--radius-full)',
                fontWeight: 700,
                fontSize: '0.88rem',
                gap: '8px',
                display: 'inline-flex',
                alignItems: 'center',
                whiteSpace: 'nowrap',
              }}
            >
              <Plus size={16} /> Save Storage Path
            </button>
          </div>

          {/* Advanced Raw Pattern Editor (Expandable) */}
          {showAdvanced && (
            <div
              style={{
                padding: '16px',
                borderRadius: 'var(--radius-lg)',
                backgroundColor: 'var(--bg-tertiary)',
                border: '1px solid var(--border-subtle)',
                display: 'flex',
                flexDirection: 'column',
                gap: '8px',
              }}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <label style={{ ...fieldLabelStyle, marginBottom: 0 }}>
                  Raw Pattern Template (Advanced)
                </label>
                <button
                  type="button"
                  onClick={() => setPathTemplate(PRESETS[0].template)}
                  style={{
                    background: 'none',
                    border: 'none',
                    color: 'var(--accent-primary)',
                    fontSize: '0.74rem',
                    cursor: 'pointer',
                  }}
                >
                  Reset Default
                </button>
              </div>
              <input
                type="text"
                className="input"
                value={pathTemplate}
                onChange={(e) => {
                  setActivePreset('custom')
                  setPathTemplate(e.target.value)
                }}
                style={{
                  fontFamily: 'var(--font-mono, monospace)',
                  fontSize: '0.86rem',
                }}
              />
              <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                You can manually edit custom folders using variables like <code>&#123;created_year&#125;</code>, <code>&#123;correspondent&#125;</code>, etc.
              </span>
            </div>
          )}
        </div>
      </form>

      {/* Success Banner */}
      {successMsg && (
        <div
          style={{
            padding: '12px 18px',
            borderRadius: 'var(--radius-md)',
            backgroundColor: 'rgba(16, 185, 129, 0.15)',
            border: '1px solid rgba(16, 185, 129, 0.3)',
            color: '#10b981',
            fontSize: '0.86rem',
            fontWeight: 600,
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
          }}
        >
          <Check size={16} /> {successMsg}
        </div>
      )}

      {/* Existing Storage Paths Section */}
      <div>
        <div style={{ fontSize: '1rem', fontWeight: 700, marginBottom: '14px', color: 'var(--text-primary)' }}>
          Configured Storage Paths ({paths.length})
        </div>

        {loading ? (
          <div style={{ padding: '40px', textAlign: 'center', color: 'var(--text-muted)' }}>
            Loading storage paths...
          </div>
        ) : paths.length === 0 ? (
          <div
            className="card-panel"
            style={{ padding: '48px', textAlign: 'center', color: 'var(--text-muted)', borderRadius: 'var(--radius-xl)' }}
          >
            No custom storage paths created yet. Documents will use the default storage layout.
          </div>
        ) : (
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(320px, 1fr))', gap: '16px' }}>
            {paths.map((p) => {
              const segs = getSimulatedPreview(p.path || '')
              return (
                <div
                  key={p.id}
                  className="card-panel card-panel-interactive"
                  style={{
                    padding: '18px 20px',
                    display: 'flex',
                    flexDirection: 'column',
                    justifyContent: 'space-between',
                    gap: '14px',
                    borderRadius: 'var(--radius-lg)',
                  }}
                >
                  <div>
                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                        <div
                          style={{
                            width: '32px',
                            height: '32px',
                            borderRadius: '8px',
                            backgroundColor: 'var(--accent-soft)',
                            color: 'var(--accent-primary)',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                          }}
                        >
                          <FolderTree size={17} />
                        </div>
                        <span style={{ fontWeight: 700, fontSize: '0.96rem', color: 'var(--text-primary)' }}>
                          {p.name}
                        </span>
                      </div>
                      <DeleteIconButton onClick={() => handleDelete(p.id)} />
                    </div>

                    {/* Visual Disk Path Simulation */}
                    <div
                      style={{
                        marginTop: '12px',
                        padding: '10px 14px',
                        borderRadius: 'var(--radius-md)',
                        backgroundColor: 'var(--bg-tertiary)',
                        border: '1px solid var(--border-subtle)',
                        display: 'flex',
                        flexDirection: 'column',
                        gap: '6px',
                      }}
                    >
                      <div
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          flexWrap: 'wrap',
                          gap: '6px',
                          fontSize: '0.8rem',
                          fontWeight: 600,
                          color: 'var(--text-primary)',
                        }}
                      >
                        {segs.map((seg, idx) => (
                          <span key={idx} style={{ display: 'inline-flex', alignItems: 'center', gap: '4px' }}>
                            {idx === segs.length - 1 ? (
                              <span style={{ color: 'var(--accent-primary)', fontFamily: 'var(--font-mono, monospace)' }}>
                                {seg}
                              </span>
                            ) : (
                              <span>{seg}</span>
                            )}
                            {idx < segs.length - 1 && <span style={{ color: 'var(--text-muted)' }}>/</span>}
                          </span>
                        ))}
                      </div>

                      <div
                        style={{
                          fontSize: '0.72rem',
                          color: 'var(--text-muted)',
                          fontFamily: 'var(--font-mono, monospace)',
                        }}
                      >
                        Pattern: {p.path}
                      </div>
                    </div>
                  </div>

                  <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                    <span>{p.document_count ?? 0} document(s) assigned</span>
                    <span style={{ textTransform: 'capitalize' }}>
                      {p.matching_algorithm === 0 ? 'Manual' : 'Automated'}
                    </span>
                  </div>
                </div>
              )
            })}
          </div>
        )}
      </div>
    </div>
  )
}
