import { useEffect, type ComponentType } from 'react'
import { Sidebar } from './components/Sidebar'
import { Header } from './components/Header'
import { UploadModal } from './components/UploadModal'
import { DocumentDrawer } from './components/DocumentDrawer'
import { GlobalSearchModal } from './components/GlobalSearchModal'
import { GlobalModal } from './components/GlobalModal'
import { Dashboard } from './pages/Dashboard'
import { Documents } from './pages/Documents'
import { Chat } from './pages/Chat'
import { Tags } from './pages/Tags'
import { Correspondents } from './pages/Correspondents'
import { DocumentTypes } from './pages/DocumentTypes'
import { Settings } from './pages/Settings'
import { StoragePaths } from './pages/StoragePaths'
import { CustomFields } from './pages/CustomFields'
import { Tasks } from './pages/Tasks'
import { Trash } from './pages/Trash'
import { Logs } from './pages/Logs'
import { SavedViews } from './pages/SavedViews'
import { Workflows } from './pages/Workflows'
import { Mail } from './pages/Mail'
import { PdfStudio } from './pages/PdfStudio'
import { useUiStore } from './store/useUiStore'
import { useDocuments, useRefreshAll } from './api/useDocuments'
import { useTags } from './api/useTags'
import { useCorrespondents } from './api/useCorrespondents'
import { useDocumentTypes } from './api/useDocumentTypes'
import { useStoragePaths } from './api/useStoragePaths'
import { useLiveStatus } from './api/useSystem'
import type { NavView } from './types'

const PAGES: Record<NavView, ComponentType> = {
  dashboard: Dashboard,
  documents: Documents,
  pdf_studio: PdfStudio,
  saved_views: SavedViews,
  chat: Chat,
  tags: Tags,
  correspondents: Correspondents,
  document_types: DocumentTypes,
  storage_paths: StoragePaths,
  custom_fields: CustomFields,
  workflows: Workflows,
  mail: Mail,
  tasks: Tasks,
  trash: Trash,
  logs: Logs,
  settings: Settings,
}

export function App() {
  const currentNav = useUiStore((s) => s.currentNav)
  const isUploadOpen = useUiStore((s) => s.isUploadOpen)
  const setUploadOpen = useUiStore((s) => s.setUploadOpen)
  const activeDocId = useUiStore((s) => s.activeDocId)
  const setActiveDocId = useUiStore((s) => s.setActiveDocId)
  const isSearchOpen = useUiStore((s) => s.isSearchOpen)
  const setSearchOpen = useUiStore((s) => s.setSearchOpen)
  const { documents } = useDocuments()
  const refreshAll = useRefreshAll()
  const { tags } = useTags()
  const { correspondents } = useCorrespondents()
  const { documentTypes } = useDocumentTypes()
  const { storagePaths } = useStoragePaths()
  const syncFromLocation = useUiStore((s) => s.syncFromLocation)

  useEffect(() => {
    const handleHash = () => syncFromLocation()
    window.addEventListener('hashchange', handleHash)
    return () => window.removeEventListener('hashchange', handleHash)
  }, [syncFromLocation])

  useLiveStatus()

  const Page = PAGES[currentNav]

  return (
    <div style={{ display: 'flex', minHeight: '100vh', backgroundColor: 'var(--bg-primary)' }}>
      <Sidebar />

      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
        <Header />
        <main style={{ flex: 1, padding: '32px 36px', maxWidth: '1440px', width: '100%', margin: '0 auto', boxSizing: 'border-box' }}>
          <Page />
        </main>
      </div>

      <UploadModal isOpen={isUploadOpen} onClose={() => setUploadOpen(false)} onSuccess={refreshAll} />

      <GlobalSearchModal isOpen={isSearchOpen} onClose={() => setSearchOpen(false)} />

      <GlobalModal />

      <DocumentDrawer
        documentId={activeDocId}
        documentIds={documents?.map((d) => d.id)}
        onNavigate={(id) => setActiveDocId(id)}
        onClose={() => setActiveDocId(null)}
        onUpdate={refreshAll}
        tags={tags}
        correspondents={correspondents}
        documentTypes={documentTypes}
        storagePaths={storagePaths}
      />
    </div>
  )
}
