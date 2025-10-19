import { docs } from '@/.velite'
import { notFound } from 'next/navigation'
import { ProcessedContent } from '@/components/content/ProcessedContent'
import { proseStyles } from '@/lib/prose-styles'

export default function DocsPage() {
  // Find the introduction page or first doc
  const introDoc = docs
    .filter(doc => !doc.draft)
    .sort((a, b) => a.order - b.order)
    .find(doc => doc.slug === '001-introduction') || docs[0]

  if (!introDoc) {
    notFound()
  }

  return (
    <article>
      <ProcessedContent 
        className={proseStyles}
        content={introDoc.content}
      />
    </article>
  )
}