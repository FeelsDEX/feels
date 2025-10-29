'use client';

import { useState, useEffect, useRef } from 'react';
import { TokenSearchResult } from '@/utils/token-search';
import { TrendingUp, TrendingDown } from 'lucide-react';
import Image from 'next/image';
import feelsGuyImage from '@/assets/images/feels_guy.png';

interface TokenSelectDropdownProps {
  results: TokenSearchResult[];
  isLoading: boolean;
  searchQuery: string;
  onSelect: (token: TokenSearchResult) => void;
  onClose: () => void;
  excludeAddress?: string; // Address to exclude from results (e.g., already selected token)
  preloadedTokens?: TokenSearchResult[]; // Pre-loaded popular tokens for instant display
}

export function TokenSelectDropdown({ 
  results, 
  isLoading, 
  searchQuery, 
  onSelect, 
  onClose,
  excludeAddress,
  preloadedTokens 
}: TokenSelectDropdownProps) {
  const [selectedIndex, setSelectedIndex] = useState(-1);
  const itemRefs = useRef<(HTMLDivElement | null)[]>([]);
  
  // Use preloaded tokens when no search query, otherwise use search results
  const displayTokens = searchQuery.trim() ? results : (preloadedTokens || results);
  
  // Filter out excluded address
  const filteredResults = displayTokens.filter(token => token.address !== excludeAddress);
  
  useEffect(() => {
    setSelectedIndex(-1);
  }, [results]);
  
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedIndex(prev => Math.min(prev + 1, filteredResults.length - 1));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedIndex(prev => Math.max(prev - 1, -1));
      } else if (e.key === 'Enter' && selectedIndex >= 0) {
        e.preventDefault();
        const token = filteredResults[selectedIndex];
        if (token) {
          onSelect(token);
          onClose?.();
        }
      } else if (e.key === 'Escape') {
        onClose?.();
      }
    };
    
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [selectedIndex, filteredResults, onSelect, onClose]);
  
  // Scroll selected item into view
  useEffect(() => {
    if (selectedIndex >= 0 && itemRefs.current[selectedIndex]) {
      itemRefs.current[selectedIndex]?.scrollIntoView({
        block: 'nearest',
        behavior: 'instant'
      });
    }
    return undefined;
  }, [selectedIndex]);
  
  // Always show dropdown, even without search query
  
  return (
    <div 
      id="token-select-dropdown"
      className="absolute top-full mt-2 w-full bg-background border border-border rounded-lg shadow-xl overflow-hidden"
      style={{ zIndex: 50000 }}
    >
      {isLoading && searchQuery ? (
        <div id="token-search-loading" className="p-4 text-center text-sm text-muted-foreground">
          Searching...
        </div>
      ) : (!searchQuery.trim() || filteredResults.length > 0) ? (
        <div id="token-search-results">
          <div className="max-h-[400px] overflow-y-auto">
          {filteredResults.slice(0, 8).map((token, index) => {
            const priceChangeColor = token.priceChange24h >= 0 ? 'text-primary' : 'text-danger-500';
            const PriceIcon = token.priceChange24h >= 0 ? TrendingUp : TrendingDown;
            const isSelected = selectedIndex === index;
            
            return (
              <div
                key={token.address}
                ref={el => { itemRefs.current[index] = el; }}
                id={`token-select-option-${token.symbol.toLowerCase()}`}
                onClick={() => {
                  onSelect(token);
                  onClose();
                }}
                className={`flex items-center gap-3 px-4 py-3 transition-colors cursor-pointer ${
                  isSelected ? 'bg-muted' : 'hover:bg-muted/50'
                }`}
              >
                {/* Token Image */}
                <div className="relative">
                  {token.imageUrl ? (
                    <Image
                      src={token.imageUrl}
                      alt={token.name}
                      width={32}
                      height={32}
                      className="rounded-md"
                      style={{ width: '32px', height: '32px', objectFit: 'contain' }}
                      onError={(e) => {
                        e.currentTarget.src = feelsGuyImage.src;
                      }}
                    />
                  ) : (
                    <div className="w-8 h-8 bg-primary rounded-md flex items-center justify-center text-xs font-bold text-primary-foreground">
                      {token.symbol.substring(0, 2).toUpperCase()}
                    </div>
                  )}
                </div>
                
                {/* Token Info */}
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-medium">{token.symbol}</span>
                    <span className="text-sm text-muted-foreground truncate">{token.name}</span>
                  </div>
                </div>
                
                {/* Price & Change */}
                <div className="text-right">
                  <div className="text-sm font-medium">${token.price.toFixed(4)}</div>
                  <div className={`text-xs ${priceChangeColor} flex items-center justify-end gap-1`}>
                    <PriceIcon className="h-3 w-3" />
                    {Math.abs(token.priceChange24h).toFixed(2)}%
                  </div>
                </div>
              </div>
            );
          })}
          </div>
        </div>
      ) : (
        <div id="token-search-no-results" className="p-4 text-center">
          <p className="text-sm text-muted-foreground">No tokens found</p>
        </div>
      )}
    </div>
  );
}