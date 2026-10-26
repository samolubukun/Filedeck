import React, { useState, useEffect } from 'react'
import { Minus, Square, Copy, X } from 'lucide-react'

export const TitleBar: React.FC = () => {
  const [isTauri, setIsTauri] = useState(false)
  const [isMaximized, setIsMaximized] = useState(false)

  useEffect(() => {
    // Detect if running inside Tauri native webview
    const hasTauri = typeof window !== 'undefined' && ('__TAURI_INTERNALS__' in window || '__TAURI__' in window)
    setIsTauri(hasTauri)
  }, [])

  const handleMinimize = async () => {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      await getCurrentWindow().minimize()
    } catch (e) {
      console.warn('Minimize not supported in web browser', e)
    }
  }

  const handleToggleMaximize = async () => {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      const win = getCurrentWindow()
      await win.toggleMaximize()
      const max = await win.isMaximized()
      setIsMaximized(max)
    } catch (e) {
      console.warn('Maximize not supported in web browser', e)
    }
  }

  const handleClose = async () => {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      await getCurrentWindow().close()
    } catch (e) {
      console.warn('Close not supported in web browser', e)
    }
  }

  // If running in pure web browser, don't show native window controls
  if (!isTauri) return null

  return (
    <header
      data-tauri-drag-region
      style={{
        height: '34px',
        backgroundColor: '#090d16',
        borderBottom: '1px solid rgba(255, 255, 255, 0.07)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        paddingLeft: '12px',
        userSelect: 'none',
        zIndex: 9999,
        position: 'sticky',
        top: 0,
      }}
    >
      {/* Brand & Title */}
      <div
        data-tauri-drag-region
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '8px',
          color: '#e2e8f0',
          fontSize: '0.8rem',
          fontWeight: 600,
          letterSpacing: '0.02em',
        }}
      >
        <div
          style={{
            width: '14px',
            height: '14px',
            borderRadius: '3px',
            background: 'linear-gradient(135deg, #3b82f6 0%, #0ea5e9 100%)',
          }}
        />
        <span>FileDeck Desktop</span>
        <span style={{ fontSize: '0.68rem', color: '#64748b', fontWeight: 400 }}>v0.1.0</span>
      </div>

      {/* Center Drag Region */}
      <div data-tauri-drag-region style={{ flex: 1, height: '100%' }} />

      {/* Window Controls */}
      <div style={{ display: 'flex', alignItems: 'center', height: '100%' }}>
        <button
          type="button"
          onClick={handleMinimize}
          title="Minimize"
          style={{
            width: '42px',
            height: '100%',
            border: 'none',
            background: 'transparent',
            color: '#94a3b8',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            cursor: 'pointer',
            transition: 'background-color 0.15s, color 0.15s',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.backgroundColor = 'rgba(255, 255, 255, 0.08)'
            e.currentTarget.style.color = '#ffffff'
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.backgroundColor = 'transparent'
            e.currentTarget.style.color = '#94a3b8'
          }}
        >
          <Minus size={13} />
        </button>

        <button
          type="button"
          onClick={handleToggleMaximize}
          title={isMaximized ? 'Restore' : 'Maximize'}
          style={{
            width: '42px',
            height: '100%',
            border: 'none',
            background: 'transparent',
            color: '#94a3b8',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            cursor: 'pointer',
            transition: 'background-color 0.15s, color 0.15s',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.backgroundColor = 'rgba(255, 255, 255, 0.08)'
            e.currentTarget.style.color = '#ffffff'
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.backgroundColor = 'transparent'
            e.currentTarget.style.color = '#94a3b8'
          }}
        >
          {isMaximized ? <Copy size={11} /> : <Square size={11} />}
        </button>

        <button
          type="button"
          onClick={handleClose}
          title="Close FileDeck"
          style={{
            width: '46px',
            height: '100%',
            border: 'none',
            background: 'transparent',
            color: '#94a3b8',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            cursor: 'pointer',
            transition: 'background-color 0.15s, color 0.15s',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.backgroundColor = '#e11d48'
            e.currentTarget.style.color = '#ffffff'
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.backgroundColor = 'transparent'
            e.currentTarget.style.color = '#94a3b8'
          }}
        >
          <X size={14} />
        </button>
      </div>
    </header>
  )
}
