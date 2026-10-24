import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from './client'
import { useFilterStore } from '../store/useFilterStore'

export function useDocuments() {
  const searchQuery = useFilterStore((s) => s.searchQuery)
  const tagFilter = useFilterStore((s) => s.tagFilter)
  const corrFilter = useFilterStore((s) => s.corrFilter)
  const typeFilter = useFilterStore((s) => s.typeFilter)
  const inboxOnly = useFilterStore((s) => s.inboxOnly)

  const query = useQuery({
    queryKey: ['documents', { searchQuery, tagFilter, corrFilter, typeFilter, inboxOnly }],
    queryFn: () =>
      api.getDocuments({
        query: searchQuery || undefined,
        correspondent__id: corrFilter || undefined,
        document_type__id: typeFilter || undefined,
        tags__id__in: tagFilter ? String(tagFilter) : undefined,
        is_in_inbox: inboxOnly ? true : undefined,
      }),
  })

  return {
    documents: query.data?.results ?? [],
    totalDocs: query.data?.count ?? 0,
    ...query,
  }
}

export function useAutocomplete(term: string) {
  return useQuery({
    queryKey: ['autocomplete', term],
    queryFn: () => api.getAutocomplete(term).catch(() => [] as string[]),
    enabled: !!term.trim(),
  })
}

/** Refetch everything (equivalent of the old monolithic loadData). */
export function useRefreshAll() {
  const qc = useQueryClient()
  return () => qc.invalidateQueries()
}
