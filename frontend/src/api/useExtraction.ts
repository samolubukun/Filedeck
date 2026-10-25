import { useState, useEffect, useCallback } from 'react'
import { apiFetch } from './client'
import type { ExtractionSchema, ExtractionBatch, BatchGridData } from '../types'

export function useExtractionSchemas() {
  const [schemas, setSchemas] = useState<ExtractionSchema[]>([])
  const [loading, setLoading] = useState<boolean>(true)
  const [error, setError] = useState<string | null>(null)

  const reload = useCallback(async () => {
    try {
      setLoading(true)
      const res = await apiFetch<{ count: number; results: ExtractionSchema[] }>('/extraction/schemas/')
      setSchemas(res.results || [])
      setError(null)
    } catch (err: any) {
      setError(err?.message || 'Failed to load schemas')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    reload()
  }, [reload])

  const createSchema = async (payload: { name: string; description?: string; columns: any[]; validation_rules?: any[] }) => {
    const res = await apiFetch<ExtractionSchema>('/extraction/schemas/', {
      method: 'POST',
      body: JSON.stringify(payload),
    })
    await reload()
    return res
  }

  const deleteSchema = async (id: number) => {
    await apiFetch<void>(`/extraction/schemas/${id}/`, { method: 'DELETE' })
    await reload()
  }

  return { schemas, loading, error, reload, createSchema, deleteSchema }
}

export function useExtractionBatches() {
  const [batches, setBatches] = useState<ExtractionBatch[]>([])
  const [loading, setLoading] = useState<boolean>(true)
  const [error, setError] = useState<string | null>(null)

  const reload = useCallback(async () => {
    try {
      setLoading(true)
      const res = await apiFetch<{ count: number; results: ExtractionBatch[] }>('/extraction/batches/')
      setBatches(res.results || [])
      setError(null)
    } catch (err: any) {
      setError(err?.message || 'Failed to load batches')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    reload()
  }, [reload])

  const createBatch = async (payload: { schema_id: number; title: string; document_ids?: number[] }) => {
    const res = await apiFetch<ExtractionBatch>('/extraction/batches/', {
      method: 'POST',
      body: JSON.stringify(payload),
    })
    await reload()
    return res
  }

  return { batches, loading, error, reload, createBatch }
}

export function useBatchGrid(batchId: number | null) {
  const [gridData, setGridData] = useState<BatchGridData | null>(null)
  const [loading, setLoading] = useState<boolean>(false)
  const [error, setError] = useState<string | null>(null)

  const reload = useCallback(async () => {
    if (!batchId) {
      setGridData(null)
      return
    }
    try {
      setLoading(true)
      const data = await apiFetch<BatchGridData>(`/extraction/batches/${batchId}/grid/`)
      setGridData(data)
      setError(null)
    } catch (err: any) {
      setError(err?.message || 'Failed to load batch review grid')
    } finally {
      setLoading(false)
    }
  }, [batchId])

  useEffect(() => {
    reload()
  }, [reload])

  const updateCell = async (cellId: number, value: string) => {
    const res = await apiFetch<{ id: number; value: string; is_reviewed: boolean }>(`/extraction/cells/${cellId}/`, {
      method: 'PATCH',
      body: JSON.stringify({
        normalized_value: value,
        is_reviewed: true,
      }),
    })
    // Optimistically update grid state
    setGridData((prev) => {
      if (!prev) return null
      const updatedRows = prev.rows.map((row) => {
        const matchingKey = Object.keys(row.cells).find((k) => row.cells[k]?.id === cellId)
        if (matchingKey) {
          return {
            ...row,
            cells: {
              ...row.cells,
              [matchingKey]: {
                ...row.cells[matchingKey],
                value,
                is_reviewed: true,
                validation_error: null,
              },
            },
          }
        }
        return row
      })
      return { ...prev, rows: updatedRows }
    })
    return res
  }

  const approveRecord = async (recordId: number) => {
    await apiFetch(`/extraction/records/${recordId}/approve/`, { method: 'POST' })
    setGridData((prev) => {
      if (!prev) return null
      const updatedRows = prev.rows.map((row) => {
        if (row.record_id === recordId) {
          const approvedCells = { ...row.cells }
          Object.keys(approvedCells).forEach((k) => {
            approvedCells[k] = { ...approvedCells[k], is_reviewed: true }
          })
          return { ...row, status: 'approved', cells: approvedCells }
        }
        return row
      })
      return { ...prev, rows: updatedRows }
    })
  }

  const triggerProcess = async () => {
    if (!batchId) return
    await apiFetch(`/extraction/batches/${batchId}/process/`, { method: 'POST' })
    setTimeout(reload, 1200)
  }

  return { gridData, loading, error, reload, updateCell, approveRecord, triggerProcess }
}
