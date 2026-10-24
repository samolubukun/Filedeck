import { useEffect } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from './client'
import { useUiStore } from '../store/useUiStore'

export function useStats() {
  return useQuery({
    queryKey: ['stats'],
    queryFn: () => api.getStats().catch(() => null),
  })
}

export function useSystemStatus() {
  return useQuery({
    queryKey: ['status'],
    queryFn: () => api.getStatus().catch(() => null),
  })
}

export function useTasks() {
  return useQuery({
    queryKey: ['tasks'],
    queryFn: () => api.getTasks().catch(() => []),
  })
}

/** Opens the status WebSocket and refreshes all data on relevant messages. */
export function useLiveStatus() {
  const qc = useQueryClient()
  const setWsConnected = useUiStore((s) => s.setWsConnected)

  useEffect(() => {
    let ws: WebSocket | null = null
    try {
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
      ws = new WebSocket(`${protocol}//${window.location.host}/ws/status/`)

      ws.onopen = () => setWsConnected(true)
      ws.onclose = () => setWsConnected(false)
      ws.onerror = () => setWsConnected(false)
      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data)
          if (
                      msg.type === 'document_created' ||
                      msg.type === 'task_progress' ||
                      (msg.type === 'status_update' &&
                        (msg.data?.status === 'SUCCESS' || msg.data?.status === 'FAILED'))
                    ) {
            qc.invalidateQueries()
          }
        } catch {
          // ignore malformed messages
        }
      }
    } catch {
      setWsConnected(false)
    }

    return () => {
      ws?.close()
    }
  }, [qc, setWsConnected])
}
