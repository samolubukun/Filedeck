import { create } from 'zustand'

export type ThemeMode = 'light' | 'dark'

interface ThemeState {
  theme: ThemeMode
  setTheme: (theme: ThemeMode) => void
  toggleTheme: () => void
}

const getInitialTheme = (): ThemeMode => {
  if (typeof window === 'undefined') return 'light'
  const saved = localStorage.getItem('filedeck-theme') as ThemeMode | null
  if (saved === 'light' || saved === 'dark') return saved
  return window.matchMedia?.('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

export const useThemeStore = create<ThemeState>((set, get) => {
  const initial = getInitialTheme()
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('data-theme', initial)
  }

  return {
    theme: initial,
    setTheme: (theme) => {
      localStorage.setItem('filedeck-theme', theme)
      document.documentElement.setAttribute('data-theme', theme)
      set({ theme })
    },
    toggleTheme: () => {
      const next = get().theme === 'light' ? 'dark' : 'light'
      localStorage.setItem('filedeck-theme', next)
      document.documentElement.setAttribute('data-theme', next)
      set({ theme: next })
    },
  }
})
