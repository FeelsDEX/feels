'use client'

import { useState, useEffect, useMemo } from 'react'
import { docs } from '@/.velite'

interface SearchResult {
  title: string
  description: string
  slug: string
  category: string
  permalink: string
  excerpt?: string
  score?: number
}

export function useDocsSearch(query: string) {
  const [results, setResults] = useState<SearchResult[]>([])
  const [loading, setLoading] = useState(false)

  // Filter out drafts and non-searchable docs
  const searchableDocs = useMemo(() => 
    docs.filter(doc => !doc.draft && doc.searchable),
    []
  )

  useEffect(() => {
    if (!query || query.length < 2) {
      setResults([])
      return
    }

    setLoading(true)
    
    // Simple search implementation
    const searchQuery = query.toLowerCase()
    const searchResults: SearchResult[] = []

    searchableDocs.forEach(doc => {
      let score = 0
      
      // Title match (highest weight)
      if (doc.title.toLowerCase().includes(searchQuery)) {
        score += 10
        if (doc.title.toLowerCase().startsWith(searchQuery)) {
          score += 5
        }
      }

      // Description match
      if (doc.description.toLowerCase().includes(searchQuery)) {
        score += 5
      }

      // Content match (use excerpt for performance)
      const contentSnippet = doc.content.substring(0, 1000).toLowerCase()
      if (contentSnippet.includes(searchQuery)) {
        score += 2
      }

      // Category match
      if (doc.category.toLowerCase().includes(searchQuery)) {
        score += 3
      }

      if (score > 0) {
        searchResults.push({
          ...doc,
          score,
          excerpt: getExcerpt(doc.content, searchQuery)
        })
      }
    })

    // Sort by score
    searchResults.sort((a, b) => (b.score || 0) - (a.score || 0))
    
    setResults(searchResults.slice(0, 10)) // Limit to top 10 results
    setLoading(false)
  }, [query, searchableDocs])

  return { results, loading }
}

function getExcerpt(content: string, query: string): string {
  // Strip HTML tags and normalize whitespace
  const plainText = content
    .replace(/<[^>]*>/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
  
  const lowerText = plainText.toLowerCase()
  const lowerQuery = query.toLowerCase()
  const index = lowerText.indexOf(lowerQuery)
  
  if (index === -1) {
    // If exact match not found, return beginning of content
    return plainText.substring(0, 150) + (plainText.length > 150 ? '...' : '')
  }
  
  // Find word boundaries for cleaner excerpts
  const contextLength = 60
  let start = Math.max(0, index - contextLength)
  let end = Math.min(plainText.length, index + query.length + contextLength)
  
  // Adjust to word boundaries
  if (start > 0) {
    const spaceIndex = plainText.lastIndexOf(' ', start)
    if (spaceIndex > start - 20) start = spaceIndex + 1
  }
  
  if (end < plainText.length) {
    const spaceIndex = plainText.indexOf(' ', end)
    if (spaceIndex !== -1 && spaceIndex < end + 20) end = spaceIndex
  }
  
  let excerpt = plainText.substring(start, end).trim()
  
  // Add ellipsis
  if (start > 0) excerpt = '...' + excerpt
  if (end < plainText.length) excerpt = excerpt + '...'
  
  // Highlight the match by wrapping in markers (will be rendered differently)
  const matchStart = excerpt.toLowerCase().indexOf(lowerQuery)
  if (matchStart !== -1) {
    excerpt = 
      excerpt.substring(0, matchStart) + 
      '**' + excerpt.substring(matchStart, matchStart + query.length) + '**' +
      excerpt.substring(matchStart + query.length)
  }
  
  return excerpt
}