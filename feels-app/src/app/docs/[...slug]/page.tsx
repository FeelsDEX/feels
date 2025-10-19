'use client';

import { notFound } from 'next/navigation'
import { docs } from '@/.velite'
import { proseStyles } from '@/lib/prose-styles'
import { ProcessedContent } from '@/components/content/ProcessedContent'
import { useParams } from 'next/navigation'

export default function DocsArticlePage() {
  const params = useParams()
  const slugArray = Array.isArray(params['slug']) ? params['slug'] : [params['slug']]
  const slug = slugArray.join('/')
  const doc = docs.find((d) => d.slug === slug)
  
  if (!doc || doc.draft) {
    notFound()
  }

  return (
    <article>
      <ProcessedContent 
        className={proseStyles}
        content={doc.content}
      />
    </article>
  )
}