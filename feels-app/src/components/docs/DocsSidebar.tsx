'use client'

import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { cn } from '@/lib/utils'
import { docs } from '@/.velite'

export function DocsSidebar() {
  const pathname = usePathname()
  
  // Group docs by category and sort by order
  const categories = docs
    .filter(doc => !doc.draft)
    .reduce((acc, doc) => {
      if (!acc[doc.category]) {
        acc[doc.category] = []
      }
      acc[doc.category]!.push(doc)
      return acc
    }, {} as Record<string, typeof docs>)

  // Sort docs within each category
  Object.values(categories).forEach(categoryDocs => {
    categoryDocs.sort((a, b) => a.order - b.order)
  })

  return (
    <aside className="w-64 bg-background py-8">
      <nav className="space-y-8">
        {Object.entries(categories).map(([category, categoryDocs]) => (
          <div key={category}>
            <h3 className="font-semibold mb-3 text-xs uppercase tracking-wider text-muted-foreground">
              {category}
            </h3>
            <ul className="space-y-1">
              {categoryDocs.map((doc) => {
                const isActive = pathname === doc.permalink || (pathname === '/docs' && doc.slug === '001-introduction')
                return (
                  <li key={doc.slug}>
                    <Link
                      href={doc.permalink}
                      className={cn(
                        "block px-4 py-1 text-sm rounded-lg transition-all duration-200",
                        isActive
                          ? "bg-primary/10 text-primary font-medium"
                          : "hover:bg-muted text-muted-foreground hover:text-foreground"
                      )}
                    >
                      {doc.title}
                    </Link>
                  </li>
                )
              })}
            </ul>
          </div>
        ))}
      </nav>
    </aside>
  )
}