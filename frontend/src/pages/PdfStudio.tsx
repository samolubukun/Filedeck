import React, { useState, useEffect } from 'react'
import {
  Wrench,
  Files,
  Split,
  Trash2,
  Layers,
  Minimize2,
  FileCheck,
  Search,
  FileImage,
  FileSpreadsheet,
  FileText,
  FileCode,
  Globe,
  RotateCw,
  Hash,
  Droplet,
  Crop,
  Edit3,
  Tag,
  Lock,
  Unlock,
  PenTool,
  EyeOff,
  GitCompare,
  Stamp,
  Download,
  Save,
  RefreshCw,
  ArrowLeft,
  UploadCloud,
  Check,
  AlertCircle,
  Eye,
  Sparkles,
} from 'lucide-react'
import { api } from '../api/client'
import { useDocuments } from '../api/useDocuments'

export interface ToolDef {
  id: string
  name: string
  desc: string
  category: 'organize' | 'optimize' | 'to_pdf' | 'from_pdf' | 'edit' | 'protect'
  icon: any
  color: string
  accept?: string
  multi?: boolean
  minFiles?: number
  viewType?: 'default' | 'organize' | 'crop' | 'edit' | 'sign' | 'redact' | 'metadata'
}

export const PDF_TOOLS: ToolDef[] = [
  // Organise
  { id: 'merge', name: 'Merge PDF', desc: 'Combine multiple PDFs into a single file in any order', category: 'organize', icon: Files, color: '#f87171', multi: true, minFiles: 2 },
  { id: 'split', name: 'Split PDF', desc: 'Split by page ranges, every N pages, or extract specific pages', category: 'organize', icon: Split, color: '#fb923c' },
  { id: 'remove_pages', name: 'Remove Pages', desc: 'Delete unwanted pages from document by page number', category: 'organize', icon: Trash2, color: '#f43f5e' },
  { id: 'organize', name: 'Organise PDF', desc: 'Visual page editor: reorder, rotate, or delete pages with mouse', category: 'organize', icon: Layers, color: '#ec4899', viewType: 'organize' },

  // Optimise
  { id: 'compress', name: 'Compress PDF', desc: 'Reduce file size while preserving reading quality', category: 'optimize', icon: Minimize2, color: '#10b981', multi: true },
  { id: 'repair', name: 'Repair PDF', desc: 'Recover content and fix corrupted or unreadable PDF files', category: 'optimize', icon: FileCheck, color: '#34d399', multi: true },
  { id: 'ocr', name: 'OCR Searchable PDF', desc: 'Extract text from scanned documents to make them selectable & searchable', category: 'optimize', icon: Search, color: '#06b6d4', multi: true },
  { id: 'flatten', name: 'Flatten PDF', desc: 'Bake interactive form fields and annotations into static page canvas', category: 'optimize', icon: Stamp, color: '#a8a29e', multi: true },

  // Convert to PDF
  { id: 'images_to_pdf', name: 'Images to PDF', desc: 'Convert JPG, PNG, HEIC, WebP, or TIFF images into clean PDF', category: 'to_pdf', icon: FileImage, color: '#f59e0b', accept: '.jpg,.jpeg,.png,.webp,.tiff,.tif,.bmp,.gif', multi: true },
  { id: 'office_to_pdf', name: 'Office to PDF', desc: 'Convert DOCX, XLSX, PPTX, or ODT files directly to PDF', category: 'to_pdf', icon: FileText, color: '#3b82f6', accept: '.docx,.doc,.xlsx,.xls,.pptx,.ppt,.odt,.ods,.odp,.rtf', multi: true },
  { id: 'html_to_pdf', name: 'HTML to PDF', desc: 'Render HTML files or website URLs directly to PDF', category: 'to_pdf', icon: Globe, color: '#f97316', accept: '.html,.htm', multi: true },

  // Convert from PDF
  { id: 'pdf_to_images', name: 'PDF to Images', desc: 'Export PDF pages as crisp JPG or PNG images (up to 300 DPI)', category: 'from_pdf', icon: FileImage, color: '#eab308', multi: true },
  { id: 'extract_images', name: 'Extract Images', desc: 'Extract all embedded raster images from the document in original quality', category: 'from_pdf', icon: FileImage, color: '#d97706', multi: true },
  { id: 'pdf_to_word', name: 'PDF to Word', desc: 'Convert PDF into an editable Microsoft Word DOCX file', category: 'from_pdf', icon: FileText, color: '#60a5fa', multi: true },
  { id: 'pdf_to_excel', name: 'PDF to Excel', desc: 'Extract tabular financial data and tables into Excel XLSX', category: 'from_pdf', icon: FileSpreadsheet, color: '#22c55e', multi: true },
  { id: 'pdf_to_pptx', name: 'PDF to PowerPoint', desc: 'Convert PDF slides into an editable presentation deck', category: 'from_pdf', icon: FileSpreadsheet, color: '#f97316', multi: true },
  { id: 'pdf_to_markdown', name: 'PDF to Markdown', desc: 'Convert PDF into structured Markdown for LLM AI context & notes', category: 'from_pdf', icon: FileCode, color: '#94a3b8', multi: true },
  { id: 'pdf_to_text', name: 'PDF to Text', desc: 'Extract all plain text without formatting into TXT', category: 'from_pdf', icon: FileText, color: '#78716c', multi: true },
  { id: 'pdf_to_pdfa', name: 'PDF to PDF/A', desc: 'Convert to ISO archival standard (1b, 2b, 3b) for long-term storage', category: 'from_pdf', icon: FileCheck, color: '#818cf8', multi: true },

  // Edit
  { id: 'rotate', name: 'Rotate PDF', desc: 'Rotate pages 90°, 180°, or 270° (all, odd, even, or custom ranges)', category: 'edit', icon: RotateCw, color: '#ec4899', multi: true },
  { id: 'page_numbers', name: 'Page Numbers', desc: 'Add customizable headers, footers, and page numbers in 9 positions', category: 'edit', icon: Hash, color: '#a855f7' },
  { id: 'watermark', name: 'Watermark PDF', desc: 'Stamp text or image watermarks with angle, opacity, and mosaic repeating', category: 'edit', icon: Droplet, color: '#38bdf8' },
  { id: 'crop', name: 'Crop PDF', desc: 'Trim document margins or crop page dimensions with visual preview', category: 'edit', icon: Crop, color: '#f43f5e', viewType: 'crop' },
  { id: 'edit', name: 'Annotate & Edit', desc: 'Add text boxes, shapes, highlights, lines, and white-out blocks', category: 'edit', icon: Edit3, color: '#c084fc', viewType: 'edit' },
  { id: 'metadata', name: 'Edit Metadata', desc: 'Modify Title, Author, Subject, and Keywords of the PDF file', category: 'edit', icon: Tag, color: '#14b8a6', viewType: 'metadata' },

  // Protect
  { id: 'protect', name: 'Protect PDF', desc: 'Encrypt document with AES-256 password and restrict print/copy permissions', category: 'protect', icon: Lock, color: '#94a3b8', multi: true },
  { id: 'unlock', name: 'Unlock PDF', desc: 'Remove password and encryption from protected documents', category: 'protect', icon: Unlock, color: '#38bdf8', multi: true },
  { id: 'sign', name: 'Sign Document', desc: 'Place drawn, typed, or uploaded signature stamp on any page', category: 'protect', icon: PenTool, color: '#6366f1', viewType: 'sign' },
  { id: 'redact', name: 'Redact Text', desc: 'Permanently cryptographically erase sensitive words and areas', category: 'protect', icon: EyeOff, color: '#64748b', viewType: 'redact' },
  { id: 'compare', name: 'Compare PDFs', desc: 'Generate visual side-by-side report highlighting changes between 2 files', category: 'protect', icon: GitCompare, color: '#2dd4bf', multi: true, minFiles: 2 },
]

