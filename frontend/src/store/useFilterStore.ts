import { create } from 'zustand'
import type { ViewMode } from '../types'

interface FilterState {
  searchQuery: string
  tagFilter: number | ''
  corrFilter: number | ''
  typeFilter: number | ''
  inboxOnly: boolean
  viewMode: ViewMode
  selectedSavedViewId: number | null
  selectedDocIds: number[]
  setSelectedSavedViewId: (id: number | null) => void
  applySavedView: (view: any) => void
  resetFilters: () => void
  setSearchQuery: (q: string) => void
  setTagFilter: (v: number | '') => void
  setCorrFilter: (v: number | '') => void
  setTypeFilter: (v: number | '') => void
  setInboxOnly: (v: boolean) => void
  setViewMode: (v: ViewMode) => void
  setSelectedDocIds: (ids: number[]) => void
  toggleSelectedDoc: (id: number) => void
}

export const useFilterStore = create<FilterState>((set) => ({
  searchQuery: '',
  tagFilter: '',
  corrFilter: '',
  typeFilter: '',
  inboxOnly: false,
  viewMode: 'cards',
  selectedDocIds: [],
  selectedSavedViewId: null,
  setSelectedSavedViewId: (selectedSavedViewId) => set({ selectedSavedViewId }),
  applySavedView: (view: any) => {
    if (!view) {
      set({
        selectedSavedViewId: null,
        searchQuery: '',
        tagFilter: '',
        corrFilter: '',
        typeFilter: '',
        inboxOnly: false,
      })
      return
    }

    let tag: number | '' = ''
    let corr: number | '' = ''
    let dtype: number | '' = ''
    let search = ''
    let inbox = false

    if (Array.isArray(view.filter_rules)) {
      for (const rule of view.filter_rules) {
        const valStr = String(rule.value ?? '')
        if (rule.rule_type === 13) {
          const tid = parseInt(valStr, 10)
          if (!isNaN(tid)) tag = tid
        } else if (rule.rule_type === 17) {
          const cid = parseInt(valStr, 10)
          if (!isNaN(cid)) corr = cid
        } else if (rule.rule_type === 19) {
          const dtid = parseInt(valStr, 10)
          if (!isNaN(dtid)) dtype = dtid
        } else if (rule.rule_type === 20) {
          inbox = valStr === 'true' || valStr === '1'
        } else if (rule.rule_type === 0 || rule.rule_type === 1 || rule.rule_type === 22) {
          search = valStr
        }
      }
    }

    set({
      selectedSavedViewId: view.id,
      tagFilter: tag,
      corrFilter: corr,
      typeFilter: dtype,
      searchQuery: search,
      inboxOnly: inbox,
      viewMode: (view.display_mode as ViewMode) || 'cards',
      selectedDocIds: [],
    })
  },
  resetFilters: () =>
    set({
      searchQuery: '',
      tagFilter: '',
      corrFilter: '',
      typeFilter: '',
      inboxOnly: false,
      selectedSavedViewId: null,
    }),
  setSearchQuery: (searchQuery) => set({ searchQuery }),
  setTagFilter: (tagFilter) => set({ tagFilter }),
  setCorrFilter: (corrFilter) => set({ corrFilter }),
  setTypeFilter: (typeFilter) => set({ typeFilter }),
  setInboxOnly: (inboxOnly) => set({ inboxOnly }),
  setViewMode: (viewMode) => set({ viewMode }),
  setSelectedDocIds: (selectedDocIds) => set({ selectedDocIds }),
  toggleSelectedDoc: (id) =>
    set((s) => ({
      selectedDocIds: s.selectedDocIds.includes(id)
        ? s.selectedDocIds.filter((i) => i !== id)
        : [...s.selectedDocIds, id],
    })),
}))
