import { useState, useEffect, useRef } from 'react'
import { AlertTriangle, Info, CheckCircle2, X } from 'lucide-react'
import { useModalStore } from '../store/useModalStore'

export function GlobalModal() {
  const {
    isOpen,
    modalType,
    title,
    message,
    confirmText,
    cancelText,
    danger,
    alertType,
    promptPlaceholder,
    promptValue: initialPromptValue,
    close,
    submitPrompt,
    resolveConfirm,
  } = useModalStore()

  const [inputVal, setInputVal] = useState('')
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (isOpen && modalType === 'prompt') {
      setInputVal(initialPromptValue || '')
      setTimeout(() => inputRef.current?.focus(), 50)
    }
  }, [isOpen, modalType, initialPromptValue])

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isOpen) return
      if (e.key === 'Escape') {
        close()
      } else if (e.key === 'Enter') {
        if (modalType === 'confirm') {
          if (resolveConfirm) resolveConfirm(true)
          close()
        } else if (modalType === 'alert') {
          close()
        } else if (modalType === 'prompt') {
          submitPrompt(inputVal)
        }
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [isOpen, modalType, inputVal, close, resolveConfirm, submitPrompt])

  if (!isOpen) return null

  const renderIcon = () => {
    if (danger || alertType === 'error') {
      return (
        <div
          style={{
            width: '42px',
            height: '42px',
            borderRadius: '12px',
            backgroundColor: 'rgba(239, 68, 68, 0.12)',
            color: '#ef4444',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            flexShrink: 0,
          }}
        >
          <AlertTriangle size={22} />
        </div>
      )
    }
    if (alertType === 'success') {
      return (
        <div
          style={{
            width: '42px',
            height: '42px',
            borderRadius: '12px',
            backgroundColor: 'rgba(16, 185, 129, 0.12)',
            color: '#10b981',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            flexShrink: 0,
          }}
        >
          <CheckCircle2 size={22} />
        </div>
      )
    }
    return (
      <div
        style={{
          width: '42px',
          height: '42px',
          borderRadius: '12px',
          backgroundColor: 'rgba(59, 130, 246, 0.12)',
          color: 'var(--accent-primary)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          flexShrink: 0,
        }}
      >
        <Info size={22} />
      </div>
    )
  }

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 9999,
        backgroundColor: 'rgba(15, 23, 42, 0.65)',
        backdropFilter: 'blur(8px)',
        WebkitBackdropFilter: 'blur(8px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: '16px',
        animation: 'fadeIn 0.15s ease-out',
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) close()
      }}
    >
      <div
        style={{
          width: '100%',
          maxWidth: '460px',
          backgroundColor: 'var(--bg-surface)',
          borderRadius: '16px',
          border: '1px solid var(--border-subtle)',
          boxShadow: '0 25px 50px -12px rgba(0, 0, 0, 0.25)',
          overflow: 'hidden',
          display: 'flex',
          flexDirection: 'column',
          animation: 'scaleIn 0.15s ease-out',
        }}
      >
        {/* Modal Header & Content */}
        <div style={{ padding: '24px 24px 20px', display: 'flex', gap: '16px', alignItems: 'flex-start' }}>
          {renderIcon()}

          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '6px' }}>
              <h2 style={{ margin: 0, fontSize: '1.1rem', fontWeight: 700, color: 'var(--text-primary)' }}>
                {title}
              </h2>
              <button
                onClick={close}
                style={{
                  background: 'none',
                  border: 'none',
                  color: 'var(--text-muted)',
                  cursor: 'pointer',
                  padding: '4px',
                  borderRadius: '6px',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                }}
              >
                <X size={16} />
              </button>
            </div>

            <p style={{ margin: 0, fontSize: '0.88rem', color: 'var(--text-secondary)', lineHeight: 1.45 }}>
              {message}
            </p>

            {modalType === 'prompt' && (
              <div style={{ marginTop: '16px' }}>
                <input
                  ref={inputRef}
                  type="text"
                  className="input"
                  value={inputVal}
                  onChange={(e) => setInputVal(e.target.value)}
                  placeholder={promptPlaceholder}
                  style={{ width: '100%', fontSize: '0.9rem', boxSizing: 'border-box' }}
                />
              </div>
            )}
          </div>
        </div>

        {/* Modal Actions Footer */}
        <div
          style={{
            padding: '14px 24px',
            backgroundColor: 'var(--bg-secondary)',
            borderTop: '1px solid var(--border-subtle)',
            display: 'flex',
            justifyContent: 'flex-end',
            alignItems: 'center',
            gap: '10px',
          }}
        >
          {modalType !== 'alert' && (
            <button
              className="btn btn-secondary"
              onClick={close}
              style={{ fontSize: '0.85rem', padding: '7px 16px', fontWeight: 500 }}
            >
              {cancelText || 'Cancel'}
            </button>
          )}

          <button
            className={`btn ${danger ? 'btn-danger' : 'btn-primary'}`}
            onClick={() => {
              if (modalType === 'confirm') {
                if (resolveConfirm) resolveConfirm(true)
                close()
              } else if (modalType === 'alert') {
                close()
              } else if (modalType === 'prompt') {
                submitPrompt(inputVal)
              }
            }}
            style={{
              fontSize: '0.85rem',
              padding: '7px 18px',
              fontWeight: 600,
              backgroundColor: danger ? '#ef4444' : undefined,
              borderColor: danger ? '#ef4444' : undefined,
              color: '#ffffff',
            }}
          >
            {confirmText || 'Confirm'}
          </button>
        </div>
      </div>
    </div>
  )
}
