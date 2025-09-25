'use client';

import { useState, useEffect, useRef } from 'react';
import { TokenSearchResult } from '@/utils/token-search';
import { TrendingUp, TrendingDown } from 'lucide-react';
import Link from 'next/link';
import Image from 'next/image';
import feelsGuyImage from '@/assets/images/feels_guy.png';
import { useRouter } from 'next/navigation';

interface SearchDropdownProps {
  results: TokenSearchResult[];
  isLoading: boolean;
  searchQuery: string;
  onClose: () => void;
}

export function SearchDropdown({ results, isLoading, searchQuery, onClose }: SearchDropdownProps) {
  const [selectedIndex, setSelectedIndex] = useState(-1);
  const router = useRouter();
  const itemRefs = useRef<(HTMLAnchorElement | null)[]>([]);
  
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
  
  if (!searchQuery.trim()) return null;
  
  return (
    <>
      {/* Invisible backdrop to ensure dropdown stays on top */}
      <div 
        className="fixed inset-0 z-[1098]" 
        onClick={onClose}
      />
      <div 
        className="absolute top-full mt-2 w-full bg-background border border-border rounded-lg shadow-xl overflow-hidden animate-in fade-in-0 slide-in-from-top-1 duration-100 z-[1099]"
      >
      {isLoading ? (
        <div className="p-4 text-center text-sm text-muted-foreground">
          Searching...
        </div>
      ) : results.length === 0 ? (
        <div className="p-4 text-center">
          <p className="text-sm text-muted-foreground">No tokens found</p>
          <Link 
            href={`/search?q=${encodeURIComponent(searchQuery)}`}
            onClick={onClose}
            className="text-xs text-primary hover:underline mt-1 block"
          >
            View all results
          </Link>
        </div>
      ) : (
        <>
          <div className="max-h-[400px] overflow-y-auto">
            {results.slice(0, 8).map((token, index) => {
              const priceChangeColor = token.priceChange24h >= 0 ? 'text-primary' : 'text-red-500';
              const PriceIcon = token.priceChange24h >= 0 ? TrendingUp : TrendingDown;
              const isSelected = selectedIndex === index;
              
              return (
                <Link
                  key={token.address}
                  ref={el => { 
                    itemRefs.current[index] = el; 
                    return undefined; 
                  }}
                  href={`/token/${token.address}`}
                  onClick={onClose}
                  onMouseEnter={() => setSelectedIndex(index)}
                  className={`flex items-center gap-3 p-3 transition-colors duration-75 ${
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
                </Link>
              );
            })}
          </div>
          
          {/* View All / Go to Search Link */}
          <Link
            href={`/search?q=${encodeURIComponent(searchQuery)}`}
            onClick={onClose}
            className="block p-3 text-sm text-center text-primary hover:bg-muted/50 border-t border-border transition-colors duration-75"
          >
            {results.length > 8 ? `View all ${results.length} results` : 'Go to search'}
          </Link>
        </>
      )}
      </div>
    </>
  );
}