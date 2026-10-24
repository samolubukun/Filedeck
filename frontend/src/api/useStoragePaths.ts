import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from './client'

export function useStoragePaths() {
  const query = useQuery({
    queryKey: ['storage_paths'],
    queryFn: () => api.getStoragePaths().catch(() => ({ count: 0, results: [] })),
  })
  return { storagePaths: query.data?.results ?? [], ...query }
}

export function useCreateStoragePath() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: api.createStoragePath,
    onSuccess: () => qc.invalidateQueries(),
  })
}

export function useDeleteStoragePath() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: api.deleteStoragePath,
    onSuccess: () => qc.invalidateQueries(),
  })
}
