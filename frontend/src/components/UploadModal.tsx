import React, { useState, useRef } from 'react'
import { UploadCloud, X, CheckCircle, AlertCircle, FileText } from 'lucide-react'
import { api } from '../api/client'

interface UploadModalProps {
  isOpen: boolean
  onClose: () => void
  onSuccess: () => void
}

export const UploadModal: React.FC<UploadModalProps> = ({ isOpen, onClose, onSuccess }) => {
  const [file, setFile] = useState<File | null>(null)
  const [title, setTitle] = useState('')
  const [isUploading, setIsUploading] = useState(false)
  const [statusMsg, setStatusMsg] = useState<{ type: 'success' | 'error'; text: string } | null>(null)
  const [isDragOver, setIsDragOver] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)

  if (!isOpen) return null

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault()
    setIsDragOver(false)
    if (e.dataTransfer.files && e.dataTransfer.files[0]) {
      const droppedFile = e.dataTransfer.files[0]
      setFile(droppedFile)
      if (!title) setTitle(droppedFile.name.replace(/\.[^/.]+$/, ''))
    }
  }

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files[0]) {
      const selected = e.target.files[0]
      setFile(selected)
      if (!title) setTitle(selected.name.replace(/\.[^/.]+$/, ''))
    }
  }

  const handleUpload = async () => {
    if (!file) return
    setIsUploading(true)
    setStatusMsg(null)

    try {
      const formData = new FormData()
      formData.append('document', file)
      if (title.trim()) formData.append('title', title.trim())

      await api.postDocument(formData)
      setStatusMsg({ type: 'success', text: 'Document submitted for ingestion!' })
      setTimeout(() => {
        onSuccess()
        onClose()
        setFile(null)
        setTitle('')
        setStatusMsg(null)
      }, 1200)
    } catch (err: any) {
      setStatusMsg({ type: 'error', text: err.message || 'Upload failed' })
    } finally {
      setIsUploading(false)
    }
  }

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.75)',
        backdropFilter: 'blur(8px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
      }}
      onClick={onClose}
    >
      <div
        className="card-panel animate-fade-in"
        style={{
          width: '100%',
          maxWidth: '520px',
          padding: '28px',
          position: 'relative',
          borderRadius: 'var(--radius-xl)',
          boxShadow: 'var(--shadow-lg)',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '20px' }}>
          <div>
            <h2 style={{ fontSize: '1.25rem', fontWeight: 800, color: 'var(--text-primary)' }}>Add New Document</h2>
            <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', marginTop: '2px' }}>
              Upload a document to ingest, OCR, and index into your vault.
            </p>
          </div>
          <button
            onClick={onClose}
            style={{
              background: 'transparent',
              border: 'none',
              color: 'var(--text-muted)',
              cursor: 'pointer',
              padding: '4px',
            }}
          >
            <X size={20} />
          </button>
        </div>

        {/* Drop zone */}
        <div
          onDragOver={(e) => {
            e.preventDefault()
            setIsDragOver(true)
          }}
          onDragLeave={() => setIsDragOver(false)}
          onDrop={handleDrop}
          onClick={() => inputRef.current?.click()}
          style={{
            border: `2px dashed ${isDragOver ? 'var(--accent-primary)' : 'var(--border-subtle)'}`,
            borderRadius: 'var(--radius-lg)',
            padding: '36px 20px',
            textAlign: 'center',
            cursor: 'pointer',
            background: isDragOver ? 'var(--accent-soft)' : 'var(--bg-tertiary)',
            transition: 'all 0.2s ease',
            marginBottom: '20px',
          }}
        >
          <input
            ref={inputRef}
            type="file"
            style={{ display: 'none' }}
            onChange={handleFileChange}
            accept=".pdf,.png,.jpg,.jpeg,.webp,.docx,.xlsx"
          />
          {file ? (
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px' }}>
              <FileText size={40} color="var(--accent-primary)" />
              <div style={{ fontWeight: 600, fontSize: '0.95rem' }}>{file.name}</div>
              <div style={{ color: 'var(--text-muted)', fontSize: '0.8rem' }}>
                {(file.size / 1024 / 1024).toFixed(2)} MB · Click to choose different file
              </div>
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px' }}>
              <UploadCloud size={40} color="var(--accent-primary)" />
              <div style={{ fontWeight: 600 }}>Drag & drop document here, or click to browse</div>
              <div style={{ color: 'var(--text-muted)', fontSize: '0.8rem' }}>
                Supports PDF, Images, Word, and Excel spreadsheets
              </div>
            </div>
          )}
        </div>

        {file && (
          <div style={{ marginBottom: '20px' }}>
            <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, marginBottom: '6px', color: 'var(--text-secondary)' }}>
              Document Title (Optional)
            </label>
            <input
              type="text"
              className="input"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Leave empty to use filename"
            />
          </div>
        )}

        {statusMsg && (
          <div
            style={{
              padding: '10px 14px',
              borderRadius: 'var(--radius-sm)',
              marginBottom: '16px',
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              fontSize: '0.875rem',
              backgroundColor: statusMsg.type === 'success' ? 'rgba(16, 185, 129, 0.15)' : 'rgba(239, 68, 68, 0.15)',
              color: statusMsg.type === 'success' ? '#6ee7b7' : '#fca5a5',
              border: `1px solid ${statusMsg.type === 'success' ? 'rgba(16, 185, 129, 0.3)' : 'rgba(239, 68, 68, 0.3)'}`,
            }}
          >
            {statusMsg.type === 'success' ? <CheckCircle size={16} /> : <AlertCircle size={16} />}
            {statusMsg.text}
          </div>
        )}

        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '10px' }}>
          <button className="btn btn-secondary" onClick={onClose} disabled={isUploading}>
            Cancel
          </button>
          <button
            className="btn btn-primary"
            onClick={handleUpload}
            disabled={!file || isUploading}
          >
            {isUploading ? 'Ingesting...' : 'Upload & Process'}
          </button>
        </div>
      </div>
    </div>
  )
}
