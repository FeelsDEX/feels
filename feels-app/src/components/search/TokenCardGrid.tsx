// Token card grid component for displaying tokens in card view on the splash page
'use client';

import { TokenSearchResult } from '@/utils/token-search';
import { CompactTokenCard } from '@/components/search/CompactTokenCard';
import { Skeleton } from '@/components/ui/skeleton';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { AlertCircle, Filter } from 'lucide-react';

interface TokenCardGridProps {
  results: TokenSearchResult[];
  isLoading: boolean;
  error: any;
  hasActiveFilters?: boolean;
  onClearFilters?: () => void;
  onToggleFilters?: () => void;
}

export function TokenCardGrid({
  results,
  isLoading,
  error,
  hasActiveFilters,
  onClearFilters,
  onToggleFilters
}: TokenCardGridProps) {
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
        {/* Header with action buttons */}
        {onToggleFilters && (
          <div className="flex items-center justify-end mb-2 lg:hidden">
            <Button
              variant="outline"
              size="sm"
              onClick={onToggleFilters}
              className="hover:border-primary h-7"
            >
              <Filter className="h-3 w-3" />
            </Button>
          </div>
        )}
        
        {/* Loading Grid */}
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
          {[...Array(12)].map((_, i) => (
            <div key={i} className="border rounded-lg overflow-hidden">
              <Skeleton className="aspect-square w-full" />
              <div className="p-3 space-y-2">
                <Skeleton className="h-4 w-3/4" />
                <Skeleton className="h-3 w-1/2" />
                <div className="grid grid-cols-2 gap-2 pt-2">
                  <Skeleton className="h-3 w-full" />
                  <Skeleton className="h-3 w-full" />
                  <Skeleton className="h-3 w-full" />
                  <Skeleton className="h-3 w-full" />
                </div>
              </div>
            </div>
          ))}
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
        {hasActiveFilters && onClearFilters && (
          <Button
            variant="outline"
            size="sm"
            onClick={onClearFilters}
            className="mt-4"
          >
            Clear filters
          </Button>
        )}
      </div>
    );
  }
  
  return (
    <div>
      {/* Header with action buttons - only show on mobile when filter toggle is available */}
      {onToggleFilters && (
        <div className="flex items-center justify-end mb-2 lg:hidden">
          <Button
            variant="outline"
            size="sm"
            onClick={onToggleFilters}
            className="hover:border-primary h-7"
          >
            <Filter className="h-3 w-3" />
          </Button>
        </div>
      )}
      
      {/* Token Grid */}
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
        {results.map(token => (
          <CompactTokenCard 
            key={token.address} 
            token={token} 
          />
        ))}
      </div>
      
      {/* Load More / Pagination */}
      {results.length >= 20 && (
        <div className="text-center py-4 mt-4">
          <p className="text-sm text-muted-foreground">
            Showing {results.length} results
          </p>
        </div>
      )}
    </div>
  );
}

