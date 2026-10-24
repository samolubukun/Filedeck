import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from './client'

export function useCorrespondents() {
  const query = useQuery({
    queryKey: ['correspondents'],
    queryFn: () => api.getCorrespondents().catch(() => ({ count: 0, results: [] })),
  })
  return { correspondents: query.data?.results ?? [], ...query }
}

export function useCreateCorrespondent() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: api.createCorrespondent,
    onSuccess: () => qc.invalidateQueries(),
  })
}

export function useDeleteCorrespondent() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: api.deleteCorrespondent,
    onSuccess: () => qc.invalidateQueries(),
  })
}
