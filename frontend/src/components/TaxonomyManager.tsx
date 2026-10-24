import type { ReactNode } from 'react'
import { Trash2 } from 'lucide-react'

export const fieldLabelStyle = {
  display: 'block',
  fontSize: '0.8rem',
  fontWeight: 600,
  color: 'var(--text-secondary)',
  marginBottom: '6px',
} as const

export function PageHeading({ title, subtitle }: { title: string; subtitle: string }) {
  return (
    <div>
      <h1 style={{ fontSize: '1.6rem', fontWeight: 800, letterSpacing: '-0.02em', color: 'var(--text-primary)' }}>
        {title}
      </h1>
      <p style={{ color: 'var(--text-secondary)', fontSize: '0.88rem', marginTop: '4px' }}>{subtitle}</p>
    </div>
  )
}

export function AddCard({ children }: { children: ReactNode }) {
  return (
    <div className="card-panel" style={{ padding: '22px 24px', display: 'flex', alignItems: 'flex-end', gap: '16px', borderRadius: 'var(--radius-lg)' }}>
      {children}
    </div>
  )
}

export function DeleteIconButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      style={{
        background: 'transparent',
        border: 'none',
        color: 'var(--text-muted)',
        cursor: 'pointer',
        padding: '5px',
        borderRadius: 'var(--radius-sm)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        transition: 'color 0.15s ease',
      }}
      onMouseEnter={(e) => {
        e.currentTarget.style.color = 'var(--danger)'
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.color = 'var(--text-muted)'
      }}
    >
      <Trash2 size={15} />
    </button>
  )
}
