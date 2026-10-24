import type { TaxonomyItem } from '../types'

interface TagBadgesProps {
  tagIds: number[]
  tags: TaxonomyItem[]
}

export function TagBadges({ tagIds, tags }: TagBadgesProps) {
  return (
    <>
      {tagIds.map((tId) => {
        const tagObj = tags.find((t) => t.id === tId)
        return tagObj ? (
          <span
            key={tId}
            className="badge"
            style={{ backgroundColor: tagObj.color || 'var(--accent-primary)', color: '#fff', fontSize: '0.65rem' }}
          >
            {tagObj.name}
          </span>
        ) : null
      })}
    </>
  )
}
