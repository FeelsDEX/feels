'use client';

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
import { AlertCircle, TextSearch, Filter, ChevronDown } from 'lucide-react';

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
      <div className="space-y-6">
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-2">
            <Skeleton className="h-8 w-20" />
            <Skeleton className="h-5 w-32" />
          </div>
          <div className="flex items-center gap-2">
            <Skeleton className="h-9 w-20" />
            <Skeleton className="h-9 w-32" />
          </div>
        </div>
        <div className="border rounded-lg overflow-hidden">
          {/* Table Header */}
          <div className="flex items-center gap-4 px-4 py-3 bg-muted/30 border-b text-xs font-medium text-muted-foreground">
            <div className="w-10" />
            <div className="flex-1 min-w-[200px]">Token</div>
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
  
  return (
    <div className="space-y-6">
      {/* Sort Controls */}
      <div className="flex items-center justify-between">
        <div className="flex items-baseline gap-2">
          <h3 className="text-lg font-semibold">Results</h3>
          {totalResults !== undefined && (
            <span className="text-sm text-muted-foreground">
              {isLoading ? (
                <span>Searching...</span>
              ) : (
                <span>
                  ({totalResults} {totalResults === 1 ? 'token' : 'tokens'}
                  {searchQuery && ` for "${searchQuery}"`})
                </span>
              )}
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          {onToggleFilters && (
            <Button
              variant="outline"
              size="sm"
              onClick={onToggleFilters}
              className="lg:hidden hover:border-primary"
            >
              <Filter className="h-4 w-4 mr-2" />
              Filters
            </Button>
          )}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button
                className="flex items-center gap-2 px-3 py-1.5 text-sm bg-background border rounded-md hover:border-primary hover:bg-accent hover:text-accent-foreground focus:outline-none focus-visible:ring-0 w-40 min-h-[2.25rem] transition-all"
              >
                <span className="flex-1 text-left">
                  {sortBy === 'relevance' && 'Relevance'}
                  {sortBy === 'marketCap' && 'Market Cap'}
                  {sortBy === 'volume' && '24h Volume'}
                  {sortBy === 'priceChange' && '24h Change'}
                  {sortBy === 'age' && 'Recently Launched'}
                </span>
                <ChevronDown className="h-4 w-4 opacity-50" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-40">
              <DropdownMenuItem 
                onClick={() => setSortBy('relevance')}
                className={sortBy === 'relevance' ? 'font-semibold' : ''}
              >
                Relevance
              </DropdownMenuItem>
              <DropdownMenuItem 
                onClick={() => setSortBy('marketCap')}
                className={sortBy === 'marketCap' ? 'font-semibold' : ''}
              >
                Market Cap
              </DropdownMenuItem>
              <DropdownMenuItem 
                onClick={() => setSortBy('volume')}
                className={sortBy === 'volume' ? 'font-semibold' : ''}
              >
                24h Volume
              </DropdownMenuItem>
              <DropdownMenuItem 
                onClick={() => setSortBy('priceChange')}
                className={sortBy === 'priceChange' ? 'font-semibold' : ''}
              >
                24h Change
              </DropdownMenuItem>
              <DropdownMenuItem 
                onClick={() => setSortBy('age')}
                className={sortBy === 'age' ? 'font-semibold' : ''}
              >
                Recently Launched
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>
      
      {/* Clear Filters */}
      {hasActiveFilters && onClearFilters && (
        <div className="flex justify-end">
          <Button
            variant="ghost"
            size="sm"
            onClick={onClearFilters}
            className="text-xs"
          >
            Clear all filters
          </Button>
        </div>
      )}
      
      {/* Results Table */}
      <div className="border rounded-lg overflow-hidden">
        {/* Table Header */}
        <div className="flex items-center gap-4 px-4 py-3 bg-muted/30 border-b text-xs font-medium text-muted-foreground">
          <div className="w-10" /> {/* Image column */}
          <div className="flex-1 min-w-[200px]">Token</div>
          <div className="w-24 text-right">Market Cap</div>
          <div className="w-24 text-right">24h Volume</div>
          <div className="w-28 text-right">Price</div>
          <div className="w-36 text-right">Price/Change</div>
          <div className="w-24 text-right">Δ 24h</div>
        </div>
        
        {/* Table Rows */}
        <div className="divide-y">
          {results.map(token => (
            <TokenSearchRow key={token.address} token={token} />
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