import { useState } from 'react'
import { Search, RefreshCw, Upload, Command, Sun, Moon } from 'lucide-react'
import { useUiStore } from '../store/useUiStore'
import { useThemeStore } from '../store/useThemeStore'
import { useRefreshAll } from '../api/useDocuments'

export function Header() {
  const setUploadOpen = useUiStore((s) => s.setUploadOpen)
  const setSearchOpen = useUiStore((s) => s.setSearchOpen)
  const theme = useThemeStore((s) => s.theme)
  const toggleTheme = useThemeStore((s) => s.toggleTheme)
  const refreshAll = useRefreshAll()
  const [isRefreshing, setIsRefreshing] = useState(false)

  const handleRefresh = async () => {
    setIsRefreshing(true)
    try {
      await refreshAll()
    } finally {
      setTimeout(() => setIsRefreshing(false), 500)
    }
  }

  return (
    <header
      style={{
        height: '64px',
        borderBottom: '1px solid var(--border-subtle)',
        backgroundColor: 'var(--bg-glass)',
        backdropFilter: 'blur(16px)',
        WebkitBackdropFilter: 'blur(16px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '0 32px',
        position: 'sticky',
        top: 0,
        zIndex: 40,
        transition: 'background-color 0.25s ease, border-color 0.25s ease',
      }}
    >
      {/* Left balance spacer */}
      <div style={{ flex: 1 }} />

      {/* Centered Global Command / Search Palette Trigger */}
      <div
        onClick={() => setSearchOpen(true)}
        style={{
          position: 'relative',
          width: '100%',
          maxWidth: '520px',
          display: 'flex',
          alignItems: 'center',
          cursor: 'pointer',
          transform: 'translateX(-2cm)',
        }}
      >
        <Search
          size={16}
          style={{
            position: 'absolute',
            left: '14px',
            color: 'var(--text-muted)',
            pointerEvents: 'none',
          }}
        />
        <input
          type="text"
          readOnly
          style={{
            width: '100%',
            height: '40px',
            paddingLeft: '40px',
            paddingRight: '64px',
            borderRadius: 'var(--radius-full)',
            cursor: 'pointer',
            backgroundColor: 'var(--bg-surface)',
            border: '1px solid var(--border-subtle)',
            color: 'var(--text-primary)',
            fontSize: '0.84rem',
            outline: 'none',
            boxShadow: 'var(--shadow-sm)',
            transition: 'all 0.2s ease',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.borderColor = 'var(--accent-primary)'
            e.currentTarget.style.boxShadow = '0 0 0 3px var(--accent-soft)'
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.borderColor = 'var(--border-subtle)'
            e.currentTarget.style.boxShadow = 'var(--shadow-sm)'
          }}
          placeholder="Search documents, tags, content, views..."
        />
        <div
          style={{
            position: 'absolute',
            right: '12px',
            display: 'flex',
            alignItems: 'center',
            gap: '2px',
            fontSize: '0.7rem',
            fontWeight: 700,
            backgroundColor: 'var(--bg-tertiary)',
            border: '1px solid var(--border-subtle)',
            padding: '2px 7px',
            borderRadius: '6px',
            color: 'var(--text-muted)',
            letterSpacing: '0.02em',
          }}
        >
          <Command size={11} style={{ marginRight: '1px' }} />K
        </div>
      </div>

      {/* Header Actions */}
      <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'flex-end', gap: '10px' }}>
        {/* Theme Toggle (Sun / Moon) */}
        <button
          className="btn btn-secondary"
          onClick={toggleTheme}
          title={`Switch to ${theme === 'light' ? 'Dark' : 'Light'} Mode`}
          style={{
            padding: '8px 12px',
            borderRadius: 'var(--radius-full)',
            border: '1px solid var(--border-subtle)',
            background: 'var(--bg-surface)',
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
            fontSize: '0.8rem',
            fontWeight: 600,
            color: 'var(--text-secondary)',
          }}
        >
          {theme === 'light' ? (
            <>
              <Moon size={15} style={{ color: 'var(--accent-primary)' }} />
              <span style={{ fontSize: '0.78rem' }}>Dark</span>
            </>
          ) : (
            <>
              <Sun size={15} style={{ color: '#f59e0b' }} />
              <span style={{ fontSize: '0.78rem' }}>Light</span>
            </>
          )}
        </button>

        <button
          className="btn btn-secondary"
          onClick={handleRefresh}
          title="Refresh Vault Data"
          style={{
            padding: '8px 12px',
            borderRadius: 'var(--radius-full)',
            border: '1px solid var(--border-subtle)',
            background: 'var(--bg-surface)',
          }}
        >
          <RefreshCw
            size={15}
            style={{
              transition: 'transform 0.5s ease',
              transform: isRefreshing ? 'rotate(180deg)' : 'none',
              color: isRefreshing ? 'var(--accent-primary)' : 'var(--text-secondary)',
            }}
          />
        </button>

        <button
          className="btn btn-primary"
          onClick={() => setUploadOpen(true)}
          style={{
            padding: '9px 18px',
            fontSize: '0.84rem',
            gap: '7px',
            borderRadius: 'var(--radius-full)',
            boxShadow: '0 2px 10px var(--accent-glow)',
          }}
        >
          <Upload size={15} /> Upload
        </button>
      </div>
    </header>
  )
}
