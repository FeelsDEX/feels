'use client';

import { useState, useEffect, Suspense } from 'react';
import { useSearchParams, useRouter, usePathname } from 'next/navigation';
import { useTokenSearch } from '@/hooks/useTokenSearch';
import { TokenSearchResults } from '@/components/search/TokenSearchResults';
import { TokenSearchFilters } from '@/components/search/TokenSearchFilters';
import { SearchBar } from '@/components/search/SearchBar';
import { Button } from '@/components/ui/button';
import { Filter } from 'lucide-react';
import { SelectedFacets } from '@/utils/token-search';

function TokensContent() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const pathname = usePathname();
  
  // Parse initial values from URL
  const initialQuery = searchParams.get('q') || '';
  const initialSort = searchParams.get('sort') || 'relevance';
  
  // Parse initial facets from URL with simplified keys
  const initialFacets: SelectedFacets = {};
  
  // Market cap: mc=small,medium
  searchParams.get('mc')?.split(',').filter(Boolean).forEach(v => {
    initialFacets.marketCapRange = initialFacets.marketCapRange || [];
    // Convert simplified values to full labels
    const mcMap: Record<string, string> = {
      'micro': 'Micro (<$100k)',
      'small': 'Small ($100k-$1M)',
      'medium': 'Medium ($1M-$10M)',
      'large': 'Large (>$10M)'
    };
    initialFacets.marketCapRange.push(mcMap[v] || v);
  });
  
  // Volume: vol=low,high
  searchParams.get('vol')?.split(',').filter(Boolean).forEach(v => {
    initialFacets.volumeRange = initialFacets.volumeRange || [];
    const volMap: Record<string, string> = {
      'low': 'Low (<$50k)',
      'medium': 'Medium ($50k-$500k)',
      'high': 'High ($500k-$2M)',
      'very-high': 'Very High (>$2M)'
    };
    initialFacets.volumeRange.push(volMap[v] || v);
  });
  
  // Price change: pc=up,moon
  searchParams.get('pc')?.split(',').filter(Boolean).forEach(v => {
    initialFacets.priceChange = initialFacets.priceChange || [];
    const pcMap: Record<string, string> = {
      'dump': 'Dumping (<-20%)',
      'down': 'Down (-20% to 0%)',
      'up': 'Up (0% to +20%)',
      'moon': 'Mooning (>+20%)'
    };
    initialFacets.priceChange.push(pcMap[v] || v);
  });
  
  // Age: age=new,fresh
  searchParams.get('age')?.split(',').filter(Boolean).forEach(v => {
    initialFacets.age = initialFacets.age || [];
    const ageMap: Record<string, string> = {
      'launch': 'Just Launched (<1hr)',
      'fresh': 'Fresh (<1 day)',
      'new': 'New (1-7 days)',
      'old': 'Established (>7 days)'
    };
    initialFacets.age.push(ageMap[v] || v);
  });
  
  // Features: f=v,l,g (verified, liquidity, graduated)
  searchParams.get('f')?.split(',').filter(Boolean).forEach(v => {
    initialFacets.features = initialFacets.features || [];
    const featMap: Record<string, string> = {
      'v': 'verified',
      'l': 'hasLiquidity',
      'g': 'graduated'
    };
    initialFacets.features.push(featMap[v] || v);
  });
  
  const [showFilters, setShowFilters] = useState(false);
  
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
  } = useTokenSearch(initialQuery, initialFacets, initialSort as any);
  
  // Update URL when filters, sort, or search query changes
  useEffect(() => {
    const params = new URLSearchParams();
    
    // Add search query
    if (searchQuery) {
      params.set('q', searchQuery);
    }
    
    // Add sort
    if (sortBy && sortBy !== 'relevance') {
      params.set('sort', sortBy);
    }
    
    // Add facets with simplified keys
    if (selectedFacets.marketCapRange?.length) {
      const mcRevMap: Record<string, string> = {
        'Micro (<$100k)': 'micro',
        'Small ($100k-$1M)': 'small',
        'Medium ($1M-$10M)': 'medium',
        'Large (>$10M)': 'large'
      };
      const simplified = selectedFacets.marketCapRange.map(v => mcRevMap[v] || v);
      params.set('mc', simplified.join(','));
    }
    
    if (selectedFacets.volumeRange?.length) {
      const volRevMap: Record<string, string> = {
        'Low (<$50k)': 'low',
        'Medium ($50k-$500k)': 'medium',
        'High ($500k-$2M)': 'high',
        'Very High (>$2M)': 'very-high'
      };
      const simplified = selectedFacets.volumeRange.map(v => volRevMap[v] || v);
      params.set('vol', simplified.join(','));
    }
    
    if (selectedFacets.priceChange?.length) {
      const pcRevMap: Record<string, string> = {
        'Dumping (<-20%)': 'dump',
        'Down (-20% to 0%)': 'down',
        'Up (0% to +20%)': 'up',
        'Mooning (>+20%)': 'moon'
      };
      const simplified = selectedFacets.priceChange.map(v => pcRevMap[v] || v);
      params.set('pc', simplified.join(','));
    }
    
    if (selectedFacets.age?.length) {
      const ageRevMap: Record<string, string> = {
        'Just Launched (<1hr)': 'launch',
        'Fresh (<1 day)': 'fresh',
        'New (1-7 days)': 'new',
        'Established (>7 days)': 'old'
      };
      const simplified = selectedFacets.age.map(v => ageRevMap[v] || v);
      params.set('age', simplified.join(','));
    }
    
    if (selectedFacets.features?.length) {
      const featRevMap: Record<string, string> = {
        'verified': 'v',
        'hasLiquidity': 'l',
        'graduated': 'g'
      };
      const simplified = selectedFacets.features.map(v => featRevMap[v] || v);
      params.set('f', simplified.join(','));
    }
    
    // Update URL without triggering navigation
    const newUrl = params.toString() ? `${pathname}?${params.toString()}` : pathname;
    router.replace(newUrl, { scroll: false });
  }, [searchQuery, sortBy, selectedFacets, router, pathname]);
  
  const hasActiveFilters = Object.values(selectedFacets).some(arr => arr.length > 0);
  
  return (
    <>
      {/* Search Bar - positioned like global search */}
      <div className="container mx-auto px-4 relative z-[1001] pointer-events-none">
        <div className="flex items-center h-16 -mt-16">
          {/* Left spacer - same as NavBar */}
          <div className="flex-1" />
          
          {/* Center - Search (same positioning as NavBar) */}
          <div className="flex-1 max-w-xl mx-8 pointer-events-auto">
            <SearchBar
              mode="page-search"
              placeholder="Search for tokens..."
              searchQuery={searchQuery}
              onSearchChange={setSearchQuery}
              autoFocus={false}
            />
          </div>
          
          {/* Right spacer - same as NavBar */}
          <div className="flex-1" />
        </div>
      </div>
      
      {/* Main Content */}
      <div className="container mx-auto px-4 pb-8 pt-4">
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
              totalResults={totalResults}
              searchQuery={searchQuery}
              hasActiveFilters={hasActiveFilters}
              onClearFilters={clearFilters}
              showFilters={showFilters}
              onToggleFilters={() => setShowFilters(!showFilters)}
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