'use client';

import { useState, useEffect, useRef } from 'react';
import { createPortal } from 'react-dom';
import { TokenSearchResult } from '@/utils/token-search';
import { TrendingUp, TrendingDown } from 'lucide-react';
import Image from 'next/image';
import feelsGuyImage from '@/assets/images/feels_guy.png';
import { useRouter } from 'next/navigation';

interface SearchDropdownProps {
  results: TokenSearchResult[];
  isLoading: boolean;
  searchQuery: string;
  onClose: () => void;
  onNavigate?: () => void;
  searchBarRect?: DOMRect;
}

export function SearchDropdown({ results, isLoading, searchQuery, onClose, onNavigate, searchBarRect }: SearchDropdownProps) {
  const [selectedIndex, setSelectedIndex] = useState(-1);
  const [mounted, setMounted] = useState(false);
  const router = useRouter();
  const itemRefs = useRef<(HTMLButtonElement | null)[]>([]);
  
  useEffect(() => {
    setMounted(true);
  }, []);
  
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
        const token = results[selectedIndex];
        if (token) {
          router.push(`/token/${token.address}`);
          onClose();
        }
      } else if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
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
        behavior: 'instant' // Changed from 'smooth' to 'instant' for snappier feel
      });
    }
    return undefined;
  }, [selectedIndex]);
  
  if (!searchQuery.trim() || !mounted) return null;

  const dropdownStyle = searchBarRect ? {
    position: 'fixed' as const,
    top: searchBarRect.bottom + 8,
    left: searchBarRect.left,
    width: searchBarRect.width,
    zIndex: 99999,
  } : {
    position: 'absolute' as const,
    top: '100%',
    marginTop: '8px',
    width: '100%',
    zIndex: 99999,
  };
  
  const dropdownContent = (
    <>
      {/* Invisible backdrop to ensure dropdown stays on top */}
      <div 
        className="fixed inset-0 z-[99998]" 
        onClick={(e) => {
          // Only close if clicking the backdrop itself, not child elements
          if (e.target === e.currentTarget) {
            onClose();
          }
        }}
        onMouseDown={(e) => {
          // Prevent form submission on backdrop click
          e.preventDefault();
        }}
      />
      <div 
        id="global-search-dropdown"
        className="bg-background border border-border rounded-lg shadow-xl overflow-hidden"
        style={dropdownStyle}
        onMouseDown={(e) => {
          // Prevent form submission when clicking inside dropdown
          e.preventDefault();
          e.stopPropagation();
        }}
      >
      {isLoading ? (
        <div className="p-4 text-center text-sm text-muted-foreground">
          Searching...
        </div>
      ) : results.length === 0 ? (
        <div className="p-4 text-center">
          <p className="text-sm text-muted-foreground">No tokens found</p>
          <button
            type="button"
            onMouseDown={(e) => {
              e.preventDefault();
              e.stopPropagation();
            }}
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              const searchUrl = `/search?q=${encodeURIComponent(searchQuery)}`;
              
              // Call onNavigate first if provided
              if (onNavigate) {
                onNavigate();
              } else {
                onClose();
              }
              
              // Use setTimeout to ensure the navigation happens after the dropdown closes
              setTimeout(() => {
                router.push(searchUrl);
              }, 10);
            }}
            className="text-sm text-primary hover:underline mt-1 block cursor-pointer bg-transparent border-none w-full"
          >
            View all results
          </button>
        </div>
      ) : (
        <>
          <div className="max-h-[400px] overflow-y-auto">
            {results.slice(0, 8).map((token, index) => {
              const priceChangeColor = token.priceChange24h >= 0 ? 'text-primary' : 'text-danger-500';
              const PriceIcon = token.priceChange24h >= 0 ? TrendingUp : TrendingDown;
              const isSelected = selectedIndex === index;
              
              return (
                <button
                  type="button"
                  key={token.address}
                  ref={el => { 
                    itemRefs.current[index] = el; 
                    return undefined; 
                  }}
                  onMouseDown={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                  }}
                  onClick={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    const tokenUrl = `/token/${token.address}`;
                    
                    // Call onNavigate first if provided
                    if (onNavigate) {
                      onNavigate();
                    } else {
                      onClose();
                    }
                    
                    setTimeout(() => {
                      router.push(tokenUrl);
                    }, 10);
                  }}
                  onMouseEnter={() => setSelectedIndex(index)}
                  className={`flex items-center gap-3 p-3 transition-colors duration-75 w-full text-left cursor-pointer bg-transparent border-none ${
                    isSelected ? 'bg-muted/50' : 'hover:bg-muted/50'
                  }`}
                >
                  {/* Token Image */}
                  <div className="relative h-10 w-10 flex-shrink-0">
                    <Image
                      src={token.imageUrl}
                      alt={token.name}
                      fill
                      sizes="40px"
                      className="rounded-md object-cover"
                      onError={(e) => {
                        const target = e.target as HTMLImageElement;
                        target.src = feelsGuyImage.src;
                      }}
                    />
                  </div>
                  
                  {/* Token Info */}
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="font-medium truncate">{token.name}</span>
                      <span className="text-sm text-muted-foreground">{token.symbol}</span>
                    </div>
                    <div className="flex items-center gap-3 text-xs text-muted-foreground">
                      <span>{token.marketCapFormatted}</span>
                      <span className={`flex items-center gap-1 ${priceChangeColor}`}>
                        <PriceIcon className="h-3 w-3" />
                        {token.priceChange24h > 0 ? '+' : ''}{token.priceChange24h.toFixed(2)}%
                      </span>
                    </div>
                  </div>
                </button>
              );
            })}
          </div>
          
          {/* View All / Go to Search page Link */}
          <button
            type="button"
            onMouseDown={(e) => {
              e.preventDefault();
              e.stopPropagation();
            }}
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              const searchUrl = `/search?q=${encodeURIComponent(searchQuery)}`;
              
              // Call onNavigate first if provided
              if (onNavigate) {
                onNavigate();
              } else {
                onClose();
              }
              
              setTimeout(() => {
                router.push(searchUrl);
              }, 10);
            }}
            className="block w-full p-3 text-sm text-center text-primary hover:bg-muted/50 border-t border-border transition-colors duration-75 cursor-pointer bg-transparent"
          >
            {results.length > 8 ? `View all ${results.length} results` : 'Go to search page'}
          </button>
        </>
      )}
      </div>
    </>
  );
  
  return mounted && typeof window !== 'undefined' 
    ? createPortal(dropdownContent, document.body)
    : null;
}