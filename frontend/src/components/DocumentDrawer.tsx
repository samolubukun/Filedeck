import React, { useState, useEffect } from 'react'
import {
  X,
  Download,
  FileText,
  Tag as TagIcon,
  Trash2,
  Plus,
  Check,
  Layers,
  Upload,
  Activity,
  Sparkles,
  Table,
  Send,
  Bot,
  User,
  Copy,
  ChevronLeft,
  ChevronRight,
  ExternalLink,
  RotateCw,
  Eye,
  Wrench,
  Share2,
  Square,
  Link as LinkIcon,
} from 'lucide-react'
import { api } from '../api/client'
import type { DocumentItem, TaxonomyItem, DocumentHistoryItem } from '../types'
import { confirmModal, alertModal, promptModal } from '../store/useModalStore'

interface DocumentDrawerProps {
  documentId: number | null
  onClose: () => void
  onUpdate: () => void
  onNavigate?: (id: number) => void
  documentIds?: number[]
  tags: TaxonomyItem[]
  correspondents: TaxonomyItem[]
  documentTypes: TaxonomyItem[]
  storagePaths?: TaxonomyItem[]
}

export const DocumentDrawer: React.FC<DocumentDrawerProps> = ({
  documentId,
  onClose,
  onUpdate,
  onNavigate,
  documentIds,
  tags,
  correspondents,
  documentTypes,
  storagePaths = [],
}) => {
  const [doc, setDoc] = useState<DocumentItem | null>(null)
  const [loading, setLoading] = useState(false)
  const [title, setTitle] = useState('')
  const [selectedCorr, setSelectedCorr] = useState<number | undefined>(undefined)
  const [selectedType, setSelectedType] = useState<number | undefined>(undefined)
  const [selectedStoragePath, setSelectedStoragePath] = useState<number | undefined>(undefined)
  const [selectedTags, setSelectedTags] = useState<number[]>([])
  const [created, setCreated] = useState('')
  const [asn, setAsn] = useState<string>('')
  const [newNote, setNewNote] = useState('')
  const [isSaving, setIsSaving] = useState(false)
  const [savedSuccess, setSavedSuccess] = useState(false)
  const [activeTab, setActiveTab] = useState<'details' | 'content' | 'ai_chat' | 'tables' | 'versions' | 'history' | 'notes'>('details')
  const [copiedContent, setCopiedContent] = useState(false)

  // Custom fields
  const [customFieldsDef, setCustomFieldsDef] = useState<any[]>([])
  const [customFieldValues, setCustomFieldValues] = useState<Record<number, any>>({})

  // History state
  const [historyItems, setHistoryItems] = useState<DocumentHistoryItem[]>([])
  const [historyLoading, setHistoryLoading] = useState(false)

  const formatActor = (actor: any) => {
    if (!actor) return 'System'
    if (typeof actor === 'string') return actor
    if (typeof actor === 'object') {
      if (actor.username) return actor.username
      const fullName = `${actor.first_name || ''} ${actor.last_name || ''}`.trim()
      if (fullName) return fullName
      if (actor.id) return `User #${actor.id}`
    }
    return String(actor)
  }

  // Version upload & edit state
  const [uploadingVersion, setUploadingVersion] = useState(false)
  const [versionLabel, setVersionLabel] = useState('')
  const [newVersionFile, setNewVersionFile] = useState<File | null>(null)

  // AI Chat state
  const [chatMessages, setChatMessages] = useState<Array<{ role: 'user' | 'assistant'; content: string }>>([])
  const [chatInput, setChatInput] = useState('')
  const [chatStreaming, setChatStreaming] = useState(false)
  const [chatAbortController, setChatAbortController] = useState<AbortController | null>(null)

  // Share link state
  const [shareModalOpen, setShareModalOpen] = useState(false)
  const [shareExpiryHours, setShareExpiryHours] = useState(24)
  const [shareSlug, setShareSlug] = useState<string | null>(null)
  const [shareCopied, setShareCopied] = useState(false)
  const [shareLoading, setShareLoading] = useState(false)

  // Tables state
  const [tables, setTables] = useState<Array<{ page: number; index: number; rows: string[][] }>>([])
  const [tablesLoading, setTablesLoading] = useState(false)
  const [tablesFetched, setTablesFetched] = useState(false)
  const [copiedTableIdx, setCopiedTableIdx] = useState<number | null>(null)

  // Preview reload key
  const [previewKey, setPreviewKey] = useState(0)

  // Load custom field definitions once
  useEffect(() => {
    api
      .getCustomFields()
      .then((res) => setCustomFieldsDef(res.results || []))
      .catch(() => {})
  }, [])

  const fetchDoc = async (id: number) => {
    setLoading(true)
    try {
      const data = await api.getDocument(id)
      setDoc(data)
      setTitle(data.title)
      setSelectedCorr(data.correspondent || undefined)
      setSelectedType(data.document_type || undefined)
      setSelectedStoragePath(data.storage_path || undefined)
      setSelectedTags(data.tags || [])
      setCreated(data.created ? data.created.split('T')[0] : '')
      setAsn(data.archive_serial_number ? String(data.archive_serial_number) : '')

      const cfMap: Record<number, any> = {}
      for (const cf of data.custom_fields || []) {
        cfMap[cf.field] = cf.value
      }
      setCustomFieldValues(cfMap)
    } catch (err) {
      console.error(err)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    if (!documentId) {
      setDoc(null)
      return
    }
    setChatMessages([])
    setChatInput('')
    setTables([])
    setTablesFetched(false)
    fetchDoc(documentId)
  }, [documentId])

  useEffect(() => {
    if (activeTab === 'history' && documentId) {
      setHistoryLoading(true)
      api
        .getDocumentHistory(documentId)
        .then((items) => setHistoryItems(Array.isArray(items) ? items : []))
        .catch(console.error)
        .finally(() => setHistoryLoading(false))
    }
  }, [activeTab, documentId])

  useEffect(() => {
    if (activeTab === 'tables' && documentId && !tablesFetched && !tablesLoading) {
      setTablesLoading(true)
      api
        .getDocumentTables(documentId)
        .then((res) => {
          setTables(res.tables || [])
          setTablesFetched(true)
        })
        .catch(console.error)
        .finally(() => setTablesLoading(false))
    }
  }, [activeTab, documentId, tablesFetched, tablesLoading])

  // Keyboard shortcut listener (Esc to close, Ctrl+S to save)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose()
      } else if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault()
        handleSave()
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [title, selectedCorr, selectedType, selectedStoragePath, selectedTags, created, asn, customFieldValues, doc])

  if (!documentId) return null

  const handleSendChat = async (promptOverride?: string) => {
    const textToSend = promptOverride || chatInput
    if (!textToSend.trim() || !doc || chatStreaming) return
    const userMsg = { role: 'user' as const, content: textToSend.trim() }
    const currentHist = [...chatMessages, userMsg]
    setChatMessages(currentHist)
    setChatInput('')
    setChatStreaming(true)

    setChatMessages((prev) => [...prev, { role: 'assistant', content: '' }])

    const controller = new AbortController()
    setChatAbortController(controller)

    try {
      await api.chatDocumentStream(
        doc.id,
        userMsg.content,
        chatMessages,
        (token) => {
          setChatMessages((prev) => {
            const updated = [...prev]
            const last = updated[updated.length - 1]
            if (last && last.role === 'assistant') {
              last.content += token
            }
            return updated
          })
        },
        controller.signal
      )
    } catch (err: any) {
      if (err.name !== 'AbortError') {
        setChatMessages((prev) => {
          const updated = [...prev]
          const last = updated[updated.length - 1]
          if (last && last.role === 'assistant') {
            last.content = `Error: ${err.message || 'Failed to get AI response'}`
          }
          return updated
        })
      }
    } finally {
      setChatStreaming(false)
      setChatAbortController(null)
    }
  }

  const handleStopChat = () => {
    if (chatAbortController) {
      chatAbortController.abort()
      setChatAbortController(null)
    }
    setChatStreaming(false)
  }

  const handleCreateShareLink = async () => {
    if (!doc) return
    setShareLoading(true)
    try {
      const res = await api.createShareLink({ document: doc.id, expiry_hours: shareExpiryHours })
      if (res && res.slug) {
        setShareSlug(res.slug)
      }
    } catch (err: any) {
      alertModal({ title: 'Share Error', message: err?.message || 'Failed to create share link', type: 'error' })
    } finally {
      setShareLoading(false)
    }
  }

  const handleCopyShareLink = () => {
    if (!shareSlug) return
    const url = `${window.location.origin}/share/${shareSlug}/`
    navigator.clipboard.writeText(url)
    setShareCopied(true)
    setTimeout(() => setShareCopied(false), 2000)
  }

  const copyTableAsCsv = (rows: string[][], idx: number) => {
    const csv = rows.map((r) => r.map((cell) => `"${cell.replace(/"/g, '""')}"`).join(',')).join('\n')
    navigator.clipboard.writeText(csv)
    setCopiedTableIdx(idx)
    setTimeout(() => setCopiedTableIdx(null), 2000)
  }

  const handleCopyContent = () => {
    if (!doc?.content) return
    navigator.clipboard.writeText(doc.content)
    setCopiedContent(true)
    setTimeout(() => setCopiedContent(false), 2000)
  }

  const handleSave = async () => {
    if (!doc) return
    setIsSaving(true)
    try {
      const formattedCfs = Object.entries(customFieldValues).map(([fieldId, val]) => ({
        field: parseInt(fieldId, 10),
        value: val,
      }))

      await api.updateDocument(doc.id, {
        title: title.trim() || doc.title,
        correspondent: selectedCorr || null,
        document_type: selectedType || null,
        storage_path: selectedStoragePath || null,
        tags: selectedTags,
        created: created ? new Date(created).toISOString() : undefined,
        archive_serial_number: asn ? parseInt(asn, 10) : null,
        custom_fields: formattedCfs,
      })
      setSavedSuccess(true)
      setTimeout(() => setSavedSuccess(false), 2500)
      onUpdate()
    } catch (err: any) {
      await alertModal({
        title: 'Save Failed',
        message: err.message || 'Failed to save document updates',
        type: 'danger',
      })
    } finally {
      setIsSaving(false)
    }
  }

  const handleSaveAndClose = async () => {
    await handleSave()
    onClose()
  }

  const handleDiscard = () => {
    if (doc) {
      setTitle(doc.title)
      setSelectedCorr(doc.correspondent || undefined)
      setSelectedType(doc.document_type || undefined)
      setSelectedStoragePath(doc.storage_path || undefined)
      setSelectedTags(doc.tags || [])
      setCreated(doc.created ? doc.created.split('T')[0] : '')
      setAsn(doc.archive_serial_number ? String(doc.archive_serial_number) : '')
    }
    onClose()
  }

  const handleDeleteDocument = async () => {
    if (!doc) return
    const ok = await confirmModal({
      title: 'Move to Trash',
      message: `Are you sure you want to move "${doc.title}" to Trash?`,
      confirmText: 'Move to Trash',
      danger: true,
    })
    if (!ok) return
    try {
      await api.deleteDocument(doc.id)
      onUpdate()
      onClose()
    } catch (err: any) {
      alertModal({ title: 'Error', message: err?.message || 'Failed to delete document', type: 'error' })
    }
  }

  const handleAddNote = async () => {
    if (!newNote.trim() || !doc) return
    try {
      await api.addNote(doc.id, newNote.trim())
      setNewNote('')
      const refreshed = await api.getDocument(doc.id)
      setDoc(refreshed)
      onUpdate()
    } catch (err: any) {
      await alertModal({
        title: 'Note Error',
        message: err.message || 'Failed to add note',
        type: 'danger',
      })
    }
  }

  const handleDeleteNote = async (noteId: number) => {
    if (!doc) return
    const ok = await confirmModal({
      title: 'Delete Note',
      message: 'Are you sure you want to delete this note?',
      confirmText: 'Delete Note',
      type: 'danger',
    })
    if (!ok) return

    try {
      await api.deleteNote(doc.id, noteId)
      const refreshed = await api.getDocument(doc.id)
      setDoc(refreshed)
      onUpdate()
    } catch (err: any) {
      await alertModal({
        title: 'Delete Failed',
        message: err.message || 'Failed to delete note',
        type: 'danger',
      })
    }
  }

  const toggleTag = (tagId: number) => {
    setSelectedTags((prev) =>
      prev.includes(tagId) ? prev.filter((id) => id !== tagId) : [...prev, tagId]
    )
  }

  const handleUploadVersion = async () => {
    if (!doc || !newVersionFile) return
    setUploadingVersion(true)
    try {
      const formData = new FormData()
      formData.append('document', newVersionFile)
      if (versionLabel.trim()) {
        formData.append('version_label', versionLabel.trim())
      }
      await api.uploadDocumentVersion(doc.id, formData)
      setNewVersionFile(null)
      setVersionLabel('')
      await fetchDoc(doc.id)
      onUpdate()
    } catch (err: any) {
      await alertModal({
        title: 'Version Upload Failed',
        message: err.message || 'Failed to upload document version',
        type: 'danger',
      })
    } finally {
      setUploadingVersion(false)
    }
  }

  const currentIndex = documentIds && documentId ? documentIds.indexOf(documentId) : -1
  const hasPrev = currentIndex > 0
  const hasNext = currentIndex >= 0 && currentIndex < (documentIds?.length || 0) - 1

  const handlePrev = () => {
    if (hasPrev && documentIds && onNavigate) {
      onNavigate(documentIds[currentIndex - 1])
    }
  }

  const handleNext = () => {
    if (hasNext && documentIds && onNavigate) {
      onNavigate(documentIds[currentIndex + 1])
    }
  }

  const handleCreateCorrespondent = async () => {
    const name = await promptModal({
      title: 'New Correspondent',
      message: 'Enter correspondent name:',
      placeholder: 'e.g. Acme Corp',
      confirmText: 'Create',
    })
    if (!name || !name.trim()) return
    try {
      const created = await api.createCorrespondent({ name: name.trim() })
      if (created && created.id) {
        setSelectedCorr(created.id)
        onUpdate()
      }
    } catch (err: any) {
      await alertModal({
        title: 'Creation Failed',
        message: `Could not create correspondent: ${err.message}`,
        type: 'danger',
      })
    }
  }

  const handleCreateDocumentType = async () => {
    const name = await promptModal({
      title: 'New Document Type',
      message: 'Enter document type name:',
      placeholder: 'e.g. Invoice, Contract',
      confirmText: 'Create',
    })
    if (!name || !name.trim()) return
    try {
      const created = await api.createDocumentType({ name: name.trim() })
      if (created && created.id) {
        setSelectedType(created.id)
        onUpdate()
      }
    } catch (err: any) {
      await alertModal({
        title: 'Creation Failed',
        message: `Could not create document type: ${err.message}`,
        type: 'danger',
      })
    }
  }

  const handleCreateStoragePath = async () => {
    const name = await promptModal({
      title: 'New Storage Path',
      message: 'Enter storage path name:',
      placeholder: 'e.g. Invoices / {created_year}',
      confirmText: 'Create',
    })
    if (!name || !name.trim()) return
    try {
      const created = await api.createStoragePath({
        name: name.trim(),
        path: '{created_year}/{correspondent}/{title}',
      })
      if (created && created.id) {
        setSelectedStoragePath(created.id)
        onUpdate()
      }
    } catch (err: any) {
      await alertModal({
        title: 'Creation Failed',
        message: `Could not create storage path: ${err.message}`,
        type: 'danger',
      })
    }
  }

  const handleNextAsn = async () => {
    try {
      const res: any = await api.getNextAsn()
      if (typeof res === 'number') {
        setAsn(String(res))
      } else if (res && res.next_asn != null) {
        setAsn(String(res.next_asn))
      }
    } catch (err: any) {
      await alertModal({
        title: 'ASN Failed',
        message: `Could not fetch next ASN: ${err.message}`,
        type: 'danger',
      })
    }
  }

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.75)',
        backdropFilter: 'blur(6px)',
        zIndex: 1000,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: '16px 20px',
      }}
      onClick={onClose}
    >
      <div
        className="card-panel"
        style={{
          width: '100%',
          maxWidth: '1750px',
          height: 'calc(100vh - 32px)',
          borderRadius: 'var(--radius-xl)',
          border: '1px solid var(--border-subtle)',
          display: 'flex',
          flexDirection: 'column',
          backgroundColor: 'var(--bg-secondary)',
          overflow: 'hidden',
          boxShadow: '0 25px 60px rgba(0, 0, 0, 0.65)',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Top Paperless-Style Action Bar */}
        <div
          style={{
            padding: '12px 20px',
            borderBottom: '1px solid var(--border-subtle)',
            backgroundColor: 'var(--bg-tertiary)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: '16px',
            flexShrink: 0,
          }}
        >
          {/* Left Title & Nav */}
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px', minWidth: 0, flex: 1 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
              <button
                type="button"
                className="btn btn-secondary"
                style={{ padding: '6px 9px' }}
                onClick={onClose}
                title="Close (Esc)"
              >
                <X size={15} />
              </button>
              {documentIds && documentIds.length > 1 && (
                <>
                  <button
                    type="button"
                    className="btn btn-secondary"
                    style={{ padding: '6px 9px' }}
                    disabled={!hasPrev}
                    onClick={handlePrev}
                    title="Previous Document (←)"
                  >
                    <ChevronLeft size={15} />
                  </button>
                  <button
                    type="button"
                    className="btn btn-secondary"
                    style={{ padding: '6px 9px' }}
                    disabled={!hasNext}
                    onClick={handleNext}
                    title="Next Document (→)"
                  >
                    <ChevronRight size={15} />
                  </button>
                </>
              )}
            </div>

            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', minWidth: 0 }}>
              <FileText size={18} style={{ color: 'var(--accent-primary)', flexShrink: 0 }} />
              <h2
                style={{
                  fontSize: '1.12rem',
                  fontWeight: 700,
                  margin: 0,
                  padding: 0,
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                  maxWidth: '380px',
                  color: 'var(--text-primary)',
                }}
              >
                {doc ? doc.title : 'Loading Document...'}
              </h2>
              {doc && (
                <span
                  style={{
                    fontSize: '0.72rem',
                    fontWeight: 700,
                    color: '#10b981',
                    backgroundColor: 'rgba(16, 185, 129, 0.15)',
                    border: '1px solid rgba(16, 185, 129, 0.3)',
                    padding: '2px 8px',
                    borderRadius: 'var(--radius-full)',
                    flexShrink: 0,
                  }}
                >
                  ID: {doc.id}
                </span>
              )}
            </div>

            {/* Quick Action buttons */}
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginLeft: 'auto' }}>
              <button
                type="button"
                className="btn btn-secondary"
                onClick={handleDiscard}
                style={{ fontSize: '0.8rem', padding: '6px 12px' }}
              >
                Discard
              </button>
              <button
                type="button"
                className="btn btn-secondary"
                onClick={handleSaveAndClose}
                disabled={isSaving}
                style={{ fontSize: '0.8rem', padding: '6px 12px' }}
              >
                Save & close
              </button>
              <button
                type="button"
                className="btn btn-primary"
                onClick={handleSave}
                disabled={isSaving}
                style={{ fontSize: '0.8rem', padding: '6px 16px', fontWeight: 700 }}
              >
                {savedSuccess ? '✓ Saved!' : isSaving ? 'Saving...' : 'Save'}
              </button>
            </div>
          </div>

          {/* Right Download & Delete Actions */}
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexShrink: 0 }}>
            {doc && (
              <>
                <button
                  type="button"
                  onClick={handleDeleteDocument}
                  className="btn btn-secondary"
                  style={{
                    color: '#ef4444',
                    borderColor: 'rgba(239, 68, 68, 0.3)',
                    fontSize: '0.8rem',
                    padding: '6px 12px',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '6px',
                  }}
                  title="Move document to Trash"
                >
                  <Trash2 size={14} /> Delete
                </button>
                <a
                  href={doc.download_url}
                  download
                  className="btn btn-secondary"
                  style={{
                    fontSize: '0.8rem',
                    padding: '6px 12px',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '6px',
                  }}
                  title="Download Document"
                >
                  <Download size={14} /> Download
                </a>

                <button
                  className="btn btn-secondary"
                  onClick={() => {
                    onClose()
                    window.location.hash = `#/pdf-studio?doc=${doc.id}`
                  }}
                  style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '0.82rem', padding: '6px 12px' }}
                  title="Open in PDF Studio to compress, watermark, sign, redact, or edit"
                >
                  <Wrench size={14} style={{ color: '#ec4899' }} /> PDF Studio
                </button>

                <button
                  type="button"
                  className="btn btn-secondary"
                  onClick={() => {
                    setShareSlug(null)
                    setShareModalOpen(true)
                  }}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '6px',
                    fontSize: '0.82rem',
                    padding: '6px 12px',
                    color: 'var(--accent-primary)',
                    borderColor: 'rgba(59, 130, 246, 0.3)',
                  }}
                  title="Create public share link"
                >
                  <Share2 size={14} /> Share
                </button>
              </>
            )}
          </div>
        </div>

        {/* MAIN SIDE-BY-SIDE SPLIT SCREEN BODY */}
        <div style={{ flex: 1, display: 'flex', minHeight: 0, overflow: 'hidden' }}>
          {/* ======================================================== */}
          {/* LEFT COLUMN: Metadata Form & Detail Tabs (48%-50% width) */}
          {/* ======================================================== */}
          <div
            style={{
              width: '48%',
              minWidth: '460px',
              maxWidth: '800px',
              display: 'flex',
              flexDirection: 'column',
              borderRight: '1px solid var(--border-subtle)',
              backgroundColor: 'var(--bg-secondary)',
              overflow: 'hidden',
            }}
          >
            {/* Tab Headers */}
            <div
              style={{
                display: 'flex',
                borderBottom: '1px solid var(--border-subtle)',
                backgroundColor: 'var(--bg-tertiary)',
                padding: '0 16px',
                overflowX: 'auto',
                flexShrink: 0,
              }}
            >
              {[
                { key: 'details', label: 'Details' },
                { key: 'content', label: 'Content' },
                { key: 'ai_chat', label: 'Ask AI ✦' },
                { key: 'tables', label: `Tables ${tables.length > 0 ? `(${tables.length})` : ''}` },
                { key: 'versions', label: `Versions (${doc?.versions?.length || 1})` },
                { key: 'history', label: 'History' },
                { key: 'notes', label: `Notes (${doc?.notes?.length || 0})` },
              ].map((tab) => (
                <button
                  key={tab.key}
                  onClick={() => setActiveTab(tab.key as any)}
                  style={{
                    padding: '11px 16px',
                    border: 'none',
                    background: 'transparent',
                    fontWeight: 600,
                    fontSize: '0.83rem',
                    color: activeTab === tab.key ? 'var(--accent-primary)' : 'var(--text-secondary)',
                    borderBottom: activeTab === tab.key ? '2px solid var(--accent-primary)' : '2px solid transparent',
                    cursor: 'pointer',
                    whiteSpace: 'nowrap',
                    transition: 'all 0.15s ease',
                  }}
                >
                  {tab.label}
                </button>
              ))}
            </div>

            {/* Tab Content Body (Scrollable Form) */}
            <div style={{ flex: 1, overflowY: 'auto', padding: '24px' }}>
              {loading ? (
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--text-secondary)' }}>
                  Loading document metadata...
                </div>
              ) : !doc ? (
                <div style={{ padding: '24px', color: 'var(--text-muted)' }}>Document not found.</div>
              ) : (
                <>
                  {/* DETAILS TAB (PRIMARY PAPERLESS METADATA FORM) */}
                  {activeTab === 'details' && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                      {/* Title Field */}
                      <div>
                        <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: 'var(--text-secondary)', marginBottom: '6px' }}>
                          Title
                        </label>
                        <input
                          type="text"
                          className="input"
                          value={title}
                          onChange={(e) => setTitle(e.target.value)}
                          placeholder="Document title..."
                        />
                      </div>

                      {/* ASN & Created Date */}
                      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px' }}>
                        <div>
                          <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: 'var(--text-secondary)', marginBottom: '6px' }}>
                            Archive Serial Number (ASN)
                          </label>
                          <div style={{ display: 'flex', gap: '8px' }}>
                            <input
                              type="number"
                              className="input"
                              placeholder="ASN #"
                              value={asn}
                              onChange={(e) => setAsn(e.target.value)}
                              style={{ flex: 1 }}
                            />
                            <button
                              type="button"
                              className="btn btn-secondary"
                              style={{ padding: '0 12px', fontSize: '0.8rem', fontWeight: 700 }}
                              title="Auto-assign next ASN"
                              onClick={handleNextAsn}
                            >
                              +1
                            </button>
                          </div>
                        </div>

                        <div>
                          <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: 'var(--text-secondary)', marginBottom: '6px' }}>
                            Date Created
                          </label>
                          <input
                            type="date"
                            className="input"
                            value={created}
                            onChange={(e) => setCreated(e.target.value)}
                          />
                        </div>
                      </div>

                      {/* Correspondent & Document Type */}
                      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px' }}>
                        <div>
                          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '6px' }}>
                            <label style={{ fontSize: '0.8rem', fontWeight: 600, color: 'var(--text-secondary)' }}>
                              Correspondent
                            </label>
                            <button
                              type="button"
                              onClick={handleCreateCorrespondent}
                              style={{
                                background: 'transparent',
                                border: 'none',
                                color: 'var(--accent-primary)',
                                fontSize: '0.75rem',
                                fontWeight: 600,
                                cursor: 'pointer',
                                display: 'inline-flex',
                                alignItems: 'center',
                                gap: '2px',
                              }}
                              title="Create new correspondent"
                            >
                              <Plus size={12} /> New
                            </button>
                          </div>
                          <select
                            className="input"
                            value={selectedCorr || ''}
                            onChange={(e) => setSelectedCorr(e.target.value ? parseInt(e.target.value, 10) : undefined)}
                          >
                            <option value="">(None)</option>
                            {correspondents.map((c) => (
                              <option key={c.id} value={c.id}>
                                {c.name}
                              </option>
                            ))}
                          </select>
                        </div>

                        <div>
                          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '6px' }}>
                            <label style={{ fontSize: '0.8rem', fontWeight: 600, color: 'var(--text-secondary)' }}>
                              Document Type
                            </label>
                            <button
                              type="button"
                              onClick={handleCreateDocumentType}
                              style={{
                                background: 'transparent',
                                border: 'none',
                                color: 'var(--accent-primary)',
                                fontSize: '0.75rem',
                                fontWeight: 600,
                                cursor: 'pointer',
                                display: 'inline-flex',
                                alignItems: 'center',
                                gap: '2px',
                              }}
                              title="Create new document type"
                            >
                              <Plus size={12} /> New
                            </button>
                          </div>
                          <select
                            className="input"
                            value={selectedType || ''}
                            onChange={(e) => setSelectedType(e.target.value ? parseInt(e.target.value, 10) : undefined)}
                          >
                            <option value="">(None)</option>
                            {documentTypes.map((t) => (
                              <option key={t.id} value={t.id}>
                                {t.name}
                              </option>
                            ))}
                          </select>
                        </div>
                      </div>

                      {/* Storage Path */}
                      <div>
                        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '6px' }}>
                          <label style={{ fontSize: '0.8rem', fontWeight: 600, color: 'var(--text-secondary)' }}>
                            Storage Path
                          </label>
                          <button
                            type="button"
                            onClick={handleCreateStoragePath}
                            style={{
                              background: 'transparent',
                              border: 'none',
                              color: 'var(--accent-primary)',
                              fontSize: '0.75rem',
                              fontWeight: 600,
                              cursor: 'pointer',
                              display: 'inline-flex',
                              alignItems: 'center',
                              gap: '2px',
                            }}
                            title="Create new storage path"
                          >
                            <Plus size={12} /> New
                          </button>
                        </div>
                        <select
                          className="input"
                          value={selectedStoragePath || ''}
                          onChange={(e) => setSelectedStoragePath(e.target.value ? parseInt(e.target.value, 10) : undefined)}
                        >
                          <option value="">(Default Vault Layout)</option>
                          {storagePaths.map((sp) => (
                            <option key={sp.id} value={sp.id}>
                              {sp.name} ({sp.path})
                            </option>
                          ))}
                        </select>
                      </div>

                      {/* Tags */}
                      <div>
                        <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: 'var(--text-secondary)', marginBottom: '8px' }}>
                          Tags
                        </label>
                        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px' }}>
                          {tags.map((t) => {
                            const isSelected = selectedTags.includes(t.id)
                            return (
                              <button
                                key={t.id}
                                type="button"
                                onClick={() => toggleTag(t.id)}
                                className="badge"
                                style={{
                                  background: isSelected ? t.color || 'var(--accent-primary)' : 'rgba(255,255,255,0.06)',
                                  color: isSelected ? '#fff' : 'var(--text-secondary)',
                                  border: `1px solid ${isSelected ? 'transparent' : 'var(--border-subtle)'}`,
                                  cursor: 'pointer',
                                  padding: '5px 11px',
                                  display: 'inline-flex',
                                  alignItems: 'center',
                                  gap: '5px',
                                  borderRadius: 'var(--radius-full)',
                                  transition: 'all 0.15s ease',
                                }}
                              >
                                <TagIcon size={12} />
                                {t.name}
                                {isSelected && <span style={{ opacity: 0.7, fontSize: '0.75rem', marginLeft: '2px' }}>×</span>}
                              </button>
                            )
                          })}
                        </div>
                      </div>

                      {/* Custom Fields (Amount, Due Date, etc.) */}
                      {customFieldsDef.length > 0 && (
                        <div>
                          <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: 'var(--text-secondary)', marginBottom: '8px' }}>
                            Custom Fields
                          </label>
                          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '12px' }}>
                            {customFieldsDef.map((cf) => {
                              const currentVal = customFieldValues[cf.id] ?? ''
                              return (
                                <div
                                  key={cf.id}
                                  style={{
                                    padding: '10px 14px',
                                    borderRadius: 'var(--radius-md)',
                                    backgroundColor: 'var(--bg-tertiary)',
                                    border: '1px solid var(--border-subtle)',
                                  }}
                                >
                                  <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: 600, color: 'var(--text-muted)', marginBottom: '4px' }}>
                                    {cf.name}
                                  </label>
                                  {cf.data_type === 'date' ? (
                                    <input
                                      type="date"
                                      className="input"
                                      style={{ height: '32px', fontSize: '0.85rem' }}
                                      value={currentVal ? String(currentVal).split('T')[0] : ''}
                                      onChange={(e) =>
                                        setCustomFieldValues((prev) => ({ ...prev, [cf.id]: e.target.value }))
                                      }
                                    />
                                  ) : cf.data_type === 'boolean' ? (
                                    <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer', height: '32px' }}>
                                      <input
                                        type="checkbox"
                                        checked={Boolean(currentVal)}
                                        onChange={(e) =>
                                          setCustomFieldValues((prev) => ({ ...prev, [cf.id]: e.target.checked }))
                                        }
                                        style={{ width: '16px', height: '16px', cursor: 'pointer' }}
                                      />
                                      <span style={{ fontSize: '0.82rem', color: currentVal ? '#10b981' : 'var(--text-muted)', fontWeight: 600 }}>
                                        {currentVal ? '✓ Yes / Active' : '✗ No / Inactive'}
                                      </span>
                                    </label>
                                  ) : cf.data_type === 'url' ? (
                                    <div style={{ display: 'flex', gap: '6px' }}>
                                      <input
                                        type="url"
                                        className="input"
                                        style={{ height: '32px', fontSize: '0.85rem', flex: 1 }}
                                        placeholder="https://..."
                                        value={currentVal}
                                        onChange={(e) =>
                                          setCustomFieldValues((prev) => ({ ...prev, [cf.id]: e.target.value }))
                                        }
                                      />
                                      {currentVal && (
                                        <a
                                          href={currentVal}
                                          target="_blank"
                                          rel="noopener noreferrer"
                                          className="btn btn-secondary"
                                          style={{ padding: '0 8px', display: 'flex', alignItems: 'center', height: '32px' }}
                                          title="Open link in new tab"
                                        >
                                          <ExternalLink size={13} />
                                        </a>
                                      )}
                                    </div>
                                  ) : cf.data_type === 'documentlink' ? (
                                    <div style={{ display: 'flex', gap: '6px' }}>
                                      <input
                                        type="number"
                                        className="input"
                                        style={{ height: '32px', fontSize: '0.85rem', flex: 1 }}
                                        placeholder="Doc ID #"
                                        value={currentVal || ''}
                                        onChange={(e) =>
                                          setCustomFieldValues((prev) => ({ ...prev, [cf.id]: e.target.value ? parseInt(e.target.value, 10) : '' }))
                                        }
                                      />
                                      {currentVal && onNavigate && (
                                        <button
                                          type="button"
                                          className="btn btn-secondary"
                                          style={{ padding: '0 8px', height: '32px', fontSize: '0.75rem', display: 'flex', alignItems: 'center', gap: '4px' }}
                                          onClick={() => onNavigate(parseInt(currentVal, 10))}
                                          title={`Navigate to document #${currentVal}`}
                                        >
                                          <LinkIcon size={12} /> #{currentVal}
                                        </button>
                                      )}
                                    </div>
                                  ) : cf.data_type === 'integer' || cf.data_type === 'float' ? (
                                    <input
                                      type="number"
                                      className="input"
                                      style={{ height: '32px', fontSize: '0.85rem' }}
                                      value={currentVal}
                                      onChange={(e) =>
                                        setCustomFieldValues((prev) => ({ ...prev, [cf.id]: parseFloat(e.target.value) || 0 }))
                                      }
                                    />
                                  ) : (
                                    <input
                                      type="text"
                                      className="input"
                                      style={{ height: '32px', fontSize: '0.85rem' }}
                                      placeholder={cf.data_type === 'monetary' ? '$0.00' : 'Value...'}
                                      value={currentVal}
                                      onChange={(e) =>
                                        setCustomFieldValues((prev) => ({ ...prev, [cf.id]: e.target.value }))
                                      }
                                    />
                                  )}
                                </div>
                              )
                            })}
                          </div>
                        </div>
                      )}

                      {/* Save Footer Bar inside form */}
                      <div
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'flex-end',
                          gap: '10px',
                          marginTop: '12px',
                          paddingTop: '16px',
                          borderTop: '1px solid var(--border-subtle)',
                        }}
                      >
                        <button
                          type="button"
                          className="btn btn-secondary"
                          onClick={handleDiscard}
                        >
                          Discard
                        </button>
                        <button
                          type="button"
                          className="btn btn-primary"
                          onClick={handleSave}
                          disabled={isSaving}
                        >
                          {savedSuccess ? '✓ Saved!' : isSaving ? 'Saving...' : 'Save Changes'}
                        </button>
                      </div>
                    </div>
                  )}

                  {/* CONTENT / OCR TEXT TAB */}
                  {activeTab === 'content' && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                        <span style={{ fontSize: '0.8rem', fontWeight: 600, color: 'var(--text-muted)' }}>
                          EXTRACTED OCR TEXT ({doc.content?.length || 0} characters)
                        </span>
                        <button
                          className="btn btn-secondary"
                          onClick={handleCopyContent}
                          style={{ fontSize: '0.78rem', padding: '4px 10px', display: 'flex', alignItems: 'center', gap: '5px' }}
                        >
                          {copiedContent ? <Check size={13} style={{ color: '#10b981' }} /> : <Copy size={13} />}
                          {copiedContent ? 'Copied!' : 'Copy Text'}
                        </button>
                      </div>
                      <div
                        style={{
                          padding: '16px',
                          backgroundColor: 'var(--bg-tertiary)',
                          borderRadius: 'var(--radius-md)',
                          border: '1px solid var(--border-subtle)',
                          minHeight: '380px',
                          fontSize: '0.85rem',
                          fontFamily: 'var(--font-mono, "SFMono-Regular", Consolas, monospace)',
                          color: 'var(--text-primary)',
                          whiteSpace: 'pre-wrap',
                          lineHeight: 1.6,
                        }}
                      >
                        {doc.content || 'No text extracted from document.'}
                      </div>
                    </div>
                  )}

                  {/* ASK AI TAB */}
                  {activeTab === 'ai_chat' && (
                    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', minHeight: '480px', gap: '16px' }}>
                      <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                        {[
                          'Summarize document',
                          'Extract key dates & deadlines',
                          'List all monetary amounts & totals',
                          'What are the key action items?',
                        ].map((prompt) => (
                          <button
                            key={prompt}
                            onClick={() => handleSendChat(prompt)}
                            disabled={chatStreaming}
                            className="btn btn-secondary"
                            style={{
                              fontSize: '0.75rem',
                              padding: '4px 10px',
                              borderRadius: '12px',
                              display: 'flex',
                              alignItems: 'center',
                              gap: '4px',
                            }}
                          >
                            <Sparkles size={12} style={{ color: 'var(--accent-primary)' }} />
                            {prompt}
                          </button>
                        ))}
                      </div>

                      <div
                        style={{
                          flex: 1,
                          overflowY: 'auto',
                          borderRadius: 'var(--radius-md)',
                          backgroundColor: 'var(--bg-tertiary)',
                          border: '1px solid var(--border-subtle)',
                          padding: '16px',
                          display: 'flex',
                          flexDirection: 'column',
                          gap: '12px',
                          minHeight: '280px',
                        }}
                      >
                        {chatMessages.length === 0 ? (
                          <div
                            style={{
                              display: 'flex',
                              flexDirection: 'column',
                              alignItems: 'center',
                              justifyContent: 'center',
                              height: '100%',
                              color: 'var(--text-muted)',
                              textAlign: 'center',
                              padding: '32px 16px',
                            }}
                          >
                            <Sparkles size={32} style={{ color: 'var(--accent-primary)', marginBottom: '12px', opacity: 0.8 }} />
                            <div style={{ fontWeight: 600, color: 'var(--text-primary)', marginBottom: '4px' }}>
                              Chat with this document
                            </div>
                            <div style={{ fontSize: '0.85rem', maxWidth: '320px', lineHeight: 1.4 }}>
                              Ask questions, extract information, or request a summary using FileDeck's local AI intelligence.
                            </div>
                          </div>
                        ) : (
                          chatMessages.map((msg, i) => (
                            <div
                              key={i}
                              style={{
                                display: 'flex',
                                gap: '10px',
                                alignItems: 'flex-start',
                                flexDirection: msg.role === 'user' ? 'row-reverse' : 'row',
                              }}
                            >
                              <div
                                style={{
                                  width: '28px',
                                  height: '28px',
                                  borderRadius: '50%',
                                  backgroundColor: msg.role === 'user' ? 'var(--accent-primary)' : 'rgba(255,255,255,0.08)',
                                  display: 'flex',
                                  alignItems: 'center',
                                  justifyContent: 'center',
                                  flexShrink: 0,
                                }}
                              >
                                {msg.role === 'user' ? <User size={14} /> : <Bot size={14} style={{ color: 'var(--accent-primary)' }} />}
                              </div>
                              <div
                                style={{
                                  maxWidth: '82%',
                                  padding: '10px 14px',
                                  borderRadius: 'var(--radius-sm)',
                                  backgroundColor:
                                    msg.role === 'user'
                                      ? 'var(--accent-soft)'
                                      : 'var(--bg-surface)',
                                  border: `1px solid ${msg.role === 'user' ? 'var(--accent-border)' : 'var(--border-subtle)'}`,
                                  fontSize: '0.88rem',
                                  lineHeight: 1.5,
                                  whiteSpace: 'pre-wrap',
                                  color: 'var(--text-primary)',
                                }}
                              >
                                {msg.content || (chatStreaming && i === chatMessages.length - 1 ? (
                                  <span style={{ color: 'var(--text-muted)', fontStyle: 'italic' }}>Thinking...</span>
                                ) : '')}
                              </div>
                            </div>
                          ))
                        )}
                      </div>

                      <form
                        onSubmit={(e) => {
                          e.preventDefault()
                          handleSendChat()
                        }}
                        style={{ display: 'flex', gap: '8px' }}
                      >
                        <input
                          type="text"
                          className="input"
                          placeholder="Ask a question about this document..."
                          value={chatInput}
                          onChange={(e) => setChatInput(e.target.value)}
                          disabled={chatStreaming}
                          style={{ flex: 1 }}
                        />
                        {chatStreaming ? (
                          <button
                            type="button"
                            className="btn btn-secondary"
                            onClick={handleStopChat}
                            style={{ display: 'flex', alignItems: 'center', gap: '6px', color: '#ef4444', borderColor: 'rgba(239, 68, 68, 0.4)' }}
                          >
                            <Square size={14} fill="#ef4444" /> Stop
                          </button>
                        ) : (
                          <button
                            type="submit"
                            className="btn btn-primary"
                            disabled={!chatInput.trim()}
                            style={{ display: 'flex', alignItems: 'center', gap: '6px' }}
                          >
                            <Send size={16} /> Send
                          </button>
                        )}
                      </form>
                    </div>
                  )}

                  {/* TABLES TAB */}
                  {activeTab === 'tables' && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                      {tablesLoading ? (
                        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', padding: '48px', color: 'var(--text-muted)' }}>
                          Extracting tabular data from document...
                        </div>
                      ) : tables.length === 0 ? (
                        <div
                          style={{
                            padding: '36px',
                            textAlign: 'center',
                            color: 'var(--text-muted)',
                            backgroundColor: 'var(--bg-tertiary)',
                            borderRadius: 'var(--radius-md)',
                            border: '1px solid var(--border-subtle)',
                          }}
                        >
                          <Table size={28} style={{ opacity: 0.5, marginBottom: '8px' }} />
                          <div>No structured tables detected in this document.</div>
                        </div>
                      ) : (
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                          <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                            Found {tables.length} structured {tables.length === 1 ? 'table' : 'tables'} in document:
                          </div>
                          {tables.map((t, tIdx) => (
                            <div
                              key={tIdx}
                              style={{
                                borderRadius: 'var(--radius-md)',
                                border: '1px solid var(--border-subtle)',
                                backgroundColor: 'var(--bg-tertiary)',
                                overflow: 'hidden',
                              }}
                            >
                              <div
                                style={{
                                  display: 'flex',
                                  justifyContent: 'space-between',
                                  alignItems: 'center',
                                  padding: '10px 14px',
                                  backgroundColor: 'rgba(255,255,255,0.03)',
                                  borderBottom: '1px solid var(--border-subtle)',
                                }}
                              >
                                <span style={{ fontSize: '0.82rem', fontWeight: 600 }}>
                                  Table #{t.index + 1} (Page {t.page})
                                </span>
                                <button
                                  onClick={() => copyTableAsCsv(t.rows, tIdx)}
                                  className="btn btn-secondary"
                                  style={{ fontSize: '0.75rem', padding: '4px 8px', display: 'flex', alignItems: 'center', gap: '4px' }}
                                >
                                  {copiedTableIdx === tIdx ? <Check size={12} style={{ color: 'var(--success)' }} /> : <Copy size={12} />}
                                  {copiedTableIdx === tIdx ? 'Copied CSV' : 'Copy CSV'}
                                </button>
                              </div>
                              <div style={{ overflowX: 'auto', padding: '8px' }}>
                                <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '0.8rem' }}>
                                  <tbody>
                                    {t.rows.map((row, rIdx) => (
                                      <tr
                                        key={rIdx}
                                        style={{
                                          backgroundColor: rIdx === 0 ? 'rgba(255,255,255,0.05)' : rIdx % 2 === 0 ? 'rgba(255,255,255,0.015)' : 'transparent',
                                          fontWeight: rIdx === 0 ? 600 : 400,
                                        }}
                                      >
                                        {row.map((cell, cIdx) => (
                                          <td
                                            key={cIdx}
                                            style={{
                                              padding: '8px 12px',
                                              borderBottom: '1px solid rgba(255,255,255,0.05)',
                                              borderRight: cIdx < row.length - 1 ? '1px solid rgba(255,255,255,0.03)' : 'none',
                                            }}
                                          >
                                            {cell || '—'}
                                          </td>
                                        ))}
                                      </tr>
                                    ))}
                                  </tbody>
                                </table>
                              </div>
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  )}

                  {/* VERSIONS TAB */}
                  {activeTab === 'versions' && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                      <div
                        className="card-panel"
                        style={{
                          padding: '16px',
                          display: 'flex',
                          flexDirection: 'column',
                          gap: '12px',
                          backgroundColor: 'var(--bg-tertiary)',
                        }}
                      >
                        <div style={{ fontWeight: 700, fontSize: '0.9rem', color: 'var(--text-primary)' }}>
                          Upload New Version
                        </div>
                        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
                          <input
                            type="file"
                            className="input"
                            onChange={(e) => setNewVersionFile(e.target.files?.[0] || null)}
                          />
                          <input
                            type="text"
                            className="input"
                            placeholder="Version label (e.g. v2 - signed)"
                            value={versionLabel}
                            onChange={(e) => setVersionLabel(e.target.value)}
                          />
                        </div>
                        <button
                          className="btn btn-primary"
                          onClick={handleUploadVersion}
                          disabled={uploadingVersion || !newVersionFile}
                          style={{ alignSelf: 'flex-end', display: 'flex', alignItems: 'center', gap: '6px' }}
                        >
                          <Upload size={15} />
                          {uploadingVersion ? 'Uploading...' : 'Upload Version'}
                        </button>
                      </div>

                      <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
                        <div style={{ fontSize: '0.8rem', fontWeight: 700, color: 'var(--text-muted)', textTransform: 'uppercase' }}>
                          Version History
                        </div>
                        {doc.versions && doc.versions.length > 0 ? (
                          doc.versions.map((v) => (
                            <div
                              key={v.id}
                              style={{
                                padding: '14px 16px',
                                borderRadius: 'var(--radius-md)',
                                backgroundColor: v.is_current ? 'var(--accent-soft)' : 'var(--bg-tertiary)',
                                border: `1px solid ${v.is_current ? 'var(--accent-primary)' : 'var(--border-subtle)'}`,
                                display: 'flex',
                                alignItems: 'center',
                                justifyContent: 'space-between',
                              }}
                            >
                              <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
                                <Layers size={18} color={v.is_current ? 'var(--accent-primary)' : 'var(--text-muted)'} />
                                <div>
                                  <div style={{ fontWeight: 600, fontSize: '0.9rem' }}>
                                    {v.version_label || `Version #${v.version_index}`}
                                    {v.is_current && (
                                      <span
                                        style={{
                                          marginLeft: '8px',
                                          fontSize: '0.7rem',
                                          padding: '2px 6px',
                                          borderRadius: '4px',
                                          backgroundColor: 'var(--accent-primary)',
                                          color: '#fff',
                                        }}
                                      >
                                        Current
                                      </span>
                                    )}
                                  </div>
                                  <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)', marginTop: '2px' }}>
                                    Added: {new Date(v.added).toLocaleString()} · SHA: {v.checksum.slice(0, 10)}
                                  </div>
                                </div>
                              </div>

                              {!v.is_current && (
                                <button
                                  className="btn btn-secondary"
                                  onClick={() => fetchDoc(v.id)}
                                  style={{ padding: '4px 10px', fontSize: '0.8rem' }}
                                >
                                  Switch View
                                </button>
                              )}
                            </div>
                          ))
                        ) : (
                          <div style={{ color: 'var(--text-muted)', fontSize: '0.85rem' }}>
                            This is the only version of this document.
                          </div>
                        )}
                      </div>
                    </div>
                  )}

                  {/* HISTORY TAB */}
                  {activeTab === 'history' && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--text-muted)', fontSize: '0.85rem' }}>
                        <Activity size={16} />
                        <span>Audit Trail Log</span>
                      </div>

                      {historyLoading ? (
                        <div style={{ padding: '24px', textAlign: 'center', color: 'var(--text-muted)' }}>
                          Loading audit entries...
                        </div>
                      ) : historyItems.length === 0 ? (
                        <div style={{ padding: '24px', textAlign: 'center', color: 'var(--text-muted)', fontSize: '0.85rem' }}>
                          No audit history entries recorded for this document yet.
                        </div>
                      ) : (
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
                          {historyItems.map((h, i) => (
                            <div
                              key={h.id || i}
                              style={{
                                padding: '12px 16px',
                                borderRadius: 'var(--radius-md)',
                                backgroundColor: 'var(--bg-tertiary)',
                                border: '1px solid var(--border-subtle)',
                                display: 'flex',
                                flexDirection: 'column',
                                gap: '6px',
                              }}
                            >
                              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                                  <span
                                    style={{
                                      fontSize: '0.75rem',
                                      fontWeight: 700,
                                      textTransform: 'uppercase',
                                      padding: '2px 6px',
                                      borderRadius: '4px',
                                      backgroundColor:
                                        h.action === 'create'
                                          ? 'rgba(16, 185, 129, 0.15)'
                                          : h.action === 'delete'
                                          ? 'rgba(239, 68, 68, 0.15)'
                                          : 'rgba(59, 130, 246, 0.15)',
                                      color:
                                        h.action === 'create'
                                          ? '#10b981'
                                          : h.action === 'delete'
                                          ? '#ef4444'
                                          : '#3b82f6',
                                    }}
                                  >
                                    {h.action}
                                  </span>
                                  <span style={{ fontSize: '0.85rem', fontWeight: 600 }}>
                                    {formatActor(h.actor)}
                                  </span>
                                </div>
                                <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                                  {new Date(h.timestamp).toLocaleString()}
                                </span>
                              </div>

                              {Boolean(
                                (h.changes && Object.keys(h.changes).length > 0) ||
                                (h.details && Object.keys(h.details).length > 0)
                              ) && (
                                <div
                                  style={{
                                    marginTop: '4px',
                                    padding: '8px 12px',
                                    borderRadius: '4px',
                                    backgroundColor: 'rgba(0,0,0,0.3)',
                                    fontSize: '0.8rem',
                                    fontFamily: 'monospace',
                                    color: 'var(--text-secondary)',
                                  }}
                                >
                                  {JSON.stringify(h.changes || h.details, null, 2)}
                                </div>
                              )}
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  )}

                  {/* NOTES TAB */}
                  {activeTab === 'notes' && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                      <div style={{ display: 'flex', gap: '8px' }}>
                        <input
                          type="text"
                          className="input"
                          placeholder="Add a new note..."
                          value={newNote}
                          onChange={(e) => setNewNote(e.target.value)}
                          onKeyDown={(e) => e.key === 'Enter' && handleAddNote()}
                        />
                        <button className="btn btn-primary" onClick={handleAddNote}>
                          <Plus size={16} /> Add
                        </button>
                      </div>

                      <div style={{ display: 'flex', flexDirection: 'column', gap: '10px', marginTop: '8px' }}>
                        {doc.notes.length === 0 ? (
                          <div style={{ color: 'var(--text-muted)', fontSize: '0.85rem' }}>No notes on this document yet.</div>
                        ) : (
                          doc.notes.map((n) => (
                            <div
                              key={n.id}
                              style={{
                                padding: '12px 16px',
                                borderRadius: 'var(--radius-md)',
                                backgroundColor: 'var(--bg-tertiary)',
                                border: '1px solid var(--border-subtle)',
                                display: 'flex',
                                justifyContent: 'space-between',
                                alignItems: 'flex-start',
                              }}
                            >
                              <div>
                                <div style={{ fontSize: '0.9rem', marginBottom: '4px' }}>{n.note}</div>
                                <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                                  {new Date(n.created).toLocaleString()}
                                </div>
                              </div>
                              <button
                                onClick={() => handleDeleteNote(n.id)}
                                style={{
                                  background: 'transparent',
                                  border: 'none',
                                  color: 'var(--text-muted)',
                                  cursor: 'pointer',
                                }}
                              >
                                <Trash2 size={14} />
                              </button>
                            </div>
                          ))
                        )}
                      </div>
                    </div>
                  )}
                </>
              )}
            </div>
          </div>

          {/* ======================================================== */}
          {/* RIGHT COLUMN: Permanent Side-by-Side PDF Document Viewer */}
          {/* ======================================================== */}
          <div
            style={{
              flex: 1,
              display: 'flex',
              flexDirection: 'column',
              backgroundColor: '#1e2022',
              overflow: 'hidden',
              position: 'relative',
            }}
          >
            {/* PDF Viewer Header Bar */}
            <div
              style={{
                padding: '10px 18px',
                backgroundColor: 'var(--bg-tertiary)',
                borderBottom: '1px solid var(--border-subtle)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                fontSize: '0.82rem',
                color: 'var(--text-secondary)',
                flexShrink: 0,
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <Eye size={15} style={{ color: 'var(--accent-primary)' }} />
                <span style={{ fontWeight: 600 }}>Live Document Viewer</span>
                {doc?.page_count && doc.page_count > 0 && (
                  <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                    • {doc.page_count} {doc.page_count === 1 ? 'page' : 'pages'}
                  </span>
                )}
              </div>

              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <button
                  type="button"
                  className="btn btn-secondary"
                  onClick={() => setPreviewKey((k) => k + 1)}
                  style={{ padding: '4px 8px', fontSize: '0.75rem' }}
                  title="Reload Preview"
                >
                  <RotateCw size={13} />
                </button>
                <a
                  href={`/api/documents/${doc?.id}/preview/`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="btn btn-secondary"
                  style={{ padding: '4px 10px', fontSize: '0.75rem', display: 'flex', alignItems: 'center', gap: '4px' }}
                  title="Open Preview in New Tab"
                >
                  <ExternalLink size={13} /> Open Tab
                </a>
              </div>
            </div>

            {/* Live Document Iframe */}
            <div style={{ flex: 1, position: 'relative', width: '100%', height: '100%' }}>
              {doc ? (
                <iframe
                  key={previewKey}
                  src={`/api/documents/${doc.id}/preview/#toolbar=1`}
                  title={`Preview of ${doc.title}`}
                  style={{
                    width: '100%',
                    height: '100%',
                    border: 'none',
                    backgroundColor: '#1e2022',
                  }}
                />
              ) : (
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--text-muted)' }}>
                  Loading document preview...
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Share Link Modal */}
        {shareModalOpen && (
          <div
            style={{
              position: 'fixed',
              inset: 0,
              backgroundColor: 'rgba(0, 0, 0, 0.65)',
              zIndex: 1100,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
            onClick={() => setShareModalOpen(false)}
          >
            <div
              className="card-panel"
              style={{
                width: '460px',
                padding: '24px',
                borderRadius: 'var(--radius-lg)',
                backgroundColor: 'var(--bg-secondary)',
                border: '1px solid var(--border-subtle)',
                boxShadow: '0 20px 50px rgba(0, 0, 0, 0.5)',
              }}
              onClick={(e) => e.stopPropagation()}
            >
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '16px' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <Share2 size={18} style={{ color: 'var(--accent-primary)' }} />
                  <h3 style={{ margin: 0, fontSize: '1.1rem', fontWeight: 700 }}>Public Share Link</h3>
                </div>
                <button
                  type="button"
                  onClick={() => setShareModalOpen(false)}
                  style={{ background: 'transparent', border: 'none', color: 'var(--text-muted)', cursor: 'pointer' }}
                >
                  <X size={18} />
                </button>
              </div>

              <p style={{ fontSize: '0.84rem', color: 'var(--text-secondary)', marginBottom: '16px', lineHeight: 1.4 }}>
                Generate an unauthenticated read-only link for <strong>{doc?.title}</strong>. Anyone with this link can view the document preview.
              </p>

              {!shareSlug ? (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
                  <div>
                    <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: 'var(--text-muted)', marginBottom: '6px' }}>
                      Expiration Duration
                    </label>
                    <select
                      className="input"
                      value={shareExpiryHours}
                      onChange={(e) => setShareExpiryHours(parseInt(e.target.value, 10))}
                    >
                      <option value={1}>1 hour</option>
                      <option value={24}>24 hours (1 day)</option>
                      <option value={72}>72 hours (3 days)</option>
                      <option value={168}>7 days</option>
                      <option value={720}>30 days</option>
                    </select>
                  </div>

                  <button
                    type="button"
                    className="btn btn-primary"
                    onClick={handleCreateShareLink}
                    disabled={shareLoading}
                    style={{ width: '100%', justifyContent: 'center', padding: '10px' }}
                  >
                    {shareLoading ? 'Generating Link...' : 'Create Share Link'}
                  </button>
                </div>
              ) : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '14px' }}>
                  <div>
                    <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: 'var(--text-muted)', marginBottom: '6px' }}>
                      Shareable URL
                    </label>
                    <div style={{ display: 'flex', gap: '6px' }}>
                      <input
                        type="text"
                        readOnly
                        className="input"
                        value={`${window.location.origin}/share/${shareSlug}/`}
                        style={{ fontSize: '0.82rem', fontFamily: 'monospace' }}
                      />
                      <button
                        type="button"
                        className="btn btn-secondary"
                        onClick={handleCopyShareLink}
                        style={{ display: 'flex', alignItems: 'center', gap: '4px', padding: '0 12px' }}
                      >
                        {shareCopied ? <Check size={14} color="#10b981" /> : <Copy size={14} />}
                        {shareCopied ? 'Copied' : 'Copy'}
                      </button>
                    </div>
                  </div>

                  <div style={{ display: 'flex', gap: '8px', justifyContent: 'flex-end', marginTop: '8px' }}>
                    <a
                      href={`/share/${shareSlug}/`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="btn btn-secondary"
                      style={{ fontSize: '0.82rem', display: 'flex', alignItems: 'center', gap: '4px' }}
                    >
                      <ExternalLink size={13} /> Open Page
                    </a>
                    <button
                      type="button"
                      className="btn btn-primary"
                      onClick={() => setShareModalOpen(false)}
                      style={{ fontSize: '0.82rem' }}
                    >
                      Done
                    </button>
                  </div>
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
