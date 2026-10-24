import { useState, useEffect } from 'react'
import { Download, Trash2, CheckCircle2, AlertCircle, RefreshCw, HardDrive, Cpu } from 'lucide-react'
import { api, type ModelStatus } from '../api/client'
import { useSystemStatus } from '../api/useSystem'
import { PageHeading, fieldLabelStyle } from '../components/TaxonomyManager'
import { confirmModal, alertModal } from '../store/useModalStore'

const rowStyle = {
  display: 'flex',
  justifyContent: 'space-between',
  padding: '10px 14px',
  borderRadius: 'var(--radius-sm)',
  backgroundColor: 'rgba(255,255,255,0.03)',
} as const

export function Settings() {
  const { data: systemStatus } = useSystemStatus()
  const [appTitle, setAppTitle] = useState('FileDeck')
  const [ocrLang, setOcrLang] = useState('eng')
  const [modelStatus, setModelStatus] = useState<ModelStatus | null>(null)
  const [loading, setLoading] = useState(false)
  const workerConnected = systemStatus?.system?.worker_connected

  const fetchModel = async () => {
    try {
      const res = await api.getModelStatus()
      setModelStatus(res)
      if (res.status === 'downloading') {
        setTimeout(fetchModel, 1000)
      }
    } catch (e) {
      console.error(e)
    }
  }

  useEffect(() => {
    fetchModel()
  }, [])

  const handleDownload = async () => {
    setLoading(true)
    try {
      await api.downloadModel()
      fetchModel()
    } finally {
      setLoading(false)
    }
  }

  const handleDelete = async () => {
    const ok = await confirmModal({
      title: 'Delete Offline AI Model',
      message: 'Are you sure you want to delete the local Qwen 2.5 model file to free disk space?',
      confirmText: 'Delete Model',
      type: 'danger',
    })
    if (!ok) return

    setLoading(true)
    try {
      await api.deleteModel()
      fetchModel()
    } finally {
      setLoading(false)
    }
  }

  const isModelReady = modelStatus?.is_ready ?? false
  const isDownloading = modelStatus?.status === 'downloading'

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '28px', maxWidth: '800px' }}>
      <PageHeading
        title="Vault Settings & Diagnostics"
        subtitle="Local configuration, in-app offline AI engines, and worker state."
      />

      {/* In-App AI Engine Hub */}
      <div className="glass-panel" style={{ padding: '24px', display: 'flex', flexDirection: 'column', gap: '16px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div>
            <h3 style={{ fontSize: '1rem', fontWeight: 700, display: 'flex', alignItems: 'center', gap: '8px' }}>
              <Cpu size={18} style={{ color: 'var(--accent-primary)' }} />
              Local AI Engine (Offline Intelligence)
            </h3>
            <p style={{ fontSize: '0.82rem', color: 'var(--text-muted)', marginTop: '2px' }}>
              Embedded GGUF inference running locally on your CPU with zero external software.
            </p>
          </div>

          <div>
            {isModelReady ? (
              <span style={{ fontSize: '0.75rem', fontWeight: 600, color: '#34d399', backgroundColor: 'rgba(16, 185, 129, 0.12)', padding: '3px 10px', borderRadius: 'var(--radius-full)', display: 'inline-flex', alignItems: 'center', gap: '5px' }}>
                <CheckCircle2 size={13} /> Ready
              </span>
            ) : isDownloading ? (
              <span style={{ fontSize: '0.75rem', fontWeight: 600, color: 'var(--accent-primary)', backgroundColor: 'var(--accent-soft)', border: '1px solid var(--accent-border)', padding: '3px 10px', borderRadius: 'var(--radius-full)', display: 'inline-flex', alignItems: 'center', gap: '5px' }}>
                <RefreshCw size={13} className="pulse-glow" /> Downloading {modelStatus?.percent || 0}%
              </span>
            ) : (
              <span style={{ fontSize: '0.75rem', fontWeight: 600, color: '#fbbf24', backgroundColor: 'rgba(245, 158, 11, 0.12)', padding: '3px 10px', borderRadius: 'var(--radius-full)', display: 'inline-flex', alignItems: 'center', gap: '5px' }}>
                <AlertCircle size={13} /> Not Downloaded
              </span>
            )}
          </div>
        </div>

        <div style={{ backgroundColor: 'rgba(255, 255, 255, 0.03)', padding: '16px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '8px' }}>
            <div>
              <div style={{ fontWeight: 700, fontSize: '0.92rem' }}>Qwen 2.5 (1.5B Instruct)</div>
              <div style={{ fontSize: '0.78rem', color: 'var(--text-muted)', marginTop: '2px' }}>
                Filename: {modelStatus?.filename || 'qwen2.5-1.5b-instruct-q4_k_m.gguf'} (~986 MB)
              </div>
            </div>

            <div>
              {isModelReady ? (
                <button
                  className="btn btn-danger"
                  onClick={handleDelete}
                  disabled={loading}
                  style={{ fontSize: '0.8rem', padding: '6px 12px' }}
                >
                  <Trash2 size={13} /> Delete Model
                </button>
              ) : isDownloading ? (
                <span style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>
                  {modelStatus?.speed_mbps || 0} MB/s
                </span>
              ) : (
                <button
                  className="btn btn-primary"
                  onClick={handleDownload}
                  disabled={loading}
                  style={{ fontSize: '0.82rem', padding: '6px 14px' }}
                >
                  <Download size={14} /> Download (986 MB)
                </button>
              )}
            </div>
          </div>

          {isDownloading && (
            <div style={{ marginTop: '12px' }}>
              <div
                style={{
                  width: '100%',
                  height: '6px',
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
                    transition: 'width 0.3s ease',
                  }}
                />
              </div>
            </div>
          )}

          <div style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '0.72rem', color: 'var(--text-muted)', marginTop: '10px' }}>
            <HardDrive size={13} />
            <span>Storage directory: {modelStatus?.models_dir || 'data/models/'}</span>
          </div>
        </div>
      </div>

      <div className="glass-panel" style={{ padding: '24px', display: 'flex', flexDirection: 'column', gap: '16px' }}>
        <h3 style={{ fontSize: '1rem', fontWeight: 700 }}>General Vault Configuration</h3>

        <div>
          <label style={fieldLabelStyle}>Vault Application Title</label>
          <input type="text" className="input" value={appTitle} onChange={(e) => setAppTitle(e.target.value)} />
        </div>

        <div>
          <label style={fieldLabelStyle}>Default OCR Language</label>
          <select className="input" value={ocrLang} onChange={(e) => setOcrLang(e.target.value)}>
            <option value="eng">English (eng)</option>
            <option value="fra">French (fra)</option>
            <option value="deu">German (deu)</option>
            <option value="spa">Spanish (spa)</option>
            <option value="ita">Italian (ita)</option>
          </select>
        </div>

        <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: '8px' }}>
          <button
            className="btn btn-primary"
            onClick={async () => {
              try {
                await api.updateConfig({ app_title: appTitle, language: ocrLang })
                await alertModal({
                  title: 'Settings Saved',
                  message: 'Your settings have been successfully updated.',
                  type: 'success',
                })
              } catch (err: any) {
                await alertModal({
                  title: 'Save Failed',
                  message: err.message || 'Failed to save settings',
                  type: 'danger',
                })
              }
            }}
          >
            Save Settings
          </button>
        </div>
      </div>

      <div className="glass-panel" style={{ padding: '24px', display: 'flex', flexDirection: 'column', gap: '16px' }}>
        <h3 style={{ fontSize: '1rem', fontWeight: 700 }}>System Health & Status</h3>

        <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
          <div style={rowStyle}>
            <span>Database Engine</span>
            <span style={{ fontWeight: 600, color: '#34d399' }}>SQLite Async (WAL)</span>
          </div>
          <div style={rowStyle}>
            <span>Full-Text Search Engine</span>
            <span style={{ fontWeight: 600, color: 'var(--accent-primary)' }}>SQLite FTS5 Inverted Index</span>
          </div>
          <div style={rowStyle}>
            <span>FastAPI Subprocess Worker</span>
            <span style={{ fontWeight: 600, color: workerConnected ? '#34d399' : '#f59e0b' }}>
              {workerConnected ? 'Connected (:8900)' : 'Standalone Worker Mode'}
            </span>
          </div>
        </div>
      </div>

      {/* Sanity Checker Section */}
      <SanityCheckSection />
    </div>
  )
}

