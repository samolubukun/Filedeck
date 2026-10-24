import { FileText, Calendar, Download, Eye, GitBranch, ExternalLink, Hash } from 'lucide-react'
import type { DocumentItem, TaxonomyItem } from '../types'
import { TagBadges } from './TagBadges'

interface DocumentLargeCardProps {
  doc: DocumentItem
  tags: TaxonomyItem[]
  correspondents?: TaxonomyItem[]
  documentTypes?: TaxonomyItem[]
  onOpen: () => void
  onMoreLikeThis?: (id: number) => void
  selection?: { selected: boolean; onToggle: () => void }
}

export function DocumentLargeCard({
  doc,
  tags,
  correspondents = [],
  documentTypes = [],
  onOpen,
  onMoreLikeThis,
  selection,
}: DocumentLargeCardProps) {
  const corr = correspondents.find((c) => c.id === doc.correspondent)
  const docType = documentTypes.find((t) => t.id === doc.document_type)

  const handleDownload = (e: React.MouseEvent) => {
    e.stopPropagation()
    const a = document.createElement('a')
    a.href = doc.download_url
    a.download = doc.original_file_name || `${doc.title}.pdf`
    a.click()
  }

  const handleView = (e: React.MouseEvent) => {
    e.stopPropagation()
    window.open(`/api/documents/${doc.id}/preview/`, '_blank')
  }

  const handleMoreLike = (e: React.MouseEvent) => {
    e.stopPropagation()
    if (onMoreLikeThis) {
      onMoreLikeThis(doc.id)
    }
  }

  // Get snippet preview
  const snippet = doc.content
    ? doc.content.replace(/\s+/g, ' ').trim().slice(0, 320) + (doc.content.length > 320 ? '...' : '')
    : 'No text extracted for this document.'

  return (
    <div
      className="card-panel card-panel-interactive"
      style={{
        padding: '20px 22px',
        display: 'flex',
        gap: '24px',
        alignItems: 'stretch',
        position: 'relative',
        cursor: 'pointer',
        borderRadius: 'var(--radius-lg)',
        borderColor: selection?.selected ? 'var(--accent-primary)' : 'var(--border-subtle)',
        boxShadow: selection?.selected
          ? '0 0 0 2px var(--accent-primary), var(--shadow-card)'
          : 'var(--shadow-card)',
      }}
      onClick={onOpen}
    >
      {/* Checkbox */}
      {selection && (
        <div
          style={{
            position: 'absolute',
            top: '14px',
            left: '14px',
            zIndex: 10,
          }}
          onClick={(e) => {
            e.stopPropagation()
            selection.onToggle()
          }}
        >
          <input
            type="checkbox"
            checked={selection.selected}
            onChange={() => {}}
            style={{ cursor: 'pointer', width: '17px', height: '17px', accentColor: 'var(--accent-primary)' }}
          />
        </div>
      )}

      {/* Left Thumbnail */}
      <div
        style={{
          width: '210px',
          minWidth: '210px',
          maxHeight: '200px',
          background: 'var(--bg-tertiary)',
          border: '1px solid var(--border-subtle)',
          borderRadius: 'var(--radius-md)',
          overflow: 'hidden',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          position: 'relative',
          flexShrink: 0,
        }}
      >
        <img
          src={doc.thumbnail_url}
          alt={doc.title}
          style={{ width: '100%', height: '100%', objectFit: 'contain' }}
          onError={(e) => {
            e.currentTarget.style.display = 'none'
          }}
        />
        <FileText size={48} style={{ color: 'var(--text-muted)', opacity: 0.35 }} />
      </div>

      {/* Right Content */}
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: '10px', minWidth: 0 }}>
        {/* Title & Tags */}
        <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: '12px' }}>
          <div style={{ display: 'flex', flexWrap: 'wrap', alignItems: 'center', gap: '8px' }}>
            <span style={{ fontWeight: 800, fontSize: '1.05rem', color: 'var(--text-primary)' }}>
              {corr ? `${corr.name}: ` : ''}{doc.title}
            </span>
            <TagBadges tagIds={doc.tags} tags={tags} />
          </div>

          {doc.archive_serial_number != null && (
            <span
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '3px',
                fontSize: '0.72rem',
                fontWeight: 700,
                color: 'var(--accent-primary)',
                backgroundColor: 'var(--accent-soft)',
                border: '1px solid var(--accent-border)',
                padding: '2px 8px',
                borderRadius: 'var(--radius-full)',
                flexShrink: 0,
              }}
            >
              <Hash size={12} /> ASN {doc.archive_serial_number}
            </span>
          )}
        </div>

        {/* Text Snippet */}
        <div
          style={{
            fontSize: '0.82rem',
            color: 'var(--text-secondary)',
            lineHeight: 1.5,
            maxHeight: '4.5em',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            display: '-webkit-box',
            WebkitLineClamp: 3,
            WebkitBoxOrient: 'vertical',
          }}
        >
          {snippet}
        </div>

        {/* Action Buttons */}
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px', marginTop: 'auto', paddingTop: '6px' }}>
          {onMoreLikeThis && (
            <button
              className="btn btn-secondary"
              style={{ padding: '5px 10px', fontSize: '0.75rem', gap: '5px' }}
              onClick={handleMoreLike}
              title="Find documents with similar content"
            >
              <GitBranch size={13} /> More like this
            </button>
          )}

          <button
            className="btn btn-secondary"
            style={{ padding: '5px 10px', fontSize: '0.75rem', gap: '5px' }}
            onClick={(e) => {
              e.stopPropagation()
              onOpen()
            }}
          >
            <ExternalLink size={13} /> Open
          </button>

          <button
            className="btn btn-secondary"
            style={{ padding: '5px 10px', fontSize: '0.75rem', gap: '5px' }}
            onClick={handleView}
          >
            <Eye size={13} /> View
          </button>

          <button
            className="btn btn-secondary"
            style={{ padding: '5px 10px', fontSize: '0.75rem', gap: '5px' }}
            onClick={handleDownload}
          >
            <Download size={13} /> Download
          </button>
        </div>

        {/* Footer Meta */}
        <div
          style={{
            display: 'flex',
            flexWrap: 'wrap',
            alignItems: 'center',
            gap: '16px',
            fontSize: '0.75rem',
            color: 'var(--text-muted)',
            borderTop: '1px solid var(--border-subtle)',
            paddingTop: '10px',
            marginTop: '2px',
          }}
        >
          {docType && (
            <span style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
              <FileText size={12} /> {docType.name}
            </span>
          )}

          <span style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
            <Calendar size={12} /> {doc.created}
          </span>

          {doc.page_count != null && (
            <span>{doc.page_count} page{doc.page_count === 1 ? '' : 's'}</span>
          )}

          {doc.original_file_name && (
            <span style={{ opacity: 0.7 }}>{doc.original_file_name}</span>
          )}
        </div>
      </div>
    </div>
  )
}
