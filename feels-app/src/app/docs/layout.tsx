'use client';

import { DocsSidebar } from '@/components/docs/DocsSidebar'
import { DocsSearchDropdown } from '@/components/docs/DocsSearchDropdown'
import { useState, useEffect, useRef } from 'react'
import { useDocsSearch } from '@/hooks/useDocsSearch'
import { FileSearch, X, Menu } from 'lucide-react'

export default function DocsLayout({
  children,
}: {
  children: React.ReactNode
}) {
  const [localSearchQuery, setLocalSearchQuery] = useState('')
  const [showDropdown, setShowDropdown] = useState(false)
  const [searchFocused, setSearchFocused] = useState(false)
  const [sidebarOpen, setSidebarOpen] = useState(false)
  const { results, loading } = useDocsSearch(localSearchQuery)
  const searchRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)

  // Handle click outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (searchRef.current && !searchRef.current.contains(e.target as Node)) {
        setShowDropdown(false)
        setSearchFocused(false)
      }
    }
    
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [])

  // Handle keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // "/" to focus search (like main nav search)
      if (e.key === '/' && !['INPUT', 'TEXTAREA'].includes((e.target as HTMLElement).tagName)) {
        e.preventDefault()
        inputRef.current?.focus()
      }
      // Escape to close dropdown
      if (e.key === 'Escape') {
        setShowDropdown(false)
        setSearchFocused(false)
        inputRef.current?.blur()
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [])

  // Update dropdown visibility when search query changes
  useEffect(() => {
    if (localSearchQuery) {
      setShowDropdown(true)
    } else {
      setShowDropdown(false)
    }
  }, [localSearchQuery])

  const clearSearch = () => {
    setLocalSearchQuery('')
    setShowDropdown(false)
    setSearchFocused(false)
  }

  return (
    <>
      {/* Mobile Header with Sidebar Toggle */}
      <div className="md:hidden container mx-auto px-4 py-4 border-b border-border">
        <div className="flex items-center gap-4">
          <button
            onClick={() => setSidebarOpen(!sidebarOpen)}
            className="p-2 hover:bg-muted rounded-md"
            aria-label="Toggle sidebar"
          >
            <Menu size={20} />
          </button>
          
          {/* Mobile Search */}
          <div ref={searchRef} className="flex-1 relative">
            <form onSubmit={(e) => e.preventDefault()} className="relative">
              <div 
                className={`relative flex items-center bg-white border rounded-lg transition-all duration-150 ${
                  searchFocused ? 'border-primary' : 'border-border'
                }`}
                style={{
                  boxShadow: searchFocused 
                    ? '0 0 12px 2px rgba(92, 202, 57, 0.15)' 
                    : 'none'
                }}
              >
                <FileSearch className="h-4 w-4 text-muted-foreground ml-3" />
                <input
                  ref={inputRef}
                  type="text"
                  value={localSearchQuery}
                  onChange={(e) => setLocalSearchQuery(e.target.value)}
                  onFocus={() => setSearchFocused(true)}
                  placeholder="Search docs..."
                  className="flex-1 bg-transparent px-3 py-2 text-sm placeholder:text-muted-foreground focus:outline-none"
                  autoComplete="off"
                />
                {localSearchQuery && (
                  <button
                    type="button"
                    onClick={clearSearch}
                    className="p-2 hover:bg-muted/10 rounded-md transition-colors mr-1"
                  >
                    <X className="h-3 w-3 text-muted-foreground" />
                  </button>
                )}
              </div>
            </form>
            
            {/* Mobile Search Dropdown */}
            {showDropdown && (
              <DocsSearchDropdown
                results={results}
                isLoading={loading}
                searchQuery={localSearchQuery}
                onClose={() => {
                  setShowDropdown(false)
                  setLocalSearchQuery('')
                }}
              />
            )}
          </div>
        </div>
      </div>

      {/* Desktop Search Bar - positioned like global search */}
      <div className="hidden md:block container mx-auto px-4 relative z-[1001] pointer-events-none">
        <div className="flex items-center h-16 -mt-16">
          {/* Left spacer - same as NavBar */}
          <div className="flex-1" />
          
          {/* Center - Search (same positioning as NavBar) */}
          <div className="flex-1 max-w-xl mx-8 pointer-events-auto">
            <div ref={searchRef} className="relative">
              <form onSubmit={(e) => e.preventDefault()} className="relative">
                <div 
                  className={`relative flex items-center bg-white border rounded-lg transition-all duration-150 ${
                    searchFocused ? 'border-primary' : 'border-border'
                  }`}
                  style={{
                    boxShadow: searchFocused 
                      ? '0 0 12px 2px rgba(92, 202, 57, 0.15)' 
                      : 'none'
                  }}
                >
                  <FileSearch className="h-5 w-5 text-muted-foreground ml-3" />
                  <input
                    ref={inputRef}
                    type="text"
                    value={localSearchQuery}
                    onChange={(e) => setLocalSearchQuery(e.target.value)}
                    onFocus={() => setSearchFocused(true)}
                    placeholder="Search documentation..."
                    className="flex-1 bg-transparent px-3 py-2 text-sm placeholder:text-muted-foreground focus:outline-none"
                    autoComplete="off"
                  />
                  {!localSearchQuery && !searchFocused && (
                    <div className="mr-3 px-1.5 py-0.5 bg-muted/30 rounded text-xs font-mono text-muted-foreground/70 font-bold">
                      /
                    </div>
                  )}
                  {localSearchQuery && (
                    <button
                      type="button"
                      onClick={clearSearch}
                      className="p-2 hover:bg-muted/10 rounded-md transition-colors mr-1"
                    >
                      <X className="h-4 w-4 text-muted-foreground" />
                    </button>
                  )}
                </div>
              </form>
              
              {/* Desktop Search Dropdown */}
              {showDropdown && (
                <DocsSearchDropdown
                  results={results}
                  isLoading={loading}
                  searchQuery={localSearchQuery}
                  onClose={() => {
                    setShowDropdown(false)
                    setLocalSearchQuery('')
                  }}
                />
              )}
            </div>
          </div>
          
          {/* Right spacer - same as NavBar */}
          <div className="flex-1" />
        </div>
      </div>

      <div className="container mx-auto px-4 flex relative">
        {/* Mobile Sidebar Overlay */}
        {sidebarOpen && (
          <div className="md:hidden fixed inset-0 z-50 bg-black/50" onClick={() => setSidebarOpen(false)} />
        )}
        
        {/* Sidebar */}
        <div className={`
          md:w-64 md:py-12 md:relative md:transform-none md:z-auto
          ${sidebarOpen 
            ? 'fixed left-0 top-0 h-full w-64 bg-background z-50 transform translate-x-0 border-r border-border py-4' 
            : 'hidden md:block'
          }
        `}>
          <DocsSidebar />
        </div>
        
        {/* Main content */}
        <div className="flex-1 py-6 md:py-12">
          <div className="max-w-3xl md:ml-16">
            {children}
          </div>
        </div>
      </div>
    </>
  )
}