function SanityCheckSection() {
  const [running, setRunning] = useState(false)
  const [result, setResult] = useState<{
    passed: boolean
    total_documents: number
    checked_files: number
    messages: Array<{ doc_id: number | null; type: 'error' | 'warning' | 'info'; message: string }>
  } | null>(null)
  const [errorMsg, setErrorMsg] = useState<string | null>(null)

  const runCheck = async () => {
    setRunning(true)
    setErrorMsg(null)
    try {
      const res = await api.getSanityCheck()
      setResult(res)
    } catch (err: any) {
      setErrorMsg(err?.message || 'Failed to execute sanity check')
    } finally {
      setRunning(false)
    }
  }

  const errors = result?.messages.filter((m) => m.type === 'error') || []
  const warnings = result?.messages.filter((m) => m.type === 'warning') || []

  return (
    <div className="glass-panel" style={{ padding: '24px', display: 'flex', flexDirection: 'column', gap: '16px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '12px' }}>
        <div>
          <h3 style={{ fontSize: '1rem', fontWeight: 700, display: 'flex', alignItems: 'center', gap: '8px' }}>
            <HardDrive size={18} style={{ color: 'var(--accent-primary)' }} />
            Storage & Document Sanity Checker
          </h3>
          <p style={{ fontSize: '0.82rem', color: 'var(--text-muted)', marginTop: '2px' }}>
            Validates disk file presence, SHA-256 database checksum matches, and detects orphan files in originals/ and archive/ vaults.
          </p>
        </div>

        <button
          className="btn btn-primary"
          onClick={runCheck}
          disabled={running}
          style={{ fontSize: '0.82rem', padding: '7px 16px', display: 'inline-flex', alignItems: 'center', gap: '6px' }}
        >
          <RefreshCw size={14} className={running ? 'spin' : ''} />
          {running ? 'Verifying Vault...' : 'Run Sanity Check'}
        </button>
      </div>

      {errorMsg && (
        <div style={{ padding: '12px', borderRadius: 'var(--radius-sm)', backgroundColor: 'rgba(239, 68, 68, 0.1)', border: '1px solid rgba(239, 68, 68, 0.25)', color: '#f87171', fontSize: '0.85rem' }}>
          {errorMsg}
        </div>
      )}

      {result && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '14px', marginTop: '4px' }}>
          <div style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '12px 16px',
            borderRadius: 'var(--radius-sm)',
            backgroundColor: result.passed ? 'rgba(16, 185, 129, 0.1)' : 'rgba(239, 68, 68, 0.1)',
            border: `1px solid ${result.passed ? 'rgba(16, 185, 129, 0.3)' : 'rgba(239, 68, 68, 0.3)'}`,
          }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              {result.passed ? (
                <CheckCircle2 size={18} style={{ color: '#34d399' }} />
              ) : (
                <AlertCircle size={18} style={{ color: '#f87171' }} />
              )}
              <span style={{ fontWeight: 600, fontSize: '0.9rem', color: result.passed ? '#34d399' : '#f87171' }}>
                {result.passed
                  ? 'All vault storage integrity checks passed.'
                  : `Sanity check detected ${errors.length} error(s) and ${warnings.length} warning(s).`}
              </span>
            </div>
            <div style={{ display: 'flex', gap: '8px', fontSize: '0.78rem' }}>
              <span style={{ padding: '2px 8px', borderRadius: '4px', backgroundColor: 'rgba(255,255,255,0.06)' }}>
                {result.total_documents} documents
              </span>
              <span style={{ padding: '2px 8px', borderRadius: '4px', backgroundColor: 'rgba(255,255,255,0.06)' }}>
                {result.checked_files} files checked
              </span>
            </div>
          </div>

          {result.messages.length > 0 && (
            <div style={{
              display: 'flex',
              flexDirection: 'column',
              gap: '6px',
              maxHeight: '260px',
              overflowY: 'auto',
              padding: '8px',
              borderRadius: 'var(--radius-sm)',
              backgroundColor: 'rgba(0, 0, 0, 0.2)',
              border: '1px solid var(--border-subtle)',
            }}>
              {result.messages.map((msg, i) => (
                <div
                  key={i}
                  style={{
                    display: 'flex',
                    alignItems: 'flex-start',
                    gap: '8px',
                    padding: '8px 10px',
                    borderRadius: '4px',
                    fontSize: '0.8rem',
                    backgroundColor: msg.type === 'error' ? 'rgba(239, 68, 68, 0.08)' : 'rgba(245, 158, 11, 0.08)',
                    color: msg.type === 'error' ? '#fca5a5' : '#fde68a',
                  }}
                >
                  <span style={{
                    fontWeight: 700,
                    textTransform: 'uppercase',
                    fontSize: '0.7rem',
                    padding: '1px 5px',
                    borderRadius: '3px',
                    backgroundColor: msg.type === 'error' ? 'rgba(239, 68, 68, 0.25)' : 'rgba(245, 158, 11, 0.25)',
                  }}>
                    {msg.type}
                  </span>
                  <span style={{ flex: 1, wordBreak: 'break-word' }}>{msg.message}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
