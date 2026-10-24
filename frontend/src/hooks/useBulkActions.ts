import { api } from '../api/client'
import { useRefreshAll } from '../api/useDocuments'
import { useFilterStore } from '../store/useFilterStore'
import { confirmModal, alertModal } from '../store/useModalStore'

export function useBulkActions() {
  const selectedDocIds = useFilterStore((s) => s.selectedDocIds)
  const setSelectedDocIds = useFilterStore((s) => s.setSelectedDocIds)
  const refreshAll = useRefreshAll()

  const bulkDelete = async () => {
    const ok = await confirmModal({
      title: 'Delete Documents',
      message: `Are you sure you want to delete ${selectedDocIds.length} document${selectedDocIds.length > 1 ? 's' : ''}?`,
      confirmText: 'Delete',
      danger: true,
    })
    if (!ok) return

    try {
      await api.bulkEdit({ documents: selectedDocIds, method: 'delete' })
      setSelectedDocIds([])
      refreshAll()
    } catch {
      alertModal({ title: 'Error', message: 'Bulk delete failed. Please try again.', type: 'error' })
    }
  }

  const bulkDownload = async () => {
    try {
      const res = await fetch('/api/documents/bulk_download/', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ documents: selectedDocIds }),
      })
      const blob = await res.blob()
      const url = window.URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = 'documents.zip'
      a.click()
    } catch {
      alertModal({ title: 'Error', message: 'Bulk download failed.', type: 'error' })
    }
  }

  return { bulkDelete, bulkDownload }
}
