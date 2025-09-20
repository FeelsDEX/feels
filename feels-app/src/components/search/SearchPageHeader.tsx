'use client';

import { useEffect, useState, useRef } from 'react';
import { useSearchContext } from '@/contexts/SearchContext';
import { useTokenSearch } from '@/hooks/useTokenSearch';
import { TextSearch, X } from 'lucide-react';

interface SearchPageHeaderProps {
  searchQuery: string;
  onSearchChange: (query: string) => void;
  onActiveChange?: (active: boolean) => void;
}

export function SearchPageHeader({ searchQuery, onSearchChange, onActiveChange }: SearchPageHeaderProps) {
  const { setIsTokenSearchModalOpen } = useSearchContext();
  const [localSearchQuery, setLocalSearchQuery] = useState(searchQuery);
  const [searchFocused, setSearchFocused] = useState(false);
  const [isActive, setIsActive] = useState(true);
  const inputRef = useRef<HTMLInputElement>(null);
  const searchRef = useRef<HTMLDivElement>(null);

  // Mark search as active when component mounts or becomes active
  useEffect(() => {
    if (isActive) {
      setIsTokenSearchModalOpen(true);
      // Auto focus on mount
      setTimeout(() => {
        inputRef.current?.focus();
        setSearchFocused(true);
      }, 100);
    } else {
      setIsTokenSearchModalOpen(false);
    }
    return () => {
      setIsTokenSearchModalOpen(false);
    };
  }, [setIsTokenSearchModalOpen, isActive]);

  // Update local query when prop changes
  useEffect(() => {
    setLocalSearchQuery(searchQuery);
  }, [searchQuery]);

  // Debounce search updates
  useEffect(() => {
    const timer = setTimeout(() => {
      onSearchChange(localSearchQuery);
    }, 300);
    return () => clearTimeout(timer);
  }, [localSearchQuery, onSearchChange]);

  // Handle click outside to deactivate search
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (searchRef.current && !searchRef.current.contains(e.target as Node)) {
        setSearchFocused(false);
        setIsActive(false);
        onActiveChange?.(false);
      }
    };
    
    if (isActive) {
      document.addEventListener('mousedown', handleClickOutside);
      return () => document.removeEventListener('mousedown', handleClickOutside);
    }
  }, [isActive, onActiveChange]);

  // Notify parent when active state changes
  useEffect(() => {
    onActiveChange?.(isActive);
  }, [isActive, onActiveChange]);

  const clearSearch = () => {
    setLocalSearchQuery('');
    onSearchChange('');
    setSearchFocused(false);
  };

  if (!isActive) return null;

  return (
    <div id="search-page-header" className="fixed top-0 left-0 right-0 z-[1100] pt-2">
      <div id="search-page-header-inner" className="container mx-auto px-4">
        <div id="search-page-header-flex" className="flex items-center h-16">
          {/* Spacer for logo area */}
          <div className="flex-1" />
          
          {/* Center - Search (same positioning as NavBar) */}
          <div id="search-page-search-wrapper" className="flex-1 max-w-xl mx-8 relative z-[1101]" ref={searchRef}>
            <form onSubmit={(e) => e.preventDefault()} className="relative">
              <div 
                className={`flex items-center bg-white border rounded-lg transition-all duration-150 ${
                  searchFocused ? 'border-primary shadow-lg' : 'border-border'
                }`}
              >
                <TextSearch className="h-5 w-5 text-muted-foreground ml-3" />
                <input
                  ref={inputRef}
                  type="text"
                  value={localSearchQuery}
                  onChange={(e) => setLocalSearchQuery(e.target.value)}
                  onFocus={() => setSearchFocused(true)}
                  onBlur={() => setSearchFocused(false)}
                  placeholder="Search for tokens..."
                  className="flex-1 bg-transparent px-3 py-2 text-sm placeholder:text-muted-foreground focus:outline-none"
                  autoComplete="off"
                  autoCorrect="off"
                  autoCapitalize="off"
                  spellCheck="false"
                />
                {localSearchQuery && (
                  <button
                    type="button"
                    onClick={clearSearch}
                    className="relative z-10 p-2 hover:bg-muted/10 rounded-md transition-colors"
                  >
                    <X className="h-4 w-4 text-muted-foreground" />
                  </button>
                )}
              </div>
            </form>
          </div>
          
          {/* Spacer for right side */}
          <div className="flex-1" />
        </div>
      </div>
    </div>
  );
}