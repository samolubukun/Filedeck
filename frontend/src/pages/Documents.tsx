import { DocumentFilters } from '../components/DocumentFilters'
import { BulkActionBar } from '../components/BulkActionBar'
import { DocumentCard } from '../components/DocumentCard'
import { DocumentLargeCard } from '../components/DocumentLargeCard'
import { DocumentTable } from '../components/DocumentTable'
import { useDocuments } from '../api/useDocuments'
import { useTags } from '../api/useTags'
import { useCorrespondents } from '../api/useCorrespondents'
import { useDocumentTypes } from '../api/useDocumentTypes'
import { useFilterStore } from '../store/useFilterStore'
import { useUiStore } from '../store/useUiStore'

export function Documents() {
  const { documents } = useDocuments()
  const { tags } = useTags()
  const { correspondents } = useCorrespondents()
  const { documentTypes } = useDocumentTypes()
  const viewMode = useFilterStore((s) => s.viewMode)
  const selectedDocIds = useFilterStore((s) => s.selectedDocIds)
  const setSelectedDocIds = useFilterStore((s) => s.setSelectedDocIds)
  const toggleSelectedDoc = useFilterStore((s) => s.toggleSelectedDoc)
  const setSearchQuery = useFilterStore((s) => s.setSearchQuery)
  const setActiveDocId = useUiStore((s) => s.setActiveDocId)

  const handleSelectAll = () => {
    if (selectedDocIds.length === documents.length) {
      setSelectedDocIds([])
    } else {
      setSelectedDocIds(documents.map((d) => d.id))
    }
  }

  const handleMoreLikeThis = (id: number) => {
    const doc = documents.find((d) => d.id === id)
    if (doc) {
      // Search for words from title or query
      const keywords = doc.title.split(/[\s:_-]+/).filter((w) => w.length > 3).slice(0, 3).join(' ')
      setSearchQuery(keywords || doc.title)
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
      <DocumentFilters />
      <BulkActionBar />

      {documents.length === 0 ? (
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
          <div style={{ fontSize: '1.1rem', fontWeight: 700, color: 'var(--text-primary)' }}>
            No documents match current filters
          </div>
          <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
            Try resetting filters or drag-and-drop a file to ingest.
          </div>
        </div>
      ) : viewMode === 'cards' ? (
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))', gap: '20px' }}>
          {documents.map((doc) => (
            <DocumentCard
              key={doc.id}
              doc={doc}
              tags={tags}
              onOpen={() => setActiveDocId(doc.id)}
              selection={{
                selected: selectedDocIds.includes(doc.id),
                onToggle: () => toggleSelectedDoc(doc.id),
              }}
            />
          ))}
        </div>
      ) : viewMode === 'large_cards' ? (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
          {documents.map((doc) => (
            <DocumentLargeCard
              key={doc.id}
              doc={doc}
              tags={tags}
              correspondents={correspondents}
              documentTypes={documentTypes}
              onOpen={() => setActiveDocId(doc.id)}
              onMoreLikeThis={handleMoreLikeThis}
              selection={{
                selected: selectedDocIds.includes(doc.id),
                onToggle: () => toggleSelectedDoc(doc.id),
              }}
            />
          ))}
        </div>
      ) : (
        <DocumentTable
          documents={documents}
          tags={tags}
          correspondents={correspondents}
          documentTypes={documentTypes}
          selectedDocIds={selectedDocIds}
          onToggleSelect={toggleSelectedDoc}
          onSelectAll={handleSelectAll}
          onOpen={setActiveDocId}
        />
      )}
    </div>
  )
}
