import { useState, useEffect } from 'react'
import { CheckCircle2, Clock, AlertCircle, RefreshCw, CheckCheck, Loader2 } from 'lucide-react'
import { api } from '../api/client'
import { PageHeading } from '../components/TaxonomyManager'
import { alertModal } from '../store/useModalStore'
import type { TaskItem } from '../types'

function formatTaskDetails(task: TaskItem): string {
  if (!task.result_data) {
    const filename = task.input_data?.filename
    return filename ? `File: ${filename}` : `Source: ${task.trigger_source || 'upload'}`
  }
  if (typeof task.result_data === 'string') {
    return task.result_data
  }
  if (typeof task.result_data === 'object') {
    const res = task.result_data as any
    if (res.error) {
      return String(res.error)
    }
    if (res.document_id) {
      return `Document #${res.document_id} processed`
    }
    if (Array.isArray(res.document_ids) && res.document_ids.length > 0) {
      return `Document #${res.document_ids.join(', #')} processed`
    }
    try {
      return JSON.stringify(res)
    } catch {
      return 'Completed'
    }
  }
  return String(task.result_data)
}

export function Tasks() {
  const [tasks, setTasks] = useState<TaskItem[]>([])
  const [loading, setLoading] = useState(true)
  const [statusFilter, setStatusFilter] = useState<string>('')
  const [busy, setBusy] = useState(false)

  const loadTasks = async () => {
    setLoading(true)
    try {
      const data = await api.getTasks(statusFilter || undefined)
      setTasks(data || [])
    } catch (err) {
      console.error(err)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadTasks()
    const timer = setInterval(() => {
      api.getTasks(statusFilter || undefined).then((res) => setTasks(res || []))
    }, 4000)
    return () => clearInterval(timer)
  }, [statusFilter])

  const handleAcknowledge = async (taskIds?: number[]) => {
    const unacked = tasks.filter((t) => !t.acknowledged && (t.status?.toUpperCase() === 'SUCCESS' || t.status?.toUpperCase() === 'FAILURE'))
    const ids = taskIds || unacked.map((t) => t.id)
    if (ids.length === 0) return

    setBusy(true)
    try {
      await api.acknowledgeTasks(ids)
      await loadTasks()
    } catch (err: any) {
      await alertModal({
        title: 'Action Failed',
        message: err.message || 'Failed to acknowledge tasks',
        type: 'danger',
      })
    } finally {
      setBusy(false)
    }
  }

  const getStatusBadge = (status: string = '') => {
    switch (status.toUpperCase()) {
      case 'SUCCESS':
        return (
          <span
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '4px',
              padding: '3px 8px',
              borderRadius: 'var(--radius-full)',
              fontSize: '0.75rem',
              fontWeight: 600,
              backgroundColor: 'rgba(16, 185, 129, 0.15)',
              color: '#10b981',
            }}
          >
            <CheckCircle2 size={12} /> Success
          </span>
        )
      case 'FAILURE':
        return (
          <span
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '4px',
              padding: '3px 8px',
              borderRadius: 'var(--radius-full)',
              fontSize: '0.75rem',
              fontWeight: 600,
              backgroundColor: 'rgba(239, 68, 68, 0.15)',
              color: '#ef4444',
            }}
          >
            <AlertCircle size={12} /> Failed
          </span>
        )
      case 'STARTED':
        return (
          <span
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '4px',
              padding: '3px 8px',
              borderRadius: 'var(--radius-full)',
              fontSize: '0.75rem',
              fontWeight: 600,
              backgroundColor: 'rgba(59, 130, 246, 0.15)',
              color: '#3b82f6',
            }}
          >
            <Loader2 size={12} className="spin" /> In Progress
          </span>
        )
      default:
        return (
          <span
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '4px',
              padding: '3px 8px',
              borderRadius: 'var(--radius-full)',
              fontSize: '0.75rem',
              fontWeight: 600,
              backgroundColor: 'rgba(245, 158, 11, 0.15)',
              color: '#f59e0b',
            }}
          >
            <Clock size={12} /> Pending
          </span>
        )
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <PageHeading
          title="Task Queue"
          subtitle="Monitor background ingestion, OCR processing, and document operations."
        />
        <div style={{ display: 'flex', gap: '10px' }}>
          <button
            className="btn btn-secondary"
            onClick={loadTasks}
            disabled={loading}
            style={{ display: 'flex', alignItems: 'center', gap: '6px' }}
          >
            <RefreshCw size={15} className={loading ? 'spin' : ''} /> Refresh
          </button>
          <button
            className="btn btn-secondary"
            onClick={() => handleAcknowledge()}
            disabled={busy}
            style={{ display: 'flex', alignItems: 'center', gap: '6px' }}
          >
            <CheckCheck size={15} /> Acknowledge All Completed
          </button>
        </div>
      </div>

      {/* Filter Tabs */}
      <div style={{ display: 'flex', gap: '8px' }}>
        {[
          { label: 'All Tasks', value: '' },
          { label: 'In Progress', value: 'STARTED' },
          { label: 'Pending', value: 'PENDING' },
          { label: 'Successful', value: 'SUCCESS' },
          { label: 'Failed', value: 'FAILURE' },
        ].map((tab) => (
          <button
            key={tab.value}
            onClick={() => setStatusFilter(tab.value)}
            style={{
              padding: '6px 14px',
              borderRadius: 'var(--radius-sm)',
              border: 'none',
              background: statusFilter === tab.value ? 'var(--accent-primary)' : 'rgba(255,255,255,0.06)',
              color: statusFilter === tab.value ? '#fff' : 'var(--text-secondary)',
              fontWeight: 600,
              fontSize: '0.825rem',
              cursor: 'pointer',
            }}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {loading && tasks.length === 0 ? (
        <div style={{ padding: '40px', textAlign: 'center', color: 'var(--text-muted)' }}>
          Loading task stream...
        </div>
      ) : tasks.length === 0 ? (
        <div
          className="glass-panel"
          style={{ padding: '48px', textAlign: 'center', color: 'var(--text-muted)' }}
        >
          No tasks matching the selected filter.
        </div>
      ) : (
        <div className="glass-panel" style={{ overflow: 'hidden' }}>
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: '130px 140px 1fr 140px 120px 80px',
              padding: '12px 18px',
              borderBottom: '1px solid var(--border-subtle)',
              backgroundColor: 'rgba(0,0,0,0.15)',
              fontSize: '0.75rem',
              fontWeight: 700,
              textTransform: 'uppercase',
              color: 'var(--text-muted)',
            }}
          >
            <div>Status</div>
            <div>Task Type</div>
            <div>Details / Result</div>
            <div>Started / Done</div>
            <div>Duration</div>
            <div>Action</div>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column' }}>
            {tasks.map((task) => {
              const details = formatTaskDetails(task)
              const isFailed = task.status?.toUpperCase() === 'FAILURE'
              return (
                <div
                  key={task.id}
                  style={{
                    display: 'grid',
                    gridTemplateColumns: '130px 140px 1fr 140px 120px 80px',
                    alignItems: 'center',
                    padding: '14px 18px',
                    borderBottom: '1px solid var(--border-subtle)',
                    fontSize: '0.85rem',
                    gap: '8px',
                  }}
                >
                  <div>{getStatusBadge(task.status)}</div>
                  <div style={{ fontWeight: 600, color: 'var(--text-primary)' }}>
                    {task.task_type || 'Ingestion'}
                  </div>
                  <div
                    style={{
                      color: isFailed ? '#ef4444' : 'var(--text-secondary)',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                    }}
                    title={details}
                  >
                    {details}
                  </div>
                <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                  {task.date_done
                    ? new Date(task.date_done).toLocaleTimeString()
                    : task.date_created
                    ? new Date(task.date_created).toLocaleTimeString()
                    : '-'}
                </div>
                <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>
                  {task.duration_seconds ? `${task.duration_seconds.toFixed(1)}s` : '-'}
                </div>
                <div>
                  {!task.acknowledged && (task.status === 'SUCCESS' || task.status === 'FAILURE') && (
                    <button
                      className="btn btn-secondary"
                      onClick={() => handleAcknowledge([task.id])}
                      style={{ padding: '4px 8px', fontSize: '0.75rem' }}
                    >
                      Ack
                    </button>
                  )}
                </div>
              </div>
            )
          })}
          </div>
        </div>
      )}
    </div>
  )
}
