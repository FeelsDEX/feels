'use client';

import { useState, useEffect, useRef } from 'react';
import Link from 'next/link';
import { useRouter } from 'next/navigation';

interface DocsSearchResult {
  slug: string;
  title: string;
  description: string;
  category: string;
  permalink: string;
  excerpt?: string;
}

interface DocsSearchDropdownProps {
  results: DocsSearchResult[];
  isLoading: boolean;
  searchQuery: string;
  onClose: () => void;
}

export function DocsSearchDropdown({
  results,
  isLoading,
  searchQuery,
  onClose
}: DocsSearchDropdownProps) {
  const [selectedIndex, setSelectedIndex] = useState(-1);
  const itemRefs = useRef<(HTMLAnchorElement | null)[]>([]);
  const router = useRouter();
  
  useEffect(() => {
    setSelectedIndex(-1);
  }, [results]);
  
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedIndex(prev => Math.min(prev + 1, results.length - 1));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedIndex(prev => Math.max(prev - 1, -1));
      } else if (e.key === 'Enter' && selectedIndex >= 0) {
        e.preventDefault();
        const result = results[selectedIndex];
        if (result) {
          router.push(result.permalink);
          onClose();
        }
      }
    };
    
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [selectedIndex, results, router, onClose]);
  
  // Scroll selected item into view
  useEffect(() => {
    if (selectedIndex >= 0 && itemRefs.current[selectedIndex]) {
      itemRefs.current[selectedIndex]?.scrollIntoView({
        block: 'nearest',
        behavior: 'instant'
      });
    }
  }, [selectedIndex]);

  if (!searchQuery) return null;

  return (
    <div className="absolute top-full mt-2 w-full bg-background border border-border rounded-lg shadow-xl overflow-hidden z-[1099]">
      {searchQuery.length === 1 ? (
        <div className="p-4 text-center text-sm text-muted-foreground">
          Please type at least 2 characters to search
        </div>
      ) : isLoading ? (
        <div className="p-4 text-center text-sm text-muted-foreground">
          Searching documentation...
        </div>
      ) : results.length === 0 ? (
        <div className="p-4 text-center">
          <p className="text-sm text-muted-foreground">No results found for &ldquo;{searchQuery}&rdquo;</p>
        </div>
      ) : (
        <div className="max-h-[400px] overflow-y-auto">
          {results.slice(0, 8).map((result, index) => {
            const isSelected = selectedIndex === index;
            
            return (
              <Link
                key={result.slug}
                ref={el => { itemRefs.current[index] = el; }}
                href={result.permalink}
                className={`px-4 py-3 transition-colors cursor-pointer block ${
                  isSelected ? 'bg-muted' : 'hover:bg-muted/50'
                }`}
                onClick={onClose}
              >
                {/* Content */}
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-medium">{result.title}</span>
                    <span className="text-xs text-muted-foreground">{result.category}</span>
                  </div>
                  <div className="text-sm text-muted-foreground line-clamp-1">
                    {result.description}
                  </div>
                  {result.excerpt && (
                    <div className="text-xs text-muted-foreground mt-1 line-clamp-2">
                      {result.excerpt.split('**').map((part, i) => 
                        i % 2 === 1 ? (
                          <span key={i} className="font-semibold text-primary">
                            {part}
                          </span>
                        ) : (
                          <span key={i}>{part}</span>
                        )
                      )}
                    </div>
                  )}
                </div>
              </Link>
            );
          })}
        </div>
      )}
    </div>
  );
}