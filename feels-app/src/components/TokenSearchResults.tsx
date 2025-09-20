'use client';

import { TokenSearchResult } from '@/lib/token-search';
import { TokenSearchCard } from '@/components/TokenSearchCard';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Skeleton } from '@/components/ui/skeleton';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { AlertCircle } from 'lucide-react';

interface TokenSearchResultsProps {
  results: TokenSearchResult[];
  sortBy: string;
  setSortBy: (sort: 'relevance' | 'marketCap' | 'volume' | 'priceChange' | 'age') => void;
  isLoading: boolean;
  error: any;
}

export function TokenSearchResults({
  results,
  sortBy,
  setSortBy,
  isLoading,
  error
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
          <Skeleton className="h-8 w-32" />
          <Skeleton className="h-10 w-40" />
        </div>
        <div className="grid gap-4">
          {[...Array(6)].map((_, i) => (
            <Skeleton key={i} className="h-32" />
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
      </div>
    );
  }
  
  return (
    <div className="space-y-6">
      {/* Sort Controls */}
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold">Results</h3>
        <Select value={sortBy} onValueChange={(value: any) => setSortBy(value)}>
          <SelectTrigger className="w-48">
            <SelectValue placeholder="Sort by" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="relevance">Relevance</SelectItem>
            <SelectItem value="marketCap">Market Cap</SelectItem>
            <SelectItem value="volume">24h Volume</SelectItem>
            <SelectItem value="priceChange">24h Change</SelectItem>
            <SelectItem value="age">Recently Launched</SelectItem>
          </SelectContent>
        </Select>
      </div>
      
      {/* Results Grid */}
      <div className="grid gap-4">
        {results.map(token => (
          <TokenSearchCard key={token.address} token={token} />
        ))}
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