import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from './client'

export function useTags() {
  const query = useQuery({
    queryKey: ['tags'],
    queryFn: () => api.getTags().catch(() => ({ count: 0, results: [] })),
  })
  return { tags: query.data?.results ?? [], ...query }
}

export function useCreateTag() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: api.createTag,
    onSuccess: () => qc.invalidateQueries(),
  })
}

export function useDeleteTag() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: api.deleteTag,
    onSuccess: () => qc.invalidateQueries(),
  })
}
