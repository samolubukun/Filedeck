import type { LucideIcon } from 'lucide-react'

interface StatCardProps {
  label: string
  value: number | string
  icon?: LucideIcon
  color?: string
  accentBg?: string
  subtitle?: string
  trend?: string
  onClick?: () => void
}

export function StatCard({
  label,
  value,
  icon: Icon,
  color = 'var(--accent-primary)',
  accentBg = 'var(--accent-soft)',
  subtitle,
  trend,
  onClick,
}: StatCardProps) {
  return (
    <div
      className="card-panel card-panel-interactive"
      onClick={onClick}
      style={{
        padding: '22px 24px',
        position: 'relative',
        overflow: 'hidden',
        cursor: onClick ? 'pointer' : 'default',
        borderRadius: 'var(--radius-lg)',
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'space-between',
        minHeight: '140px',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '14px' }}>
        {Icon && (
          <div
            style={{
              width: '42px',
              height: '42px',
              borderRadius: '12px',
              background: accentBg,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: color,
            }}
          >
            <Icon size={20} />
          </div>
        )}
        {trend && (
          <span
            style={{
              fontSize: '0.72rem',
              fontWeight: 700,
              color: 'var(--success)',
              backgroundColor: 'rgba(16, 185, 129, 0.1)',
              padding: '3px 8px',
              borderRadius: 'var(--radius-full)',
            }}
          >
            {trend}
          </span>
        )}
      </div>

      <div>
        <div
          style={{
            fontSize: '0.85rem',
            fontWeight: 600,
            color: 'var(--text-secondary)',
            marginBottom: '4px',
          }}
        >
          {label}
        </div>

        <div style={{ display: 'flex', alignItems: 'baseline', gap: '8px' }}>
          <div
            style={{
              fontSize: '2.1rem',
              fontWeight: 800,
              letterSpacing: '-0.03em',
              color: 'var(--text-primary)',
              fontVariantNumeric: 'tabular-nums',
              lineHeight: 1.1,
            }}
          >
            {value}
          </div>
        </div>

        {subtitle && (
          <div style={{ fontSize: '0.78rem', color: 'var(--text-muted)', marginTop: '6px', fontWeight: 500 }}>
            {subtitle}
          </div>
        )}
      </div>
    </div>
  )
}
