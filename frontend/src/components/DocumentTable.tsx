import { Download, Eye, MessageSquare, Hash, FileText } from 'lucide-react'
import type { DocumentItem, TaxonomyItem } from '../types'
import { TagBadges } from './TagBadges'

interface DocumentTableProps {
  documents: DocumentItem[]
  tags: TaxonomyItem[]
  correspondents: TaxonomyItem[]
  documentTypes: TaxonomyItem[]
  selectedDocIds: number[]
  onToggleSelect: (id: number) => void
  onSelectAll: () => void
  onOpen: (id: number) => void
}

export function DocumentTable({
  documents,
  tags,
  correspondents,
  documentTypes,
  selectedDocIds,
  onToggleSelect,
  onSelectAll,
  onOpen,
}: DocumentTableProps) {
  return (
    <div className="card-panel" style={{ overflowX: 'auto', borderRadius: 'var(--radius-lg)' }}>
      <table style={{ width: '100%', borderCollapse: 'collapse', textAlign: 'left', fontSize: '0.85rem' }}>
        <thead>
          <tr style={{ borderBottom: '1px solid var(--border-subtle)', color: 'var(--text-muted)', backgroundColor: 'var(--bg-tertiary)' }}>
            <th style={{ padding: '14px 16px', width: '38px' }}>
              <input
                type="checkbox"
                checked={selectedDocIds.length === documents.length && documents.length > 0}
                onChange={onSelectAll}
                style={{ cursor: 'pointer', width: '16px', height: '16px', accentColor: 'var(--accent-primary)' }}
              />
            </th>
            <th style={{ padding: '14px 16px', width: '90px' }}>ASN</th>
            <th style={{ padding: '14px 16px' }}>Correspondent</th>
            <th style={{ padding: '14px 16px' }}>Title</th>
            <th style={{ padding: '14px 16px', width: '70px', textAlign: 'center' }}>Notes</th>
            <th style={{ padding: '14px 16px' }}>Document Type</th>
            <th style={{ padding: '14px 16px' }}>Created</th>
            <th style={{ padding: '14px 16px', width: '70px', textAlign: 'center' }}>Pages</th>
            <th style={{ padding: '14px 16px', width: '90px', textAlign: 'right' }}>Actions</th>
          </tr>
        </thead>
        <tbody>
          {documents.map((doc) => {
            const isSelected = selectedDocIds.includes(doc.id)
            const corr = correspondents.find((c) => c.id === doc.correspondent)
            const docType = documentTypes.find((t) => t.id === doc.document_type)
            const noteCount = doc.notes?.length || 0

            return (
              <tr
                key={doc.id}
                style={{
                  borderBottom: '1px solid var(--border-subtle)',
                  cursor: 'pointer',
                  backgroundColor: isSelected ? 'var(--accent-soft)' : 'transparent',
                  transition: 'background-color 0.15s ease',
                }}
                onClick={() => onOpen(doc.id)}
                onMouseEnter={(e) => {
                  if (!isSelected) e.currentTarget.style.backgroundColor = 'var(--bg-card-hover)'
                }}
                onMouseLeave={(e) => {
                  if (!isSelected) e.currentTarget.style.backgroundColor = 'transparent'
                }}
              >
                {/* Checkbox */}
                <td style={{ padding: '14px 16px' }} onClick={(e) => e.stopPropagation()}>
                  <input
                    type="checkbox"
                    checked={isSelected}
                    onChange={() => onToggleSelect(doc.id)}
                    style={{ cursor: 'pointer', width: '16px', height: '16px', accentColor: 'var(--accent-primary)' }}
                  />
                </td>

                {/* ASN */}
                <td style={{ padding: '14px 16px', whiteSpace: 'nowrap' }}>
                  {doc.archive_serial_number != null ? (
                    <span
                      style={{
                        display: 'inline-flex',
                        alignItems: 'center',
                        gap: '2px',
                        fontSize: '0.72rem',
                        fontWeight: 700,
                        color: 'var(--accent-primary)',
                        backgroundColor: 'var(--accent-soft)',
                        padding: '2px 7px',
                        borderRadius: 'var(--radius-full)',
                      }}
                    >
                      <Hash size={11} /> {doc.archive_serial_number}
                    </span>
                  ) : (
                    <span style={{ color: 'var(--text-muted)', fontSize: '0.78rem' }}>-</span>
                  )}
                </td>

                {/* Correspondent */}
                <td style={{ padding: '14px 16px', color: 'var(--text-secondary)', fontWeight: 600 }}>
                  {corr?.name || '-'}
                </td>

                {/* Title + inline eye + Tags */}
                <td style={{ padding: '14px 16px' }}>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                      <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>{doc.title}</span>
                      <button
                        title="Quick Preview"
                        onClick={(e) => {
                          e.stopPropagation()
                          window.open(`/api/documents/${doc.id}/preview/`, '_blank')
                        }}
                        style={{
                          background: 'transparent',
                          border: 'none',
                          color: 'var(--text-muted)',
                          cursor: 'pointer',
                          padding: '2px',
                          display: 'flex',
                          alignItems: 'center',
                          opacity: 0.6,
                        }}
                        onMouseEnter={(e) => {
                          e.currentTarget.style.opacity = '1'
                          e.currentTarget.style.color = 'var(--accent-primary)'
                        }}
                        onMouseLeave={(e) => {
                          e.currentTarget.style.opacity = '0.6'
                          e.currentTarget.style.color = 'var(--text-muted)'
                        }}
                      >
                        <Eye size={14} />
                      </button>
                    </div>

                    {doc.tags && doc.tags.length > 0 && (
                      <div style={{ display: 'flex', gap: '4px', flexWrap: 'wrap' }}>
                        <TagBadges tagIds={doc.tags} tags={tags} />
                      </div>
                    )}
                  </div>
                </td>

                {/* Notes */}
                <td style={{ padding: '14px 16px', textAlign: 'center' }}>
                  {noteCount > 0 ? (
                    <span
                      style={{
                        display: 'inline-flex',
                        alignItems: 'center',
                        gap: '3px',
                        fontSize: '0.72rem',
                        fontWeight: 600,
                        backgroundColor: 'var(--bg-tertiary)',
                        color: 'var(--text-secondary)',
                        padding: '2px 7px',
                        borderRadius: 'var(--radius-full)',
                      }}
                      title={`${noteCount} note${noteCount > 1 ? 's' : ''}`}
                    >
                      <MessageSquare size={11} /> {noteCount}
                    </span>
                  ) : (
                    <span style={{ color: 'var(--text-muted)', opacity: 0.4 }}>-</span>
                  )}
                </td>

                {/* Document Type */}
                <td style={{ padding: '14px 16px', color: 'var(--text-secondary)' }}>
                  {docType ? (
                    <span style={{ display: 'inline-flex', alignItems: 'center', gap: '5px' }}>
                      <FileText size={13} style={{ opacity: 0.6 }} /> {docType.name}
                    </span>
                  ) : (
                    '-'
                  )}
                </td>

                {/* Created Date */}
                <td style={{ padding: '14px 16px', color: 'var(--text-muted)', whiteSpace: 'nowrap' }}>
                  {doc.created}
                </td>

                {/* Page Count */}
                <td style={{ padding: '14px 16px', textAlign: 'center', color: 'var(--text-muted)' }}>
                  {doc.page_count != null ? doc.page_count : '-'}
                </td>

                {/* Actions */}
                <td style={{ padding: '14px 16px', textAlign: 'right' }} onClick={(e) => e.stopPropagation()}>
                  <div style={{ display: 'inline-flex', alignItems: 'center', gap: '8px' }}>
                    <button
                      title="View preview"
                      onClick={() => window.open(`/api/documents/${doc.id}/preview/`, '_blank')}
                      style={{
                        background: 'transparent',
                        border: 'none',
                        color: 'var(--text-muted)',
                        cursor: 'pointer',
                        padding: '3px',
                      }}
                    >
                      <Eye size={15} />
                    </button>
                    <a
                      href={doc.download_url}
                      download={doc.original_file_name || `${doc.title}.pdf`}
                      style={{ color: 'var(--text-muted)', display: 'inline-flex' }}
                      title="Download"
                    >
                      <Download size={15} />
                    </a>
                  </div>
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}
