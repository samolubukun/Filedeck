import { useState, useRef } from 'react'
import {
  Files,
  Inbox,
  Tag,
  Users,
  Upload,
  ChevronRight,
  FileUp,
  PieChart,
  Bookmark,
  Filter,
  ArrowUpRight,
} from 'lucide-react'
import { useQuery } from '@tanstack/react-query'
import { StatCard } from '../components/StatCard'
import { DocumentCard } from '../components/DocumentCard'
import { useDocuments } from '../api/useDocuments'
import { useTags } from '../api/useTags'
import { useCorrespondents } from '../api/useCorrespondents'
import { useStats } from '../api/useSystem'
import { useUiStore } from '../store/useUiStore'
import { useFilterStore } from '../store/useFilterStore'
import { api } from '../api/client'

export function Dashboard() {
  const { documents, totalDocs, refetch } = useDocuments()
  const { tags } = useTags()
  const { correspondents } = useCorrespondents()
  const { data: stats } = useStats()
  const setCurrentNav = useUiStore((s) => s.setCurrentNav)
  const setUploadOpen = useUiStore((s) => s.setUploadOpen)
  const setActiveDocId = useUiStore((s) => s.setActiveDocId)
  const applySavedView = useFilterStore((s) => s.applySavedView)

  const { data: savedViewsData } = useQuery({
    queryKey: ['saved_views'],
    queryFn: () => api.getSavedViews(),
  })
  const savedViews = savedViewsData?.results ?? []

  const [isDragging, setIsDragging] = useState(false)
  const [uploading, setUploading] = useState(false)
  const fileInputRef = useRef<HTMLInputElement>(null)

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(true)
  }

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(false)
  }

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(false)
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      await handleFiles(Array.from(e.dataTransfer.files))
    }
  }

  const handleFiles = async (files: File[]) => {
    setUploading(true)
    try {
      for (const file of files) {
        const formData = new FormData()
        formData.append('document', file)
        formData.append('title', file.name.replace(/\.[^/.]+$/, ''))
        await api.postDocument(formData)
      }
      await refetch()
    } catch (err) {
      console.error('Failed to ingest dropped file', err)
    } finally {
      setUploading(false)
    }
  }

  // Calculate file type breakdown
  const fileTypeCounts = stats?.document_file_type_counts || []
  let pdfCount = 0
  let imageCount = 0
  let officeCount = 0
  let otherCount = 0

  if (fileTypeCounts.length > 0) {
    for (const item of fileTypeCounts) {
      const mime = item.mime_type.toLowerCase()
      if (mime.includes('pdf')) {
        pdfCount += item.count
      } else if (
        mime.includes('image') ||
        mime.includes('png') ||
        mime.includes('jpeg') ||
        mime.includes('jpg') ||
        mime.includes('webp') ||
        mime.includes('tiff')
      ) {
        imageCount += item.count
      } else if (
        mime.includes('word') ||
        mime.includes('sheet') ||
        mime.includes('excel') ||
        mime.includes('office') ||
        mime.includes('msword')
      ) {
        officeCount += item.count
      } else {
        otherCount += item.count
      }
    }
  } else {
    for (const doc of documents) {
      const mime = (doc.mime_type || '').toLowerCase()
      const ext = (doc.original_file_name || '').toLowerCase()
      if (mime.includes('pdf') || ext.endsWith('.pdf')) {
        pdfCount++
      } else if (mime.includes('image') || ext.match(/\.(png|jpe?g|webp|tiff?|bmp)$/)) {
        imageCount++
      } else if (ext.match(/\.(docx?|xlsx?|pptx?)$/)) {
        officeCount++
      } else {
        otherCount++
      }
    }
  }

  const sumCount = pdfCount + imageCount + officeCount + otherCount
  const totalTypes = sumCount > 0 ? sumCount : 0
  const pdfPct = totalTypes > 0 ? Math.round((pdfCount / totalTypes) * 100) : 0
  const imgPct = totalTypes > 0 ? Math.round((imageCount / totalTypes) * 100) : 0
  const offPct = totalTypes > 0 ? Math.round((officeCount / totalTypes) * 100) : 0
  const othPct = totalTypes > 0 ? Math.max(0, 100 - pdfPct - imgPct - offPct) : 0

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '28px' }}>
      {/* Header Stat Cards Grid: Four in a row */}
      <div className="dashboard-stats-grid">
        <StatCard
          label="Total Documents"
          value={stats?.documents_total ?? totalDocs}
          icon={Files}
          color="#2563eb"
          accentBg="rgba(37, 99, 235, 0.1)"
          subtitle="Indexed in vault"
          onClick={() => setCurrentNav('documents')}
        />
        <StatCard
          label="Inbox Items"
          value={stats?.documents_inbox ?? 0}
          icon={Inbox}
          color="#06b6d4"
          accentBg="rgba(6, 182, 212, 0.1)"
          subtitle="Pending review & tagging"
          onClick={() => setCurrentNav('documents')}
        />
        <StatCard
          label="Active Tags"
          value={stats?.tags_total ?? tags.length}
          icon={Tag}
          color="#a855f7"
          accentBg="rgba(168, 85, 247, 0.1)"
          subtitle="Classification taxonomy"
          onClick={() => setCurrentNav('tags')}
        />
        <StatCard
          label="Correspondents"
          value={stats?.correspondents_total ?? correspondents.length}
          icon={Users}
          color="#10b981"
          accentBg="rgba(16, 185, 129, 0.1)"
          subtitle="Senders & institutions"
          onClick={() => setCurrentNav('correspondents')}
        />
      </div>

      {/* Prominent Upload Drop Zone (SlothUI Reference Match) */}
      <div
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
        onClick={() => fileInputRef.current?.click()}
        className="card-panel"
        style={{
          padding: '32px 24px',
          textAlign: 'center',
          cursor: 'pointer',
          borderRadius: 'var(--radius-xl)',
          backgroundColor: isDragging ? 'var(--accent-soft)' : 'var(--bg-card)',
          border: isDragging ? '2px dashed var(--accent-primary)' : '1.5px dashed var(--accent-primary)',
          boxShadow: isDragging ? '0 0 24px var(--accent-glow)' : 'var(--shadow-card)',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '12px',
          transition: 'all 0.2s ease',
        }}
      >
        <input
          ref={fileInputRef}
          type="file"
          multiple
          style={{ display: 'none' }}
          onChange={(e) => {
            if (e.target.files && e.target.files.length > 0) {
              handleFiles(Array.from(e.target.files))
            }
          }}
        />

        <div
          style={{
            width: '48px',
            height: '48px',
            borderRadius: '50%',
            backgroundColor: 'var(--accent-soft)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: 'var(--accent-primary)',
          }}
        >
          <FileUp size={24} />
        </div>

        <div>
          <div style={{ fontSize: '1rem', fontWeight: 700, color: 'var(--text-primary)' }}>
            <span style={{ color: 'var(--accent-primary)' }}>Click here</span> to upload your file or drag.
          </div>
          <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)', marginTop: '4px' }}>
            Supported Formats: PDF, DOCX, XLSX, PNG, JPG, WEBP, TIFF (Embedded ONNX OCR)
          </div>
        </div>

        {uploading && (
          <div style={{ fontSize: '0.84rem', color: 'var(--accent-primary)', fontWeight: 600 }}>
            Processing & Ingesting Files...
          </div>
        )}
      </div>

      {/* File Format Distribution Breakdown Bar */}
      <div
        className="card-panel"
        style={{
          padding: '20px 24px',
          display: 'flex',
          flexDirection: 'column',
          gap: '14px',
          borderRadius: 'var(--radius-lg)',
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <div
              style={{
                width: '32px',
                height: '32px',
                borderRadius: '8px',
                backgroundColor: 'var(--accent-soft)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color: 'var(--accent-primary)',
              }}
            >
              <PieChart size={17} />
            </div>
            <h3 style={{ fontSize: '0.98rem', fontWeight: 700, letterSpacing: '-0.01em', color: 'var(--text-primary)' }}>
              Document Formats Distribution
            </h3>
          </div>
          <span style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>
            {stats?.documents_total ?? totalDocs} total items processed
          </span>
        </div>

        {/* Multi-segment progress bar */}
        <div
          style={{
            height: '10px',
            borderRadius: 'var(--radius-full)',
            backgroundColor: 'var(--bg-tertiary)',
            display: 'flex',
            overflow: 'hidden',
          }}
        >
          {pdfPct > 0 && (
            <div
              style={{
                width: `${pdfPct}%`,
                backgroundColor: '#2563eb',
                transition: 'width 0.4s ease',
              }}
              title={`PDF: ${pdfCount} (${pdfPct}%)`}
            />
          )}
          {imgPct > 0 && (
            <div
              style={{
                width: `${imgPct}%`,
                backgroundColor: '#10b981',
                transition: 'width 0.4s ease',
              }}
              title={`Images: ${imageCount} (${imgPct}%)`}
            />
          )}
          {offPct > 0 && (
            <div
              style={{
                width: `${offPct}%`,
                backgroundColor: '#06b6d4',
                transition: 'width 0.4s ease',
              }}
              title={`Office Docs: ${officeCount} (${offPct}%)`}
            />
          )}
          {othPct > 0 && (
            <div
              style={{
                width: `${othPct}%`,
                backgroundColor: '#f59e0b',
                transition: 'width 0.4s ease',
              }}
              title={`Other / Text: ${otherCount} (${othPct}%)`}
            />
          )}
        </div>

        {/* Category Legend & Badges */}
        <div style={{ display: 'flex', gap: '22px', flexWrap: 'wrap', fontSize: '0.82rem' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span style={{ width: '10px', height: '10px', borderRadius: '50%', backgroundColor: '#2563eb' }} />
            <span style={{ color: 'var(--text-secondary)' }}>PDF Documents:</span>
            <span style={{ fontWeight: 700, color: 'var(--text-primary)' }}>
              {pdfCount} ({pdfPct}%)
            </span>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span style={{ width: '10px', height: '10px', borderRadius: '50%', backgroundColor: '#10b981' }} />
            <span style={{ color: 'var(--text-secondary)' }}>Images & Scans:</span>
            <span style={{ fontWeight: 700, color: 'var(--text-primary)' }}>
              {imageCount} ({imgPct}%)
            </span>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span style={{ width: '10px', height: '10px', borderRadius: '50%', backgroundColor: '#06b6d4' }} />
            <span style={{ color: 'var(--text-secondary)' }}>Office (Word/Excel):</span>
            <span style={{ fontWeight: 700, color: 'var(--text-primary)' }}>
              {officeCount} ({offPct}%)
            </span>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span style={{ width: '10px', height: '10px', borderRadius: '50%', backgroundColor: '#f59e0b' }} />
            <span style={{ color: 'var(--text-secondary)' }}>Plain Text & Other:</span>
            <span style={{ fontWeight: 700, color: 'var(--text-primary)' }}>
              {otherCount} ({othPct}%)
            </span>
          </div>
        </div>
      </div>

      {/* Quick Saved Views Widgets */}
      {savedViews.length > 0 && (
        <div>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '14px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <Bookmark size={18} style={{ color: 'var(--accent-primary)' }} />
              <h2 style={{ fontSize: '1.15rem', fontWeight: 700, letterSpacing: '-0.01em', color: 'var(--text-primary)' }}>
                Quick Saved Views
              </h2>
            </div>
            <button
              className="btn btn-secondary"
              onClick={() => setCurrentNav('saved_views')}
              style={{ fontSize: '0.8rem', padding: '6px 14px', gap: '6px', borderRadius: 'var(--radius-full)' }}
            >
              Manage Views <ChevronRight size={14} />
            </button>
          </div>

          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(auto-fill, minmax(260px, 1fr))',
              gap: '16px',
            }}
          >
            {savedViews.slice(0, 4).map((view) => (
              <div
                key={view.id}
                onClick={() => {
                  applySavedView(view)
                  setCurrentNav('documents')
                }}
                className="card-panel card-panel-interactive"
                style={{
                  padding: '16px 20px',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  borderRadius: 'var(--radius-md)',
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
                  <div
                    style={{
                      width: '38px',
                      height: '38px',
                      borderRadius: '10px',
                      backgroundColor: 'var(--accent-soft)',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      color: 'var(--accent-primary)',
                    }}
                  >
                    <Filter size={18} />
                  </div>
                  <div>
                    <div style={{ fontWeight: 700, fontSize: '0.92rem', color: 'var(--text-primary)' }}>
                      {view.name}
                    </div>
                    <div style={{ fontSize: '0.78rem', color: 'var(--text-muted)' }}>
                      {view.filter_rules?.length ?? 0} filter criteria
                    </div>
                  </div>
                </div>

                <div
                  style={{
                    color: 'var(--accent-primary)',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '4px',
                    fontSize: '0.8rem',
                    fontWeight: 700,
                  }}
                >
                  Browse <ArrowUpRight size={14} />
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Recently Ingested Documents Section */}
      <div>
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            marginBottom: '18px',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <h2 style={{ fontSize: '1.25rem', fontWeight: 800, letterSpacing: '-0.02em', color: 'var(--text-primary)' }}>
              Recently Ingested Documents
            </h2>
            {documents.length > 0 && (
              <span
                style={{
                  fontSize: '0.75rem',
                  fontWeight: 700,
                  backgroundColor: 'var(--accent-soft)',
                  color: 'var(--accent-primary)',
                  padding: '3px 10px',
                  borderRadius: 'var(--radius-full)',
                }}
              >
                {documents.length} recent
              </span>
            )}
          </div>

          {documents.length > 0 && (
            <div style={{ display: 'flex', gap: '10px' }}>
              <button
                className="btn btn-primary"
                onClick={() => setUploadOpen(true)}
                style={{ fontSize: '0.82rem', padding: '8px 16px', gap: '6px', borderRadius: 'var(--radius-full)' }}
              >
                <Upload size={14} /> Ingest File
              </button>
              <button
                className="btn btn-secondary"
                onClick={() => setCurrentNav('documents')}
                style={{ fontSize: '0.82rem', padding: '8px 16px', gap: '6px', borderRadius: 'var(--radius-full)' }}
              >
                View All Documents <ChevronRight size={14} />
              </button>
            </div>
          )}
        </div>

        {documents.length > 0 ? (
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))',
              gap: '20px',
            }}
          >
            {documents.slice(0, 6).map((doc) => (
              <DocumentCard
                key={doc.id}
                doc={doc}
                tags={tags}
                onOpen={() => setActiveDocId(doc.id)}
              />
            ))}
          </div>
        ) : (
          <div
            className="card-panel"
            style={{
              padding: '48px 24px',
              textAlign: 'center',
              borderRadius: 'var(--radius-xl)',
            }}
          >
            <Files size={38} style={{ color: 'var(--text-muted)', marginBottom: '12px' }} />
            <h3 style={{ fontSize: '1.1rem', fontWeight: 700, color: 'var(--text-primary)', marginBottom: '6px' }}>
              No documents in vault yet
            </h3>
            <p style={{ color: 'var(--text-secondary)', fontSize: '0.88rem', maxWidth: '420px', margin: '0 auto 18px' }}>
              Upload your invoices, receipts, letters, or PDF scans using the drop zone above.
            </p>
          </div>
        )}
      </div>
    </div>
  )
}
