import React from 'react'

interface LogoProps {
  size?: number
  iconSize?: number
  className?: string
  style?: React.CSSProperties
}

/**
 * FileDeck Signature Icon
 * Uses the official FileDeck.svg asset.
 */
export function FileDeckGlyph({
  size = 24,
  style,
}: {
  size?: number
  color?: string
  strokeWidth?: number
  style?: React.CSSProperties
}) {
  return (
    <img
      src="/FileDeck.svg"
      alt="FileDeck"
      width={size}
      height={size}
      style={{
        width: `${size}px`,
        height: `${size}px`,
        objectFit: 'contain',
        display: 'inline-block',
        ...style,
      }}
    />
  )
}

export function LogoBadge({ size = 36, style, className }: LogoProps) {
  return (
    <img
      src="/FileDeck.svg"
      alt="FileDeck"
      width={size}
      height={size}
      className={className}
      style={{
        width: `${size}px`,
        height: `${size}px`,
        minWidth: `${size}px`,
        objectFit: 'contain',
        borderRadius: `${Math.round(size * 0.22)}px`,
        filter: 'drop-shadow(0 2px 8px rgba(37, 99, 235, 0.32))',
        transition: 'transform 0.2s cubic-bezier(0.16, 1, 0.3, 1), filter 0.2s ease',
        ...style,
      }}
    />
  )
}

/**
 * FileDeck brand wordmark with "File" in primary text and "Deck" with
 * the vibrant electric blue-to-purple gradient matching the official identity.
 */
export function FileDeckText({
  fontSize = '1.25rem',
  style,
  className,
}: {
  fontSize?: string | number
  style?: React.CSSProperties
  className?: string
}) {
  const size = typeof fontSize === 'number' ? `${fontSize}px` : fontSize
  return (
    <span
      className={className}
      style={{
        fontSize: size,
        fontWeight: 800,
        letterSpacing: '-0.025em',
        display: 'inline-flex',
        alignItems: 'baseline',
        lineHeight: 1.15,
        userSelect: 'none',
        ...style,
      }}
    >
      <span style={{ color: 'var(--text-primary)' }}>File</span>
      <span
        style={{
          background: 'linear-gradient(92deg, #0080FF 0%, #1A6CFF 35%, #5E3BE8 75%, #8B2BE2 100%)',
          WebkitBackgroundClip: 'text',
          WebkitTextFillColor: 'transparent',
          backgroundClip: 'text',
          display: 'inline-block',
        }}
      >
        Deck
      </span>
    </span>
  )
}

