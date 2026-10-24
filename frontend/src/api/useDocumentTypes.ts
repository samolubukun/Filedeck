import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from './client'

export function useDocumentTypes() {
  const query = useQuery({
    queryKey: ['document_types'],
    queryFn: () => api.getDocumentTypes().catch(() => ({ count: 0, results: [] })),
  })
  return { documentTypes: query.data?.results ?? [], ...query }
}

export function useCreateDocumentType() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: api.createDocumentType,
    onSuccess: () => qc.invalidateQueries(),
  })
}

export function useDeleteDocumentType() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: api.deleteDocumentType,
    onSuccess: () => qc.invalidateQueries(),
  })
}