const CATEGORIES = [
  { id: 'organize', name: 'Organise' },
  { id: 'optimize', name: 'Optimise' },
  { id: 'to_pdf', name: 'Convert to PDF' },
  { id: 'from_pdf', name: 'Convert from PDF' },
  { id: 'edit', name: 'Edit PDF' },
  { id: 'protect', name: 'Security & Legal' },
]

export const PdfStudio: React.FC = () => {
  const [activeTool, setActiveTool] = useState<ToolDef | null>(null)
  const [searchQuery, setSearchQuery] = useState('')
  const [activeCategory, setActiveCategory] = useState<string>('all')

  // Files in workspace: [{ id, name, size, pages, encrypted, vault_doc_id }]
  const [files, setFiles] = useState<any[]>([])
  const [activePageIndex, setActivePageIndex] = useState(0)
  const [loading, setLoading] = useState(false)
  const [previewLoading, setPreviewLoading] = useState(false)
  const [previewData, setPreviewData] = useState<any | null>(null)
  const [errorMsg, setErrorMsg] = useState<string | null>(null)
  const [successResult, setSuccessResult] = useState<any | null>(null)
  const [saveStatus, setSaveStatus] = useState<string | null>(null)

  // Tool parameter states
  const [options, setOptions] = useState<Record<string, any>>({
    compress_level: 'recommended',
    watermark_text: 'CONFIDENTIAL',
    watermark_opacity: 0.25,
    watermark_rotation: 45,
    watermark_color: '#d53f4a',
    watermark_size: 48,
    watermark_mosaic: false,
    watermark_position: 'mc',
    rotate_angle: 90,
    rotate_apply: 'all',
    page_number_pos: 'bc',
    page_number_fmt: '{n} of {total}',
    page_number_size: 12,
    password: '',
    allow_print: true,
    allow_copy: false,
    dpi: 150,
    img_fmt: 'jpg',
    redact_terms: '',
    html_url: '',
  })

  const { documents } = useDocuments()
  const [showVaultPicker, setShowVaultPicker] = useState(false)

  // Parse deep link query params (e.g. #/pdf-studio?doc=5&tool=watermark)
  useEffect(() => {
    const hash = window.location.hash || ''
    const [_, queryPart] = hash.split('?')
    if (queryPart) {
      const p = new URLSearchParams(queryPart)
      const toolId = p.get('tool')
      const docId = p.get('doc')
      if (toolId) {
        const found = PDF_TOOLS.find((t) => t.id === toolId)
        if (found) setActiveTool(found)
      }
      if (docId) {
        const parsed = parseInt(docId, 10)
        if (!isNaN(parsed)) {
          loadVaultDoc(parsed)
        }
      }
    }
  }, [])

  const loadVaultDoc = async (docId: number) => {
    setLoading(true)
    setErrorMsg(null)
    try {
      const info = await api.loadVaultDocumentToPdfStudio(docId)
      setFiles([info])
      setActivePageIndex(0)
    } catch (e: any) {
      setErrorMsg(`Failed to load vault document #${docId}: ${e.message}`)
    } finally {
      setLoading(false)
    }
  }

  const handleFileUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!e.target.files || e.target.files.length === 0) return
    const uploadedFiles = Array.from(e.target.files)
    setLoading(true)
    setErrorMsg(null)
    try {
      const res = await api.uploadPdfFiles(uploadedFiles)
      if (activeTool?.multi) {
        setFiles((prev) => [...prev, ...(res.files || [])])
      } else {
        setFiles(res.files || [])
      }
      setActivePageIndex(0)
    } catch (err: any) {
      setErrorMsg(`Upload failed: ${err.message}`)
    } finally {
      setLoading(false)
    }
  }

  // Trigger live preview when files or options change
  useEffect(() => {
    if (!activeTool || files.length === 0) {
      setPreviewData(null)
      return
    }

    const timer = setTimeout(async () => {
      setPreviewLoading(true)
      try {
        const payload = {
          files: files.map((f) => f.id),
          options: {
            ...options,
            level: options.compress_level,
            text: options.watermark_text,
            opacity: options.watermark_opacity,
            rotation: options.watermark_rotation,
            color: options.watermark_color,
            size: options.watermark_size,
            mosaic: options.watermark_mosaic,
            position: options.watermark_position || options.page_number_pos,
            angle: options.rotate_angle,
            apply: options.rotate_apply,
            format: options.page_number_fmt,
            terms: options.redact_terms ? options.redact_terms.split(',').map((s: string) => s.trim()) : [],
            url: options.html_url,
          },
        }
        const res = await api.previewPdfTool(activeTool.id, payload)
        setPreviewData(res)
      } catch (err) {
        console.error('Preview error', err)
      } finally {
        setPreviewLoading(false)
      }
    }, 350)

    return () => clearTimeout(timer)
  }, [activeTool, files, options])

  const handleRunTool = async () => {
    if (!activeTool) return
    setLoading(true)
    setErrorMsg(null)
    setSaveStatus(null)
    try {
      const payload = {
        files: files.map((f) => f.id),
        options: {
          ...options,
          level: options.compress_level,
          text: options.watermark_text,
          opacity: options.watermark_opacity,
          rotation: options.watermark_rotation,
          color: options.watermark_color,
          size: options.watermark_size,
          mosaic: options.watermark_mosaic,
          position: options.watermark_position || options.page_number_pos,
          angle: options.rotate_angle,
          apply: options.rotate_apply,
          format: options.page_number_fmt,
          terms: options.redact_terms ? options.redact_terms.split(',').map((s: string) => s.trim()) : [],
          url: options.html_url,
        },
      }
      const res = await api.runPdfTool(activeTool.id, payload)
      setSuccessResult(res)
    } catch (err: any) {
      setErrorMsg(err.message || 'Operation failed')
    } finally {
      setLoading(false)
    }
  }

  const handleSaveAsNewDoc = async (outputId: string, outputName: string) => {
    setSaveStatus('Saving to vault...')
    try {
      await api.savePdfAsNewDocument({
        output_id: outputId,
        title: outputName.replace(/\.pdf$/i, ''),
      })
      setSaveStatus('Successfully saved as new document!')
    } catch (e: any) {
      setSaveStatus(`Failed to save: ${e.message}`)
    }
  }

  const handleSaveAsVersion = async (outputId: string) => {
    const rootDocId = files[0]?.vault_doc_id
    if (!rootDocId) return
    setSaveStatus('Saving new version to vault...')
    try {
      await api.savePdfAsVersion(rootDocId, {
        output_id: outputId,
        version_label: `${activeTool?.name || 'Modified'} Version`,
      })
      setSaveStatus('Successfully saved as new version in vault!')
    } catch (e: any) {
      setSaveStatus(`Failed to save version: ${e.message}`)
    }
  }

  const filteredTools = PDF_TOOLS.filter((t) => {
    const matchesCat = activeCategory === 'all' || t.category === activeCategory
    const matchesSearch =
      !searchQuery ||
      t.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      t.desc.toLowerCase().includes(searchQuery.toLowerCase())
    return matchesCat && matchesSearch
  })

  const previewFileId = previewData?.outputs?.[0]?.id || files[0]?.id
  const currentTotalPages = files[0]?.pages || 1

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
      {/* Top Header */}
      <header
        style={{
          padding: '16px 24px',
          borderBottom: '1px solid var(--border-subtle)',
          backgroundColor: 'var(--bg-secondary)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          flexShrink: 0,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '14px' }}>
          {activeTool ? (
            <button
              className="btn btn-secondary"
              onClick={() => {
                setActiveTool(null)
                setSuccessResult(null)
                setPreviewData(null)
              }}
              style={{ display: 'flex', alignItems: 'center', gap: '6px', padding: '6px 12px' }}
            >
              <ArrowLeft size={16} /> All Tools
            </button>
          ) : (
            <div
              style={{
                width: '36px',
                height: '36px',
                borderRadius: '8px',
                backgroundColor: 'rgba(59, 130, 246, 0.15)',
                color: 'var(--accent-primary)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
              }}
            >
              <Wrench size={20} />
            </div>
          )}

          <div>
            <h1 style={{ margin: 0, fontSize: '1.25rem', fontWeight: 700, display: 'flex', alignItems: 'center', gap: '8px' }}>
              {activeTool ? activeTool.name : 'PDF Studio'}
            </h1>
            <p style={{ margin: 0, fontSize: '0.8rem', color: 'var(--text-muted)' }}>
              {activeTool ? activeTool.desc : '30 professional PDF tools running entirely on your machine.'}
            </p>
          </div>
        </div>

        {/* Global Action Right */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          {activeTool && files.length > 0 && (
            <button
              className="btn btn-primary"
              onClick={handleRunTool}
              disabled={loading}
              style={{ display: 'flex', alignItems: 'center', gap: '8px', padding: '8px 18px', fontWeight: 600 }}
            >
              {loading ? <RefreshCw size={16} className="animate-spin" /> : <Sparkles size={16} />}
              Apply {activeTool.name}
            </button>
          )}
        </div>
      </header>

      {/* Main Workspace Body */}
      <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
        {/* VIEW 1: TOOL CATALOG */}
        {!activeTool ? (
          <div style={{ flex: 1, overflowY: 'auto', padding: '16px 8px 32px' }}>
            {/* Search and Category Filter Header */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: '14px', marginBottom: '24px' }}>
              {/* Row 1: Search Input & Tool Counter */}
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: '16px' }}>
                <div style={{ position: 'relative', width: '340px', maxWidth: '100%' }}>
                  <Search size={16} style={{ position: 'absolute', left: '12px', top: '10px', color: 'var(--text-muted)' }} />
                  <input
                    type="text"
                    className="input"
                    placeholder="Search 30 PDF tools (e.g. merge, compress, ocr, watermark)..."
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    style={{ paddingLeft: '36px', width: '100%', fontSize: '0.85rem' }}
                  />
                </div>
                <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)', fontWeight: 500 }}>
                  Showing <span style={{ fontWeight: 700, color: 'var(--text-primary)' }}>{filteredTools.length}</span> of 30 tools
                </div>
              </div>

              {/* Row 2: Category Filter Pills In One Clean Line */}
              <div
                style={{
                  display: 'flex',
                  gap: '8px',
                  flexWrap: 'nowrap',
                  overflowX: 'auto',
                  paddingBottom: '2px',
                }}
              >
                <button
                  className={`btn ${activeCategory === 'all' ? 'btn-primary' : 'btn-secondary'}`}
                  onClick={() => setActiveCategory('all')}
                  style={{ fontSize: '0.82rem', padding: '6px 14px', whiteSpace: 'nowrap', flexShrink: 0, borderRadius: '8px' }}
                >
                  All
                </button>
                {CATEGORIES.map((c) => (
                  <button
                    key={c.id}
                    className={`btn ${activeCategory === c.id ? 'btn-primary' : 'btn-secondary'}`}
                    onClick={() => setActiveCategory(c.id)}
                    style={{ fontSize: '0.82rem', padding: '6px 14px', whiteSpace: 'nowrap', flexShrink: 0, borderRadius: '8px' }}
                  >
                    {c.name}
                  </button>
                ))}
              </div>
            </div>

            {/* Tool Grid: 5 Columns with Zero Title/Description Truncation */}
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(5, minmax(0, 1fr))',
                gap: '12px',
              }}
            >
              {filteredTools.map((t) => {
                const IconComponent = t.icon
                return (
                  <div
                    key={t.id}
                    onClick={() => {
                      setActiveTool(t)
                      setErrorMsg(null)
                    }}
                    style={{
                      padding: '16px 14px',
                      borderRadius: '12px',
                      backgroundColor: 'var(--bg-secondary)',
                      border: '1px solid var(--border-subtle)',
                      cursor: 'pointer',
                      display: 'flex',
                      flexDirection: 'column',
                      justifyContent: 'flex-start',
                      minHeight: '138px',
                      transition: 'transform 0.18s ease, border-color 0.18s ease, box-shadow 0.18s ease',
                      boxSizing: 'border-box',
                    }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.borderColor = t.color
                      e.currentTarget.style.transform = 'translateY(-3px)'
                      e.currentTarget.style.boxShadow = '0 6px 16px -2px rgba(0, 0, 0, 0.08)'
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.borderColor = 'var(--border-subtle)'
                      e.currentTarget.style.transform = 'none'
                      e.currentTarget.style.boxShadow = 'none'
                    }}
                  >
                    {/* Top Row: Icon Badge */}
                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '10px' }}>
                      <div
                        style={{
                          width: '36px',
                          height: '36px',
                          borderRadius: '10px',
                          backgroundColor: `${t.color}18`,
                          color: t.color,
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'center',
                          flexShrink: 0,
                        }}
                      >
                        <IconComponent size={19} />
                      </div>
                    </div>

                    {/* Tool Name: Full width, NO ellipsis, wraps cleanly if needed */}
                    <div
                      style={{
                        fontWeight: 700,
                        fontSize: '0.90rem',
                        color: 'var(--text-primary)',
                        lineHeight: 1.25,
                        marginBottom: '6px',
                      }}
                    >
                      {t.name}
                    </div>

                    {/* Description: Full readable text without truncation */}
                    <div
                      style={{
                        fontSize: '0.75rem',
                        color: 'var(--text-secondary)',
                        lineHeight: 1.38,
                      }}
                    >
                      {t.desc}
                    </div>
                  </div>
                )
              })}
            </div>
          </div>
        ) : (
          /* VIEW 2: ACTIVE TOOL WORKSPACE (SPLIT SCREEN) */
          <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
            {/* Left Panel: Settings & Parameters */}
            <div
              style={{
                width: '380px',
                borderRight: '1px solid var(--border-subtle)',
                backgroundColor: 'var(--bg-secondary)',
                display: 'flex',
                flexDirection: 'column',
                overflowY: 'auto',
                flexShrink: 0,
              }}
            >
              {/* File Selector Section */}
              <div style={{ padding: '16px 20px', borderBottom: '1px solid var(--border-subtle)' }}>
                <div style={{ fontSize: '0.85rem', fontWeight: 700, marginBottom: '10px', color: 'var(--text-secondary)' }}>
                  DOCUMENT FILES
                </div>

                {files.length === 0 ? (
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                    <label
                      className="btn btn-secondary"
                      style={{
                        padding: '16px',
                        border: '2px dashed var(--border-subtle)',
                        borderRadius: 'var(--radius-md)',
                        display: 'flex',
                        flexDirection: 'column',
                        alignItems: 'center',
                        justifyContent: 'center',
                        gap: '6px',
                        cursor: 'pointer',
                        textAlign: 'center',
                      }}
                    >
                      <UploadCloud size={24} style={{ color: 'var(--accent-primary)' }} />
                      <span style={{ fontSize: '0.85rem', fontWeight: 600 }}>Choose file from disk</span>
                      <span style={{ fontSize: '0.72rem', color: 'var(--text-muted)' }}>
                        {activeTool.accept || '.pdf files'}
                      </span>
                      <input
                        type="file"
                        accept={activeTool.accept || '.pdf'}
                        multiple={activeTool.multi}
                        style={{ display: 'none' }}
                        onChange={handleFileUpload}
                      />
                    </label>

                    <button
                      className="btn btn-secondary"
                      onClick={() => setShowVaultPicker(true)}
                      style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '8px', fontSize: '0.85rem' }}
                    >
                      <Files size={15} /> Select from FileDeck Vault
                    </button>
                  </div>
                ) : (
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                    {files.map((f, i) => (
                      <div
                        key={f.id || i}
                        style={{
                          padding: '10px 14px',
                          borderRadius: 'var(--radius-md)',
                          backgroundColor: 'var(--bg-tertiary)',
                          border: '1px solid var(--border-subtle)',
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'space-between',
                        }}
                      >
                        <div style={{ overflow: 'hidden' }}>
                          <div style={{ fontSize: '0.85rem', fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                            {f.name}
                          </div>
                          <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                            {f.pages ? `${f.pages} pages • ` : ''}{(f.size / 1024).toFixed(1)} KB
                            {f.vault_doc_id && <span style={{ color: 'var(--accent-primary)', marginLeft: '4px' }}>[Vault #{f.vault_doc_id}]</span>}
                          </div>
                        </div>

                        <button
                          onClick={() => setFiles(files.filter((_, idx) => idx !== i))}
                          style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer' }}
                        >
                          <Trash2 size={14} />
                        </button>
                      </div>
                    ))}

                    <div style={{ display: 'flex', gap: '8px', marginTop: '6px' }}>
                      {activeTool.multi && (
                        <label className="btn btn-secondary" style={{ flex: 1, padding: '6px', fontSize: '0.78rem', cursor: 'pointer', textAlign: 'center' }}>
                          + Add More
                          <input type="file" multiple accept={activeTool.accept || '.pdf'} style={{ display: 'none' }} onChange={handleFileUpload} />
                        </label>
                      )}
                      <button className="btn btn-secondary" onClick={() => setFiles([])} style={{ flex: 1, padding: '6px', fontSize: '0.78rem' }}>
                        Clear
                      </button>
                    </div>
                  </div>
                )}
              </div>

              {/* Dynamic Parameter Options Section */}
              <div style={{ padding: '20px', flex: 1, display: 'flex', flexDirection: 'column', gap: '16px' }}>
                <div style={{ fontSize: '0.85rem', fontWeight: 700, color: 'var(--text-secondary)' }}>TOOL SETTINGS</div>

                {/* Watermark Options */}
                {activeTool.id === 'watermark' && (
                  <>
                    <div>
                      <label style={{ fontSize: '0.8rem', fontWeight: 600, display: 'block', marginBottom: '6px' }}>Watermark Text</label>
                      <input
                        type="text"
                        className="input"
                        value={options.watermark_text}
                        onChange={(e) => setOptions({ ...options, watermark_text: e.target.value })}
                        style={{ width: '100%' }}
                      />
                    </div>

                    <div>
                      <label style={{ fontSize: '0.8rem', fontWeight: 600, display: 'block', marginBottom: '6px' }}>
                        Angle ({options.watermark_rotation}°)
                      </label>
                      <input
                        type="range"
                        min="0"
                        max="360"
                        step="15"
                        value={options.watermark_rotation}
                        onChange={(e) => setOptions({ ...options, watermark_rotation: parseInt(e.target.value, 10) })}
                        style={{ width: '100%' }}
                      />
                    </div>

                    <div>
                      <label style={{ fontSize: '0.8rem', fontWeight: 600, display: 'block', marginBottom: '6px' }}>
                        Opacity ({Math.round(options.watermark_opacity * 100)}%)
                      </label>
                      <input
                        type="range"
                        min="0.05"
                        max="1.0"
                        step="0.05"
                        value={options.watermark_opacity}
                        onChange={(e) => setOptions({ ...options, watermark_opacity: parseFloat(e.target.value) })}
                        style={{ width: '100%' }}
                      />
                    </div>

                    <div>
                      <label style={{ fontSize: '0.8rem', fontWeight: 600, display: 'block', marginBottom: '6px' }}>Font Size ({options.watermark_size}pt)</label>
                      <input
                        type="range"
                        min="16"
                        max="96"
                        step="4"
                        value={options.watermark_size}
                        onChange={(e) => setOptions({ ...options, watermark_size: parseInt(e.target.value, 10) })}
                        style={{ width: '100%' }}
                      />
                    </div>

                    <label style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '0.85rem', cursor: 'pointer' }}>
                      <input
                        type="checkbox"
                        checked={options.watermark_mosaic}
                        onChange={(e) => setOptions({ ...options, watermark_mosaic: e.target.checked })}
                      />
                      Repeat across full page (Mosaic Tile)
                    </label>
                  </>
                )}

                {/* Compress Options */}
                {activeTool.id === 'compress' && (
                  <div>
                    <label style={{ fontSize: '0.8rem', fontWeight: 600, display: 'block', marginBottom: '8px' }}>Compression Preset</label>
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                      {[
                        { id: 'low', label: 'Low Compression', sub: 'Highest visual quality, light size reduction' },
                        { id: 'recommended', label: 'Recommended', sub: 'Balanced quality and size reduction' },
                        { id: 'extreme', label: 'Extreme Compression', sub: 'Maximum size reduction for fast sharing' },
                      ].map((lvl) => (
                        <div
                          key={lvl.id}
                          onClick={() => setOptions({ ...options, compress_level: lvl.id })}
                          style={{
                            padding: '10px 14px',
                            borderRadius: 'var(--radius-md)',
                            border: `1px solid ${options.compress_level === lvl.id ? 'var(--accent-primary)' : 'var(--border-subtle)'}`,
                            backgroundColor: options.compress_level === lvl.id ? 'var(--accent-soft)' : 'var(--bg-tertiary)',
                            cursor: 'pointer',
                          }}
                        >
                          <div style={{ fontWeight: 600, fontSize: '0.85rem' }}>{lvl.label}</div>
                          <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>{lvl.sub}</div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {/* Rotate Options */}
                {activeTool.id === 'rotate' && (
                  <>
                    <div>
                      <label style={{ fontSize: '0.8rem', fontWeight: 600, display: 'block', marginBottom: '6px' }}>Rotation Angle</label>
                      <div style={{ display: 'flex', gap: '8px' }}>
                        {[90, 180, 270].map((deg) => (
                          <button
                            key={deg}
                            className={`btn ${options.rotate_angle === deg ? 'btn-primary' : 'btn-secondary'}`}
                            onClick={() => setOptions({ ...options, rotate_angle: deg })}
                            style={{ flex: 1 }}
                          >
                            +{deg}°
                          </button>
                        ))}
                      </div>
                    </div>

                    <div>
                      <label style={{ fontSize: '0.8rem', fontWeight: 600, display: 'block', marginBottom: '6px' }}>Apply to Pages</label>
                      <select
                        className="input"
                        value={options.rotate_apply}
                        onChange={(e) => setOptions({ ...options, rotate_apply: e.target.value })}
                        style={{ width: '100%' }}
                      >
                        <option value="all">All Pages</option>
                        <option value="odd">Odd Pages Only</option>
                        <option value="even">Even Pages Only</option>
                      </select>
                    </div>
                  </>
                )}

                {/* Page Numbers Options */}
                {activeTool.id === 'page_numbers' && (
                  <>
                    <div>
                      <label style={{ fontSize: '0.8rem', fontWeight: 600, display: 'block', marginBottom: '6px' }}>Format Template</label>
                      <input
                        type="text"
                        className="input"
                        value={options.page_number_fmt}
                        onChange={(e) => setOptions({ ...options, page_number_fmt: e.target.value })}
                        style={{ width: '100%' }}
                      />
                      <div style={{ fontSize: '0.72rem', color: 'var(--text-muted)', marginTop: '4px' }}>
                        Use {'{n}'} for page number, {'{total}'} for total pages
                      </div>
                    </div>

                    <div>
                      <label style={{ fontSize: '0.8rem', fontWeight: 600, display: 'block', marginBottom: '6px' }}>Position Grid</label>
                      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '6px', width: '150px' }}>
                        {['tl', 'tc', 'tr', 'ml', 'mc', 'mr', 'bl', 'bc', 'br'].map((p) => (
                          <button
                            key={p}
                            className={`btn ${options.page_number_pos === p ? 'btn-primary' : 'btn-secondary'}`}
                            onClick={() => setOptions({ ...options, page_number_pos: p })}
                            style={{ padding: '6px', fontSize: '0.75rem', textTransform: 'uppercase' }}
                          >
                            {p}
                          </button>
                        ))}
                      </div>
                    </div>
                  </>
                )}

                {/* Protect Options */}
                {activeTool.id === 'protect' && (
                  <>
                    <div>
                      <label style={{ fontSize: '0.8rem', fontWeight: 600, display: 'block', marginBottom: '6px' }}>Set Password</label>
                      <input
                        type="password"
                        className="input"
                        placeholder="Enter password..."
                        value={options.password}
                        onChange={(e) => setOptions({ ...options, password: e.target.value })}
                        style={{ width: '100%' }}
                      />
                    </div>
                    <label style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '0.85rem' }}>
                      <input
                        type="checkbox"
                        checked={options.allow_print}
                        onChange={(e) => setOptions({ ...options, allow_print: e.target.checked })}
                      />
                      Allow Printing
                    </label>
                    <label style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '0.85rem' }}>
                      <input
                        type="checkbox"
                        checked={options.allow_copy}
                        onChange={(e) => setOptions({ ...options, allow_copy: e.target.checked })}
                      />
                      Allow Text & Image Copying
                    </label>
                  </>
                )}

                {/* Redaction Options */}
                {activeTool.id === 'redact' && (
                  <div>
                    <label style={{ fontSize: '0.8rem', fontWeight: 600, display: 'block', marginBottom: '6px' }}>
                      Search Words to Permanently Redact
                    </label>
                    <input
                      type="text"
                      className="input"
                      placeholder="e.g. Secret, SSN, 123-45-6789"
                      value={options.redact_terms}
                      onChange={(e) => setOptions({ ...options, redact_terms: e.target.value })}
                      style={{ width: '100%' }}
                    />
                    <div style={{ fontSize: '0.72rem', color: 'var(--text-muted)', marginTop: '4px' }}>
                      Separate multiple keywords with commas. Underlying text will be erased cryptographically.
                    </div>
                  </div>
                )}

                {/* HTML to PDF Options */}
                {activeTool.id === 'html_to_pdf' && (
                  <div>
                    <label style={{ fontSize: '0.8rem', fontWeight: 600, display: 'block', marginBottom: '6px' }}>Website URL</label>
                    <input
                      type="text"
                      className="input"
                      placeholder="https://example.com"
                      value={options.html_url}
                      onChange={(e) => setOptions({ ...options, html_url: e.target.value })}
                      style={{ width: '100%' }}
                    />
                  </div>
                )}
              </div>
            </div>

            {/* Right Panel: Live Document Preview */}
            <div
              style={{
                flex: 1,
                display: 'flex',
                flexDirection: 'column',
                backgroundColor: 'var(--bg-primary)',
                overflow: 'hidden',
                position: 'relative',
              }}
            >
              {/* Preview Bar */}
              <div
                style={{
                  padding: '10px 16px',
                  borderBottom: '1px solid var(--border-subtle)',
                  backgroundColor: 'var(--bg-secondary)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '0.85rem' }}>
                  <Eye size={15} style={{ color: 'var(--accent-primary)' }} />
                  <span style={{ fontWeight: 600 }}>Live Output Preview</span>
                  {previewLoading && (
                    <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                      <RefreshCw size={12} className="animate-spin inline mr-1" /> Updating...
                    </span>
                  )}
                </div>

                {files.length > 0 && currentTotalPages > 1 && (
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '0.85rem' }}>
                    <button
                      className="btn btn-secondary"
                      disabled={activePageIndex <= 0}
                      onClick={() => setActivePageIndex((p) => Math.max(0, p - 1))}
                      style={{ padding: '4px 8px' }}
                    >
                      Prev
                    </button>
                    <span>
                      Page {activePageIndex + 1} of {currentTotalPages}
                    </span>
                    <button
                      className="btn btn-secondary"
                      disabled={activePageIndex >= currentTotalPages - 1}
                      onClick={() => setActivePageIndex((p) => Math.min(currentTotalPages - 1, p + 1))}
                      style={{ padding: '4px 8px' }}
                    >
                      Next
                    </button>
                  </div>
                )}
              </div>

              {/* Preview Canvas / Image Stage */}
              <div
                style={{
                  flex: 1,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  overflow: 'auto',
                  padding: '24px',
                  backgroundColor: 'rgba(0,0,0,0.15)',
                }}
              >
                {previewFileId ? (
                  <div
                    style={{
                      boxShadow: '0 8px 30px rgba(0,0,0,0.3)',
                      borderRadius: '4px',
                      overflow: 'hidden',
                      backgroundColor: '#ffffff',
                      maxWidth: '100%',
                      maxHeight: '100%',
                    }}
                  >
                    <img
                      src={api.getPdfThumbnailUrl(previewFileId, activePageIndex, 800, options.password)}
                      alt="Preview page"
                      style={{ display: 'block', maxWidth: '100%', maxHeight: '70vh', objectFit: 'contain' }}
                      onError={(e) => {
                        e.currentTarget.style.display = 'none'
                      }}
                    />
                  </div>
                ) : (
                  <div style={{ textAlign: 'center', color: 'var(--text-muted)' }}>
                    <Layers size={48} style={{ opacity: 0.3, marginBottom: '12px' }} />
                    <p>Add a file on the left to see live preview.</p>
                  </div>
                )}
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Vault Document Picker Modal */}
      {showVaultPicker && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            backgroundColor: 'rgba(0,0,0,0.6)',
            zIndex: 100,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          <div
            style={{
              width: '560px',
              maxHeight: '80vh',
              backgroundColor: 'var(--bg-secondary)',
              borderRadius: 'var(--radius-lg)',
              border: '1px solid var(--border-subtle)',
              display: 'flex',
              flexDirection: 'column',
              overflow: 'hidden',
              boxShadow: '0 12px 40px rgba(0,0,0,0.5)',
            }}
          >
            <div style={{ padding: '16px 20px', borderBottom: '1px solid var(--border-subtle)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <h3 style={{ margin: 0, fontSize: '1.1rem', fontWeight: 700 }}>Select Document from Vault</h3>
              <button className="btn btn-secondary" onClick={() => setShowVaultPicker(false)}>
                ✕
              </button>
            </div>

            <div style={{ flex: 1, overflowY: 'auto', padding: '12px 20px' }}>
              {documents.length === 0 ? (
                <div style={{ textAlign: 'center', color: 'var(--text-muted)', padding: '24px' }}>No documents in vault.</div>
              ) : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                  {documents.map((d) => (
                    <div
                      key={d.id}
                      onClick={() => {
                        loadVaultDoc(d.id)
                        setShowVaultPicker(false)
                      }}
                      style={{
                        padding: '12px 16px',
                        borderRadius: 'var(--radius-md)',
                        backgroundColor: 'var(--bg-tertiary)',
                        border: '1px solid var(--border-subtle)',
                        cursor: 'pointer',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                      }}
                      onMouseEnter={(e) => (e.currentTarget.style.borderColor = 'var(--accent-primary)')}
                      onMouseLeave={(e) => (e.currentTarget.style.borderColor = 'var(--border-subtle)')}
                    >
                      <div>
                        <div style={{ fontWeight: 600, fontSize: '0.9rem' }}>{d.title}</div>
                        <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                          ID #{d.id} • {d.original_file_name || 'document.pdf'}
                        </div>
                      </div>
                      <span className="btn btn-primary" style={{ fontSize: '0.75rem', padding: '4px 10px' }}>
                        Select
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Success Output Action Modal */}
      {successResult && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            backgroundColor: 'rgba(0,0,0,0.6)',
            zIndex: 110,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          <div
            style={{
              width: '540px',
              backgroundColor: 'var(--bg-secondary)',
              borderRadius: 'var(--radius-lg)',
              border: '1px solid var(--border-subtle)',
              padding: '24px',
              boxShadow: '0 12px 40px rgba(0,0,0,0.5)',
              display: 'flex',
              flexDirection: 'column',
              gap: '16px',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <div
                style={{
                  width: '36px',
                  height: '36px',
                  borderRadius: '50%',
                  backgroundColor: 'rgba(16, 185, 129, 0.15)',
                  color: '#10b981',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                }}
              >
                <Check size={20} />
              </div>
              <h3 style={{ margin: 0, fontSize: '1.2rem', fontWeight: 700 }}>Operation Completed!</h3>
            </div>

            <p style={{ margin: 0, fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
              Completed in {successResult.duration}s. Choose how you would like to handle the processed output:
            </p>

            {successResult.outputs?.map((out: any, idx: number) => (
              <div
                key={idx}
                style={{
                  padding: '14px',
                  borderRadius: 'var(--radius-md)',
                  backgroundColor: 'var(--bg-tertiary)',
                  border: '1px solid var(--border-subtle)',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '12px',
                }}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <div style={{ fontWeight: 600, fontSize: '0.9rem' }}>{out.name}</div>
                  <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>{(out.size / 1024).toFixed(1)} KB</div>
                </div>

                <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                  <a
                    href={api.getPdfDownloadUrl(out.id)}
                    download={out.name}
                    className="btn btn-primary"
                    style={{ textDecoration: 'none', display: 'flex', alignItems: 'center', gap: '6px', fontSize: '0.85rem' }}
                  >
                    <Download size={15} /> Download to Disk
                  </a>

                  <button
                    className="btn btn-secondary"
                    onClick={() => handleSaveAsNewDoc(out.id, out.name)}
                    style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '0.85rem' }}
                  >
                    <Save size={15} /> Save as New Document
                  </button>

                  {files[0]?.vault_doc_id && (
                    <button
                      className="btn btn-secondary"
                      onClick={() => handleSaveAsVersion(out.id)}
                      style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '0.85rem' }}
                    >
                      <Layers size={15} /> Save as Version of #{files[0].vault_doc_id}
                    </button>
                  )}
                </div>
              </div>
            ))}

            {saveStatus && (
              <div style={{ fontSize: '0.85rem', color: 'var(--accent-primary)', fontWeight: 600 }}>{saveStatus}</div>
            )}

            <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: '8px' }}>
              <button className="btn btn-secondary" onClick={() => setSuccessResult(null)}>
                Close
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Error Bar */}
      {errorMsg && (
        <div
          style={{
            padding: '10px 20px',
            backgroundColor: 'rgba(239, 68, 68, 0.15)',
            borderTop: '1px solid #ef4444',
            color: '#ef4444',
            fontSize: '0.85rem',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <AlertCircle size={16} />
            <span>{errorMsg}</span>
          </div>
          <button onClick={() => setErrorMsg(null)} style={{ background: 'none', border: 'none', color: '#ef4444', cursor: 'pointer' }}>
            ✕
          </button>
        </div>
      )}
    </div>
  )
}
