import { create } from 'zustand'
import type { NavView } from '../types'
import { parseHash, syncHashToState } from '../lib/routing'

interface UiState {
  currentNav: NavView
  isUploadOpen: boolean
  activeDocId: number | null
  wsConnected: boolean
  isSearchOpen: boolean
  sidebarCollapsed: boolean
  setCurrentNav: (nav: NavView) => void
  setUploadOpen: (open: boolean) => void
  setActiveDocId: (id: number | null) => void
  setWsConnected: (connected: boolean) => void
  setSearchOpen: (open: boolean) => void
  setSidebarCollapsed: (collapsed: boolean) => void
  toggleSidebarCollapsed: () => void
  syncFromLocation: () => void
}

const initialSidebar =
  typeof window !== 'undefined'
    ? localStorage.getItem('filedeck_sidebar_collapsed') === 'true'
    : false

const initialHash = parseHash()

export const useUiStore = create<UiState>((set, get) => ({
  currentNav: initialHash.nav,
  isUploadOpen: false,
  activeDocId: initialHash.docId,
  wsConnected: false,
  isSearchOpen: false,
  sidebarCollapsed: initialSidebar,

  setCurrentNav: (currentNav) => {
    const activeDocId = currentNav === 'documents' ? get().activeDocId : null
    syncHashToState(currentNav, activeDocId)
    set({ currentNav, activeDocId })
  },

  setUploadOpen: (isUploadOpen) => set({ isUploadOpen }),

  setActiveDocId: (activeDocId) => {
    const currentNav = get().currentNav
    syncHashToState(currentNav, activeDocId)
    set({ activeDocId })
  },

  setWsConnected: (wsConnected) => set({ wsConnected }),
  setSearchOpen: (isSearchOpen) => set({ isSearchOpen }),

  setSidebarCollapsed: (sidebarCollapsed) => {
    localStorage.setItem('filedeck_sidebar_collapsed', String(sidebarCollapsed))
    set({ sidebarCollapsed })
  },

  toggleSidebarCollapsed: () =>
    set((s) => {
      const next = !s.sidebarCollapsed
      localStorage.setItem('filedeck_sidebar_collapsed', String(next))
      return { sidebarCollapsed: next }
    }),

  syncFromLocation: () => {
    const parsed = parseHash()
    set({
      currentNav: parsed.nav,
      activeDocId: parsed.docId,
    })
  },
}))


