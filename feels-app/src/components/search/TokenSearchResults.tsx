'use client';

import { useState } from 'react';
import { TokenSearchResult } from '@/utils/token-search';
import { TokenSearchCard } from '@/components/search/TokenSearchCard';
import { TokenSearchRow } from '@/components/search/TokenSearchRow';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Skeleton } from '@/components/ui/skeleton';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { AlertCircle, TextSearch, Filter, ChevronDown, ChevronUp, ChevronsUpDown } from 'lucide-react';

interface TokenSearchResultsProps {
  results: TokenSearchResult[];
  sortBy: string;
  setSortBy: (sort: 'relevance' | 'marketCap' | 'volume' | 'priceChange' | 'age') => void;
  isLoading: boolean;
  error: any;
  searchActive?: boolean;
  onSearchClick?: () => void;
  totalResults?: number;
  searchQuery?: string;
  hasActiveFilters?: boolean;
  onClearFilters?: () => void;
  showFilters?: boolean;
  onToggleFilters?: () => void;
}

type SortField = 'relevance' | 'marketCap' | 'volume' | 'price' | 'priceChange' | 'age';
type SortOrder = 'asc' | 'desc';

export function TokenSearchResults({
  results,
  sortBy,
  setSortBy,
  isLoading,
  error,
  searchActive,
  onSearchClick,
  totalResults,
  searchQuery,
  hasActiveFilters,
  onClearFilters,
  showFilters,
  onToggleFilters
}: TokenSearchResultsProps) {
  const [sortOrder, setSortOrder] = useState<SortOrder>('desc');
  
  const handleSort = (field: SortField) => {
    if (sortBy === field) {
      // Toggle sort order if clicking same column
      setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc');
    } else {
      // Default to descending for new column
      setSortBy(field as any);
      setSortOrder('desc');
    }
  };
  
  const getSortIcon = (field: SortField) => {
    const isSelected = sortBy === field;
    
    if (!isSelected) {
      return <ChevronsUpDown className="h-3 w-3 opacity-50" />;
    }
    
    // When selected, show custom chevrons with the same layout as ChevronsUpDown
    return (
      <svg
        width="12"
        height="12"
        viewBox="0 0 24 24"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        className="h-3 w-3"
      >
        <path
          d="M7 15L12 20L17 15"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          opacity={sortOrder === 'desc' ? '1' : '0.3'}
        />
        <path
          d="M7 9L12 4L17 9"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          opacity={sortOrder === 'asc' ? '1' : '0.3'}
        />
      </svg>
    );
  };
  
  if (error) {
    return (
      <Alert variant="destructive">
        <AlertCircle className="h-4 w-4" />
        <AlertDescription>
          Failed to load tokens. Please try again later.
        </AlertDescription>
      </Alert>
    );
  }
  
  if (isLoading) {
    return (
      <div>
        <div className="border rounded-lg overflow-hidden">
          {/* Table Header */}
          <div className="flex items-center gap-4 px-4 py-3 border-b text-xs font-medium text-muted-foreground">
            <div className="flex-1 min-w-[240px]">Token</div>
            <div className="w-24 text-right">Market Cap</div>
            <div className="w-24 text-right">24h Volume</div>
            <div className="w-28 text-right">Price</div>
            <div className="w-36 text-right">Price/Change</div>
            <div className="w-24 text-right">Δ 24h</div>
          </div>
          <div className="divide-y">
            {[...Array(6)].map((_, i) => (
              <div key={i} className="flex items-center gap-4 px-4 py-3">
                <Skeleton className="w-10 h-10 rounded-md" />
                <div className="flex-1 min-w-[200px]">
                  <Skeleton className="h-4 w-32 mb-1" />
                  <Skeleton className="h-3 w-16" />
                </div>
                <div className="w-24">
                  <Skeleton className="h-4 w-full" />
                </div>
                <div className="w-24">
                  <Skeleton className="h-4 w-full" />
                </div>
                <div className="w-28">
                  <Skeleton className="h-4 w-full" />
                </div>
                <div className="w-36">
                  <Skeleton className="h-4 w-full" />
                </div>
                <div className="w-24">
                  <Skeleton className="h-4 w-full" />
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    );
  }
  
  if (results.length === 0) {
    return (
      <div className="text-center py-12">
        <p className="text-lg text-muted-foreground mb-2">No tokens found</p>
        <p className="text-sm text-muted-foreground">
          Try adjusting your search criteria or filters
        </p>
      </div>
    );
  }
  
  // Sort results based on current sort field and order
  const sortedResults = [...results].sort((a, b) => {
    let compareValue = 0;
    
    switch (sortBy) {
      case 'relevance':
        compareValue = (b._score || 0) - (a._score || 0);
        break;
      case 'marketCap':
        compareValue = b.marketCap - a.marketCap;
        break;
      case 'volume':
        compareValue = b.volume24h - a.volume24h;
        break;
      case 'price':
        compareValue = b.price - a.price;
        break;
      case 'priceChange':
        compareValue = b.priceChange24h - a.priceChange24h;
        break;
      case 'age':
        compareValue = b.launchDate.getTime() - a.launchDate.getTime();
        break;
      default:
        return 0;
    }
    
    // Reverse for ascending order
    return sortOrder === 'asc' ? -compareValue : compareValue;
  });
  
  return (
    <div>
      {/* Results Table */}
      <div className="border rounded-lg overflow-hidden relative">
        {/* Action Buttons */}
        <div className="absolute top-3 right-4 flex items-center gap-2 z-10">
          {hasActiveFilters && onClearFilters && (
            <Button
              variant="ghost"
              size="sm"
              onClick={onClearFilters}
              className="text-xs h-7 px-2"
            >
              Clear filters
            </Button>
          )}
          {onToggleFilters && (
            <Button
              variant="outline"
              size="sm"
              onClick={onToggleFilters}
              className="lg:hidden hover:border-primary h-7"
            >
              <Filter className="h-3 w-3" />
            </Button>
          )}
        </div>
        {/* Table Header */}
        <div className="flex items-center gap-4 px-4 py-3 border-b text-xs font-medium text-muted-foreground">
          <div className="flex-1 min-w-[240px] flex items-center justify-between">
            <span>
              Token
              {totalResults !== undefined && !isLoading && (
                <span className="ml-2 font-normal">
                  ({totalResults} {totalResults === 1 ? 'result' : 'results'})
                </span>
              )}
            </span>
            {searchQuery && (
              <button
                onClick={() => handleSort('relevance')}
                className="flex items-center gap-1 hover:text-foreground transition-colors cursor-pointer"
              >
                <span>Relevance</span>
                {getSortIcon('relevance')}
              </button>
            )}
          </div>
          <button
            onClick={() => handleSort('marketCap')}
            className="w-24 text-right flex items-center justify-end gap-1 hover:text-foreground transition-colors cursor-pointer"
          >
            <span>Market Cap</span>
            {getSortIcon('marketCap')}
          </button>
          <button
            onClick={() => handleSort('volume')}
            className="w-24 text-right flex items-center justify-end gap-1 hover:text-foreground transition-colors cursor-pointer"
          >
            <span>24h Volume</span>
            {getSortIcon('volume')}
          </button>
          <button
            onClick={() => handleSort('price')}
            className="w-28 text-right flex items-center justify-end gap-1 hover:text-foreground transition-colors cursor-pointer"
          >
            <span>Price</span>
            {getSortIcon('price')}
          </button>
          <button
            onClick={() => handleSort('priceChange')}
            className="w-36 text-right flex items-center justify-end gap-1 hover:text-foreground transition-colors cursor-pointer"
          >
            <span>Price/Change</span>
            {getSortIcon('priceChange')}
          </button>
          <button
            onClick={() => handleSort('priceChange')}
            className="w-24 text-right flex items-center justify-end gap-1 hover:text-foreground transition-colors cursor-pointer"
          >
            <span>Δ 24h</span>
            {getSortIcon('priceChange')}
          </button>
        </div>
        
        {/* Table Rows */}
        <div className="divide-y">
          {sortedResults.map(token => (
            <TokenSearchRow 
              key={token.address} 
              token={token} 
              showRelevance={!!searchQuery}
            />
          ))}
        </div>
      </div>
      
      {/* Load More / Pagination would go here */}
      {results.length >= 20 && (
        <div className="text-center py-4">
          <p className="text-sm text-muted-foreground">
            Showing {results.length} results
          </p>
        </div>
      )}
    </div>
  );
}