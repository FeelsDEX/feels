'use client';

import { useState, useEffect, Suspense } from 'react';
import { useSearchParams } from 'next/navigation';
import { useTokenSearch } from '@/hooks/useTokenSearch';
import { TokenSearchResults } from '@/components/search/TokenSearchResults';
import { TokenSearchFilters } from '@/components/search/TokenSearchFilters';
import { SearchPageHeader } from '@/components/search/SearchPageHeader';
import { Button } from '@/components/ui/button';
import { Filter, Search } from 'lucide-react';

function TokensContent() {
  const searchParams = useSearchParams();
  const initialQuery = searchParams.get('q') || '';
  const [showFilters, setShowFilters] = useState(true);
  // Only activate search page component if not coming from global search
  const [searchActive, setSearchActive] = useState(false);
  
  const {
    searchQuery,
    setSearchQuery,
    selectedFacets,
    toggleFacet,
    clearFilters,
    sortBy,
    setSortBy,
    results,
    totalResults,
    facetCounts,
    isLoading,
    error
  } = useTokenSearch(initialQuery);
  
  // Update search query when URL changes
  useEffect(() => {
    setSearchQuery(initialQuery);
  }, [initialQuery, setSearchQuery]);
  
  const hasActiveFilters = Object.values(selectedFacets).some(arr => arr.length > 0);
  
  return (
    <>
      {/* Search Header - positioned like NavBar */}
      {searchActive && (
        <SearchPageHeader 
          searchQuery={searchQuery}
          onSearchChange={setSearchQuery}
          onActiveChange={setSearchActive}
        />
      )}
      
      {/* Main Content - with padding to account for fixed header */}
      <div className={`container mx-auto px-4 pb-8 ${searchActive ? 'pt-24' : 'pt-4'}`}>
        {/* Page Header */}
        <div className="mb-8">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h1 className="text-3xl font-bold mb-2">Search Feels</h1>
              <p className="text-muted-foreground">
                Find and filter tokens launched on Feels Protocol
              </p>
            </div>
            
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowFilters(!showFilters)}
              className="lg:hidden"
            >
              <Filter className="h-4 w-4 mr-2" />
              Filters
            </Button>
          </div>
        </div>
        
        {/* Search Summary */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <div className="text-sm text-muted-foreground">
              {isLoading ? (
                <span>Searching...</span>
              ) : (
                <span>
                  {totalResults} {totalResults === 1 ? 'token' : 'tokens'} found
                  {searchQuery && ` for "${searchQuery}"`}
                </span>
              )}
            </div>
            
            {!searchActive && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => setSearchActive(true)}
                className="flex items-center gap-2"
              >
                <Search className="h-3 w-3" />
                Search
              </Button>
            )}
          </div>
          
          {hasActiveFilters && (
            <Button
              variant="ghost"
              size="sm"
              onClick={clearFilters}
              className="text-xs"
            >
              Clear all filters
            </Button>
          )}
        </div>
        
        <div className="flex flex-col lg:flex-row gap-8">
          {/* Filters Sidebar */}
          <aside className={`lg:w-64 ${showFilters ? 'block' : 'hidden lg:block'}`}>
            <TokenSearchFilters
              selectedFacets={selectedFacets}
              toggleFacet={toggleFacet}
              clearFilters={clearFilters}
              facetCounts={facetCounts}
            />
          </aside>
          
          {/* Results Section */}
          <main className="flex-1">
            <TokenSearchResults
              results={results}
              sortBy={sortBy}
              setSortBy={setSortBy}
              isLoading={isLoading}
              error={error}
            />
          </main>
        </div>
      </div>
    </>
  );
}

export default function TokensPage() {
  return (
    <Suspense fallback={<div className="container mx-auto px-4 py-8">Loading...</div>}>
      <TokensContent />
    </Suspense>
  );
}