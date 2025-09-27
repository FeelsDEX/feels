'use client';

import { useState, useEffect, useRef } from 'react';
import { Compass, Repeat2, X, Search, TextSearch } from 'lucide-react';
import { SearchDropdown } from '@/components/search/SearchDropdown';
import { TokenSelectDropdown } from '@/components/search/TokenSelectDropdown';
import { useTokenSearch } from '@/hooks/useTokenSearch';
import { TokenSearchResult } from '@/utils/token-search';
import { useRouter } from 'next/navigation';

interface SearchBarProps {
  placeholder?: string;
  onTokenSelect?: (token: TokenSearchResult) => void;
  excludeAddress?: string;
  mode?: 'navigation' | 'token-select' | 'page-search';
  onClose?: () => void;
  autoFocus?: boolean;
  searchQuery?: string;
  onSearchChange?: (query: string) => void;
  preloadedTokens?: TokenSearchResult[]; // Pre-loaded tokens for instant dropdown
}

export function SearchBar({ 
  placeholder = "Find tokens by name, ticker, or address...",
  onTokenSelect,
  excludeAddress,
  mode = 'navigation',
  onClose,
  autoFocus = false,
  searchQuery: externalSearchQuery,
  onSearchChange,
  preloadedTokens
}: SearchBarProps) {
  const [localSearchQuery, setLocalSearchQuery] = useState(externalSearchQuery || '');
  const [searchFocused, setSearchFocused] = useState(autoFocus || false);
  const [showDropdown, setShowDropdown] = useState(false);
  const [isClearing, setIsClearing] = useState(false);
  const searchRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const router = useRouter();
  
  const { 
    searchQuery,
    setSearchQuery,
    results,
    isLoading 
  } = useTokenSearch();
  
  // Update local query when external query changes (for page-search mode)
  useEffect(() => {
    if (mode === 'page-search' && externalSearchQuery !== undefined) {
      setLocalSearchQuery(externalSearchQuery);
    }
  }, [externalSearchQuery, mode]);
  
  // Reset state when token-select mode is closed
  useEffect(() => {
    if (mode === 'token-select') {
      // Reset everything when the parent component unmounts/closes
      return () => {
        setLocalSearchQuery('');
        setSearchQuery('');
        setSearchFocused(false);
        setShowDropdown(false);
        if (inputRef.current) {
          inputRef.current.blur();
        }
      };
    }
  }, [mode, setSearchQuery]);

  // Filter results if in token-select mode
  const filteredResults = mode === 'token-select' && excludeAddress 
    ? results.filter(token => token.address !== excludeAddress)
    : results;

  // Update search query after debounce
  useEffect(() => {
    if (mode === 'page-search') {
      // For page-search mode, update external search immediately
      onSearchChange?.(localSearchQuery);
      return;
    }
    
    if (localSearchQuery === '' || isClearing) {
      // Clear immediately when search is cleared
      setSearchQuery('');
      setShowDropdown(false);
      return;
    }
    
    const timer = setTimeout(() => {
      setSearchQuery(localSearchQuery);
      // Show dropdown for navigation and token-select modes when not clearing
      if (!isClearing) {
        if (mode === 'navigation') {
          setShowDropdown(localSearchQuery.trim().length > 0);
        } else if (mode === 'token-select') {
          setShowDropdown(searchFocused);
        }
      }
    }, 100);
    
    return () => clearTimeout(timer);
  }, [localSearchQuery, setSearchQuery, mode, onSearchChange, isClearing, searchFocused]);

  // Handle click outside to close dropdown
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      // Skip if we're clearing (X button was clicked)
      if (isClearing) return;
      
      // Skip for token-select mode - let the modal handle it
      if (mode === 'token-select') return;
      
      if (searchRef.current && !searchRef.current.contains(e.target as Node)) {
        setShowDropdown(false);
        setSearchFocused(false);
        setLocalSearchQuery('');
        // Blur the input to remove focus
        if (inputRef.current) {
          inputRef.current.blur();
        }
        if (mode === 'page-search' && onClose) {
          onClose();
        }
      }
    };
    
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [mode, onClose, isClearing]);

  // Auto focus if requested
  useEffect(() => {
    if (autoFocus && inputRef.current) {
      // Use a longer timeout to ensure the modal is fully rendered
      const timer = setTimeout(() => {
        inputRef.current?.focus();
        // Also set the search as focused for styling
        setSearchFocused(true);
      }, 150);
      return () => clearTimeout(timer);
    }
    return undefined;
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
    setIsClearing(true);
    setLocalSearchQuery('');
    setSearchQuery('');
    setShowDropdown(false);
    setSearchFocused(false);
    // For navigation mode, blur the input to ensure dropdown doesn't reappear
    if (mode === 'navigation' && inputRef.current) {
      inputRef.current.blur();
    }
    // Reset clearing flag after a short delay
    setTimeout(() => setIsClearing(false), 100);
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
    // Blur the input
    if (inputRef.current) {
      inputRef.current.blur();
    }
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
            (searchFocused || mode === 'token-select') ? 'border-[#5cca39]' : 'border-border'
          }`}
          style={{
            boxShadow: (searchFocused || mode === 'token-select') 
              ? '0 0 12px 2px rgba(92, 202, 57, 0.15)' 
              : 'none'
          }}
        >
          {mode === 'navigation' ? (
            <Compass className="h-5 w-5 text-muted-foreground ml-3" />
          ) : mode === 'token-select' ? (
            <Repeat2 className="h-5 w-5 text-muted-foreground ml-3" />
          ) : mode === 'page-search' ? (
            <TextSearch className="h-5 w-5 text-muted-foreground ml-3" />
          ) : (
            <Search className="h-5 w-5 text-muted-foreground ml-3" />
          )}
          <input
            ref={inputRef}
            id={inputId}
            type="text"
            value={localSearchQuery}
            onChange={(e) => setLocalSearchQuery(e.target.value)}
            onFocus={() => {
              setSearchFocused(true);
              if (!isClearing) {
                if (mode === 'navigation' && localSearchQuery.trim()) {
                  setShowDropdown(true);
                } else if (mode === 'token-select') {
                  setShowDropdown(true);
                }
              }
            }}
            onBlur={() => {
              // Don't handle blur here - let click outside handle everything
            }}
            onKeyDown={(e) => {
              if (e.key === 'Escape') {
                setShowDropdown(false);
                setSearchFocused(false);
                if ((mode === 'token-select' || mode === 'page-search') && onClose) {
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
          {((mode === 'navigation' && !localSearchQuery && !searchFocused) || 
            (mode === 'page-search' && !localSearchQuery && !searchFocused)) && (
            <div className="mr-3 px-1.5 py-0.5 bg-muted/30 rounded text-xs font-mono text-muted-foreground/70 font-bold">
              /
            </div>
          )}
          {localSearchQuery && (
            <button
              type="button"
              onMouseDown={(e) => {
                e.preventDefault();
                e.stopPropagation();
                setIsClearing(true);
              }}
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                clearSearch();
              }}
              className="relative z-[1103] p-2 hover:bg-muted/10 rounded-md transition-colors mr-1"
            >
              <X className="h-4 w-4 text-muted-foreground" />
            </button>
          )}
        </div>
        
        {/* Dropdown - show for navigation and token-select modes */}
        {showDropdown && ((mode === 'navigation' && localSearchQuery.trim()) || mode === 'token-select') && (
          <div id={`${mode}-dropdown-wrapper`} className="relative z-[1102]">
            {mode === 'token-select' ? (
              <TokenSelectDropdown
                results={filteredResults}
                isLoading={isLoading}
                searchQuery={searchQuery}
                onSelect={handleTokenSelect}
                onClose={() => {
                  // For token-select mode, don't handle closing here
                  if (mode === 'token-select') return;
                  setShowDropdown(false);
                  setSearchFocused(false);
                }}
                preloadedTokens={preloadedTokens}
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