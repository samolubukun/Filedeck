import React, { useState, useEffect } from 'react'
import {
  Sparkles,
  Send,
  Bot,
  User,
  Download,
  CheckCircle2,
  AlertCircle,
  XCircle,
  RefreshCw,
} from 'lucide-react'
import { api, type ModelStatus } from '../api/client'

interface Message {
  role: 'user' | 'assistant'
  content: string
  sources?: Array<{
    document_id: number
    document_title: string
    chunk_index: number
    score: number
  }>
}

export const Chat: React.FC = () => {
  const [messages, setMessages] = useState<Message[]>([])
  const [input, setInput] = useState('')
  const [streaming, setStreaming] = useState(false)
  const [modelStatus, setModelStatus] = useState<ModelStatus | null>(null)
  const [actionLoading, setActionLoading] = useState(false)

  // Fetch model status on load & poll while downloading
  useEffect(() => {
    let timer: any = null

    const checkStatus = async () => {
      try {
        const res = await api.getModelStatus()
        setModelStatus(res)
        if (res.status === 'downloading') {
          timer = setTimeout(checkStatus, 1000)
        }
      } catch (err) {
        console.error('Failed to get model status', err)
      }
    }

    checkStatus()

    return () => {
      if (timer) clearTimeout(timer)
    }
  }, [])

  const handleDownload = async () => {
    setActionLoading(true)
    try {
      await api.downloadModel()
      const res = await api.getModelStatus()
      setModelStatus(res)
      // Start polling
      pollProgress()
    } catch (err) {
      console.error('Failed to start download', err)
    } finally {
      setActionLoading(false)
    }
  }

  const handleCancel = async () => {
    setActionLoading(true)
    try {
      await api.cancelModelDownload()
      const res = await api.getModelStatus()
      setModelStatus(res)
    } catch (err) {
      console.error('Failed to cancel download', err)
    } finally {
      setActionLoading(false)
    }
  }

  const pollProgress = async () => {
    try {
      const res = await api.getModelStatus()
      setModelStatus(res)
      if (res.status === 'downloading') {
        setTimeout(pollProgress, 1000)
      }
    } catch {
      // ignore
    }
  }

  const handleSend = async (overridePrompt?: string) => {
    const textToSend = overridePrompt || input
    if (!textToSend.trim() || streaming) return

    const userMsg: Message = { role: 'user', content: textToSend.trim() }
    const history = [...messages, userMsg]
    setMessages(history)
    setInput('')
    setStreaming(true)

    // Add empty assistant response to stream into
    setMessages((prev) => [...prev, { role: 'assistant', content: '' }])

    try {
      await api.chatGlobalStream(
        userMsg.content,
        messages.map((m) => ({ role: m.role, content: m.content })),
        (chunk) => {
          setMessages((prev) => {
            const updated = [...prev]
            const last = updated[updated.length - 1]
            if (last && last.role === 'assistant') {
              last.content += chunk
            }
            return updated
          })
        }
      )
    } catch (err: any) {
      setMessages((prev) => {
        const updated = [...prev]
        const last = updated[updated.length - 1]
        if (last && last.role === 'assistant') {
          last.content = `Error: ${err.message || 'AI service is currently offline or unreachable.'}`
        }
        return updated
      })
    } finally {
      setStreaming(false)
    }
  }

  const isModelReady = modelStatus?.is_ready ?? false
  const isDownloading = modelStatus?.status === 'downloading'

  return (
    <div style={{ height: 'calc(100vh - 120px)', display: 'flex', flexDirection: 'column', gap: '20px' }}>
      {/* Header */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          paddingBottom: '16px',
          borderBottom: '1px solid var(--border-subtle)',
        }}
      >
        <div>
          <h2 style={{ fontSize: '1.4rem', fontWeight: 700, display: 'flex', alignItems: 'center', gap: '10px' }}>
            <Sparkles size={22} style={{ color: 'var(--accent-primary)' }} />
            AI Document Intelligence
          </h2>
          <p style={{ color: 'var(--text-muted)', fontSize: '0.85rem', marginTop: '4px' }}>
            Ask questions across all indexed documents using semantic search and local AI.
          </p>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          {isModelReady ? (
            <div
              style={{
                fontSize: '0.75rem',
                padding: '4px 12px',
                borderRadius: '20px',
                backgroundColor: 'rgba(16, 185, 129, 0.12)',
                border: '1px solid rgba(16, 185, 129, 0.3)',
                color: '#34d399',
                fontWeight: 600,
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
              }}
            >
              <CheckCircle2 size={13} />
              Qwen 2.5 (1.5B) Ready
            </div>
          ) : isDownloading ? (
            <div
              style={{
                fontSize: '0.75rem',
                padding: '4px 12px',
                borderRadius: '20px',
                backgroundColor: 'var(--accent-soft)',
                border: '1px solid var(--accent-border)',
                color: 'var(--accent-primary)',
                fontWeight: 600,
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
              }}
            >
              <RefreshCw size={13} className="pulse-glow" />
              Downloading Engine: {modelStatus?.percent || 0}%
            </div>
          ) : (
            <div
              style={{
                fontSize: '0.75rem',
                padding: '4px 12px',
                borderRadius: '20px',
                backgroundColor: 'rgba(245, 158, 11, 0.12)',
                border: '1px solid rgba(245, 158, 11, 0.3)',
                color: '#fbbf24',
                fontWeight: 600,
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
              }}
            >
              <AlertCircle size={13} />
              Engine Not Downloaded
            </div>
          )}
        </div>
      </div>

      {/* Model Downloader Banner (If not ready or downloading) */}
      {!isModelReady && (
        <div
          className="glass-panel"
          style={{
            padding: '20px 24px',
            background: 'var(--bg-card)',
            border: '1px solid var(--border-subtle)',
            borderRadius: 'var(--radius-md)',
          }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '16px' }}>
            <div style={{ maxWidth: '620px' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '6px' }}>
                <span style={{ fontSize: '0.72rem', fontWeight: 700, textTransform: 'uppercase', color: 'var(--accent-primary)', letterSpacing: '0.06em' }}>
                  100% Offline & Private
                </span>
                <span style={{ fontSize: '0.72rem', color: 'var(--text-muted)' }}>•</span>
                <span style={{ fontSize: '0.72rem', color: 'var(--text-muted)', fontWeight: 600 }}>Zero External Software Required</span>
              </div>
              <h3 style={{ fontSize: '1.1rem', fontWeight: 700, marginBottom: '4px' }}>
                Activate In-App Intelligence Engine: Qwen 2.5 (1.5B)
              </h3>
              <p style={{ fontSize: '0.84rem', color: 'var(--text-secondary)', lineHeight: 1.5 }}>
                Download once (~986 MB) to run local document Q&A entirely on your device with high-speed CPU inference. No Ollama, no LM Studio, and no terminal setup needed.
              </p>
            </div>

            <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
              {isDownloading ? (
                <button
                  className="btn btn-danger"
                  onClick={handleCancel}
                  disabled={actionLoading}
                  style={{ padding: '8px 16px', fontSize: '0.84rem' }}
                >
                  <XCircle size={15} /> Cancel Download
                </button>
              ) : (
                <button
                  className="btn btn-primary"
                  onClick={handleDownload}
                  disabled={actionLoading}
                  style={{ padding: '10px 20px', fontSize: '0.88rem', gap: '8px' }}
                >
                  <Download size={16} /> Download Model (986 MB)
                </button>
              )}
            </div>
          </div>

          {/* Download Progress Bar */}
          {isDownloading && (
            <div style={{ marginTop: '16px', paddingTop: '16px', borderTop: '1px solid rgba(255, 255, 255, 0.08)' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.78rem', color: 'var(--text-secondary)', marginBottom: '8px' }}>
                <span>Downloading Qwen 2.5 GGUF...</span>
                <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>
                  {modelStatus?.percent || 0}% ({modelStatus?.speed_mbps || 0} MB/s)
                </span>
              </div>
              <div
                style={{
                  width: '100%',
                  height: '8px',
                  backgroundColor: 'rgba(255, 255, 255, 0.08)',
                  borderRadius: 'var(--radius-full)',
                  overflow: 'hidden',
                }}
              >
                <div
                  style={{
                    height: '100%',
                    width: `${modelStatus?.percent || 0}%`,
                    background: 'var(--accent-gradient)',
                    borderRadius: 'var(--radius-full)',
                    transition: 'width 0.3s ease',
                  }}
                />
              </div>
            </div>
          )}
        </div>
      )}

      {/* Quick Prompts Bar */}
      <div style={{ display: 'flex', gap: '10px', flexWrap: 'wrap' }}>
        {[
          'Summarize recent invoices and expenses',
          'Find documents regarding health or medical records',
          'What contracts or agreements are in the archive?',
          'List all documents mentioning warranty or terms',
        ].map((prompt) => (
          <button
            key={prompt}
            onClick={() => handleSend(prompt)}
            disabled={streaming || !isModelReady}
            className="btn btn-secondary"
            style={{
              fontSize: '0.78rem',
              padding: '6px 12px',
              borderRadius: '16px',
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              opacity: isModelReady ? 1 : 0.6,
              cursor: isModelReady ? 'pointer' : 'not-allowed',
            }}
          >
            <Sparkles size={12} style={{ color: 'var(--accent-primary)' }} />
            {prompt}
          </button>
        ))}
      </div>

      {/* Chat Messages Timeline */}
      <div
        style={{
          flex: 1,
          overflowY: 'auto',
          borderRadius: 'var(--radius-md)',
          backgroundColor: 'rgba(0,0,0,0.25)',
          border: '1px solid var(--border-subtle)',
          padding: '24px',
          display: 'flex',
          flexDirection: 'column',
          gap: '18px',
        }}
      >
        {messages.length === 0 ? (
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              height: '100%',
              color: 'var(--text-muted)',
              textAlign: 'center',
            }}
          >
            <div
              style={{
                width: '64px',
                height: '64px',
                borderRadius: '50%',
                backgroundColor: 'var(--accent-soft)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                marginBottom: '16px',
              }}
            >
              <Bot size={32} style={{ color: 'var(--accent-primary)' }} />
            </div>
            <h3 style={{ fontSize: '1.15rem', fontWeight: 600, color: 'var(--text-primary)', marginBottom: '6px' }}>
              Vault-Wide Document Assistant
            </h3>
            <p style={{ fontSize: '0.85rem', maxWidth: '420px', lineHeight: 1.5 }}>
              Ask anything about your documents. Responses are synthesized from relevant document text chunks using local semantic search.
            </p>
          </div>
        ) : (
          messages.map((msg, i) => (
            <div
              key={i}
              style={{
                display: 'flex',
                gap: '14px',
                alignItems: 'flex-start',
                alignSelf: msg.role === 'user' ? 'flex-end' : 'flex-start',
                maxWidth: '82%',
              }}
            >
              {msg.role === 'assistant' && (
                <div
                  style={{
                    width: '32px',
                    height: '32px',
                    borderRadius: '8px',
                    backgroundColor: 'var(--accent-soft)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    flexShrink: 0,
                    border: '1px solid var(--accent-border)',
                  }}
                >
                  <Bot size={18} style={{ color: 'var(--accent-primary)' }} />
                </div>
              )}

              <div
                style={{
                  padding: '14px 18px',
                  borderRadius: 'var(--radius-md)',
                  backgroundColor: msg.role === 'user' ? 'var(--accent-soft)' : 'var(--bg-surface)',
                  border:
                    msg.role === 'user'
                      ? '1px solid var(--accent-border)'
                      : '1px solid var(--border-subtle)',
                  fontSize: '0.9rem',
                  lineHeight: 1.6,
                  color: 'var(--text-primary)',
                  whiteSpace: 'pre-wrap',
                }}
              >
                {msg.content || (streaming && i === messages.length - 1 ? 'Thinking...' : '')}
              </div>

              {msg.role === 'user' && (
                <div
                  style={{
                    width: '32px',
                    height: '32px',
                    borderRadius: '8px',
                    backgroundColor: 'rgba(255, 255, 255, 0.08)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    flexShrink: 0,
                  }}
                >
                  <User size={18} style={{ color: 'var(--text-secondary)' }} />
                </div>
              )}
            </div>
          ))
        )}
      </div>

      {/* Input Field */}
      <div style={{ display: 'flex', gap: '10px' }}>
        <input
          type="text"
          className="input"
          placeholder={
            !isModelReady
              ? 'Download Qwen 2.5 above to unlock document intelligence...'
              : 'Ask a question across all documents...'
          }
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') handleSend()
          }}
          disabled={streaming || !isModelReady}
          style={{ flex: 1, padding: '12px 18px', fontSize: '0.9rem' }}
        />
        <button
          className="btn btn-primary"
          onClick={() => handleSend()}
          disabled={streaming || !input.trim() || !isModelReady}
          style={{ padding: '0 24px', opacity: !isModelReady ? 0.6 : 1 }}
        >
          <Send size={18} />
        </button>
      </div>
    </div>
  )
}
