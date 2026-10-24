import { useState, useEffect } from 'react'
import { Terminal, RefreshCw, AlertTriangle, Info } from 'lucide-react'
import { api } from '../api/client'
import { PageHeading } from '../components/TaxonomyManager'

interface LogEntry {
  timestamp: string
  level: string
  message: string
}

export function Logs() {
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [loading, setLoading] = useState(true)

  const loadLogs = async () => {
    setLoading(true)
    try {
      const data = await api.getLogs()
      setLogs(Array.isArray(data) ? data : [])
    } catch (err) {
      console.error(err)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadLogs()
  }, [])

  const getLevelBadge = (level: string) => {
    const l = level.toUpperCase()
    if (l === 'ERROR') {
      return (
        <span style={{ color: '#ef4444', fontWeight: 700, display: 'inline-flex', alignItems: 'center', gap: '4px' }}>
          <AlertTriangle size={13} /> ERROR
        </span>
      )
    }
    if (l === 'WARN' || l === 'WARNING') {
      return (
        <span style={{ color: '#f59e0b', fontWeight: 700, display: 'inline-flex', alignItems: 'center', gap: '4px' }}>
          <AlertTriangle size={13} /> WARN
        </span>
      )
    }
    return (
      <span style={{ color: '#38bdf8', fontWeight: 700, display: 'inline-flex', alignItems: 'center', gap: '4px' }}>
        <Info size={13} /> INFO
      </span>
    )
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <PageHeading
          title="System Logs"
          subtitle="Real-time log stream from the FileDeck server and background workers."
        />
        <button
          className="btn btn-secondary"
          onClick={loadLogs}
          disabled={loading}
          style={{ display: 'flex', alignItems: 'center', gap: '6px' }}
        >
          <RefreshCw size={15} className={loading ? 'spin' : ''} /> Refresh Logs
        </button>
      </div>

      <div
        className="glass-panel"
        style={{
          fontFamily: 'monospace',
          backgroundColor: '#0a0d14',
          borderRadius: 'var(--radius-md)',
          padding: '20px',
          overflowX: 'auto',
          fontSize: '0.85rem',
          minHeight: '400px',
          border: '1px solid rgba(255, 255, 255, 0.08)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--text-muted)', marginBottom: '16px', borderBottom: '1px solid rgba(255,255,255,0.06)', paddingBottom: '12px' }}>
          <Terminal size={16} />
          <span>filedeck-service.log</span>
        </div>

        {loading ? (
          <div style={{ color: 'var(--text-muted)', padding: '20px 0' }}>Loading logs...</div>
        ) : logs.length === 0 ? (
          <div style={{ color: 'var(--text-muted)', padding: '20px 0' }}>No log entries recorded yet.</div>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
            {logs.map((entry, idx) => (
              <div
                key={idx}
                style={{
                  display: 'grid',
                  gridTemplateColumns: '180px 90px 1fr',
                  gap: '12px',
                  lineHeight: '1.5',
                  borderBottom: '1px solid rgba(255,255,255,0.03)',
                  paddingBottom: '4px',
                }}
              >
                <span style={{ color: 'var(--text-muted)' }}>
                  {new Date(entry.timestamp).toLocaleTimeString()}
                </span>
                <div>{getLevelBadge(entry.level)}</div>
                <div style={{ color: '#e2e8f0', wordBreak: 'break-all' }}>{entry.message}</div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
