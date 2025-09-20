'use client';

import { useState, useEffect, useRef } from 'react';
import { Compass, Repeat2, X } from 'lucide-react';
import { SearchDropdown } from '@/components/SearchDropdown';
import { TokenSelectDropdown } from '@/components/TokenSelectDropdown';
import { useTokenSearch } from '@/hooks/useTokenSearch';
import { TokenSearchResult } from '@/lib/token-search';
import { useRouter } from 'next/navigation';

interface SearchBarProps {
  placeholder?: string;
  onTokenSelect?: (token: TokenSearchResult) => void;
  excludeAddress?: string;
  mode?: 'navigation' | 'token-select';
  onClose?: () => void;
  autoFocus?: boolean;
}

export function SearchBar({ 
  placeholder = "Find tokens by name, ticker, or address...",
  onTokenSelect,
  excludeAddress,
  mode = 'navigation',
  onClose,
  autoFocus = false
}: SearchBarProps) {
  const [localSearchQuery, setLocalSearchQuery] = useState('');
  const [searchFocused, setSearchFocused] = useState(false);
  const [showDropdown, setShowDropdown] = useState(false);
  const searchRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const router = useRouter();
  
  const { 
    searchQuery,
    setSearchQuery,
    results,
    isLoading 
  } = useTokenSearch();

  // Filter results if in token-select mode
  const filteredResults = mode === 'token-select' && excludeAddress 
    ? results.filter(token => token.address !== excludeAddress)
    : results;

  // Update search query after debounce
  useEffect(() => {
    if (localSearchQuery === '') {
      // Clear immediately when search is cleared
      setSearchQuery('');
      setShowDropdown(false);
    } else {
      const timer = setTimeout(() => {
        setSearchQuery(localSearchQuery);
        setShowDropdown(localSearchQuery.trim().length > 0);
      }, 100);
      
      return () => clearTimeout(timer);
    }
  }, [localSearchQuery, setSearchQuery]);

  // Handle click outside to close dropdown
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (searchRef.current && !searchRef.current.contains(e.target as Node)) {
        setShowDropdown(false);
        setSearchFocused(false);
        // If in token-select mode, close the modal when clicking outside
        if (mode === 'token-select' && onClose) {
          onClose();
        }
      }
    };
    
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [mode, onClose]);

  // Auto focus if requested
  useEffect(() => {
    if (autoFocus && inputRef.current) {
      // Use a longer timeout to ensure the modal is fully rendered
      const timer = setTimeout(() => {
        inputRef.current?.focus();
        // Also set the search as focused for styling
        setSearchFocused(true);
      }, 100);
      return () => clearTimeout(timer);
    }
  }, [autoFocus]);

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (mode === 'navigation' && localSearchQuery.trim()) {
      router.push(`/search?q=${encodeURIComponent(localSearchQuery)}`);
      setShowDropdown(false);
      setSearchFocused(false);
      setLocalSearchQuery('');
    }
  };

  const clearSearch = () => {
    setLocalSearchQuery('');
    setSearchQuery('');
    setShowDropdown(false);
    setSearchFocused(false);
  };

  const handleTokenSelect = (token: TokenSearchResult) => {
    if (mode === 'token-select' && onTokenSelect) {
      onTokenSelect(token);
      if (onClose) onClose();
    } else if (mode === 'navigation') {
      router.push(`/token/${token.address}`);
    }
    setShowDropdown(false);
    setSearchFocused(false);
    setLocalSearchQuery('');
  };

  const inputId = mode === 'navigation' ? 'nav-search-input' : `${mode}-search-input`;
  const formId = mode === 'navigation' ? 'nav-search-form' : `${mode}-search-form`;
  const containerId = mode === 'navigation' ? 'nav-search-container' : `${mode}-search-container`;
  const wrapperId = mode === 'navigation' ? 'nav-search-wrapper' : `${mode}-search-wrapper`;

  return (
    <div id={wrapperId} ref={searchRef} className="relative">
      <form id={formId} onSubmit={handleSearch} className="relative">
        <div 
          id={containerId} 
          className={`relative flex items-center bg-white border rounded-lg transition-all duration-150 ${
            searchFocused ? 'border-primary shadow-lg' : 'border-border'
          }`}
        >
          {mode === 'navigation' ? (
            <Compass className="h-5 w-5 text-muted-foreground ml-3" />
          ) : (
            <Repeat2 className="h-5 w-5 text-muted-foreground ml-3" />
          )}
          <input
            ref={inputRef}
            id={inputId}
            type="text"
            value={localSearchQuery}
            onChange={(e) => setLocalSearchQuery(e.target.value)}
            onFocus={() => {
              setSearchFocused(true);
              if (localSearchQuery.trim()) setShowDropdown(true);
            }}
            onKeyDown={(e) => {
              if (e.key === 'Escape') {
                setShowDropdown(false);
                setSearchFocused(false);
                if (mode === 'token-select' && onClose) {
                  onClose();
                }
              }
            }}
            placeholder={placeholder}
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
        
        {/* Dropdown */}
        {showDropdown && localSearchQuery.trim() && (
          <div id={`${mode}-dropdown-wrapper`} className="relative z-[1102]">
            {mode === 'token-select' ? (
              <TokenSelectDropdown
                results={filteredResults}
                isLoading={isLoading}
                searchQuery={searchQuery}
                onSelect={handleTokenSelect}
                onClose={() => {
                  setShowDropdown(false);
                  setSearchFocused(false);
                }}
              />
            ) : (
              <SearchDropdown
                results={filteredResults}
                isLoading={isLoading}
                searchQuery={searchQuery}
                onClose={() => {
                  setShowDropdown(false);
                  setSearchFocused(false);
                }}
              />
            )}
          </div>
        )}
      </form>
    </div>
  );
}