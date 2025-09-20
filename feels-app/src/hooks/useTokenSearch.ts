import { useState, useMemo, useCallback } from 'react';
import Fuse from 'fuse.js';
import { useQuery } from '@tanstack/react-query';
import { 
  TokenSearchResult, 
  SelectedFacets, 
  convertToSearchResult,
  searchFacets,
  FacetBucket,
  DateFacetBucket
} from '@/lib/token-search';
import { WOJAK_TOKENS } from '@/lib/testData';
import feelsGuyImage from '@/assets/images/feels_guy.png';
import { useDataSource } from '@/contexts/DataSourceContext';
import { useMarkets } from './useIndexer';

const fuseOptions = {
  keys: [
    { name: 'symbol', weight: 3 },
    { name: 'name', weight: 2 },
    { name: 'description', weight: 1 }
  ],
  threshold: 0.3,
  includeScore: true,
  shouldSort: true,
};

export function useTokenSearch(initialQuery: string = '') {
  const [searchQuery, setSearchQuery] = useState(initialQuery);
  const [selectedFacets, setSelectedFacets] = useState<SelectedFacets>({});
  const [sortBy, setSortBy] = useState<'relevance' | 'marketCap' | 'volume' | 'priceChange' | 'age'>('relevance');
  const { dataSource } = useDataSource();
  
  // Get market data from indexer when in indexer mode
  const { data: indexerMarkets, loading: indexerLoading } = useMarkets({ 
    enabled: dataSource === 'indexer'
  });
  
  // Fetch token data based on data source
  const { data: tokens, isLoading, error } = useQuery({
    queryKey: ['tokens', dataSource, indexerMarkets],
    queryFn: async () => {
      if (dataSource === 'test') {
        // Use test data
        return WOJAK_TOKENS.map(convertToSearchResult);
      } else if (dataSource === 'indexer' && indexerMarkets) {
        // Transform indexer markets to search results
        // This is simplified - in a real app you'd fetch token metadata
        return indexerMarkets.map((market, index) => {
          const sqrtPrice = parseFloat(market.sqrt_price) / 1e9;
          const price = (sqrtPrice * sqrtPrice) / 1e18;
          
          return {
            id: market.address,
            address: market.token_1,
            symbol: `TOKEN${index + 1}`,
            name: `Token ${index + 1}`,
            imageUrl: feelsGuyImage.src,
            price: price,
            priceChange24h: 0,
            marketCap: 0,
            marketCapFormatted: '$0',
            volume24h: 0,
            volumeFormatted: '$0',
            launchDate: new Date(),
            isVerified: true,
            hasLiquidity: parseFloat(market.liquidity) > 0,
            isGraduated: market.phase === 'SteadyState',
            _score: undefined
          };
        });
      }
      
      // Default to empty array if neither condition is met
      return [];
    },
    staleTime: 30 * 1000, // 30 seconds
    gcTime: 5 * 60 * 1000, // 5 minutes (renamed from cacheTime)
    enabled: dataSource === 'test' || (dataSource === 'indexer' && !indexerLoading)
  });
  
  // Apply search and filters
  const searchResults = useMemo(() => {
    if (!tokens) return [];
    
    let results = [...tokens];
    
    // Apply text search if query exists
    if (searchQuery.trim()) {
      const fuse = new Fuse(results, fuseOptions);
      const searchResults = fuse.search(searchQuery);
      results = searchResults.map(result => ({
        ...result.item,
        _score: result.score
      }));
    }
    
    // Apply facet filters
    results = results.filter(token => {
      // Market cap filter
      if (selectedFacets.marketCapRange?.length) {
        const matchesMarketCap = selectedFacets.marketCapRange.some(bucketLabel => {
          const bucket = searchFacets.marketCapRange.buckets.find(b => b.label === bucketLabel);
          if (!bucket) return false;
          return token.marketCap >= bucket.min && token.marketCap < bucket.max;
        });
        if (!matchesMarketCap) return false;
      }
      
      // Volume filter
      if (selectedFacets.volumeRange?.length) {
        const matchesVolume = selectedFacets.volumeRange.some(bucketLabel => {
          const bucket = searchFacets.volumeRange.buckets.find(b => b.label === bucketLabel);
          if (!bucket) return false;
          return token.volume24h >= bucket.min && token.volume24h < bucket.max;
        });
        if (!matchesVolume) return false;
      }
      
      // Price change filter
      if (selectedFacets.priceChange?.length) {
        const matchesPriceChange = selectedFacets.priceChange.some(bucketLabel => {
          const bucket = searchFacets.priceChange.buckets.find(b => b.label === bucketLabel);
          if (!bucket) return false;
          return token.priceChange24h >= bucket.min && token.priceChange24h < bucket.max;
        });
        if (!matchesPriceChange) return false;
      }
      
      // Age filter
      if (selectedFacets.age?.length) {
        const now = new Date();
        const matchesAge = selectedFacets.age.some(bucketLabel => {
          const bucket = searchFacets.age.buckets.find(b => b.label === bucketLabel);
          if (!bucket) return false;
          
          const ageInDays = (now.getTime() - token.launchDate.getTime()) / (1000 * 60 * 60 * 24);
          
          if (bucket.days === Infinity) {
            return ageInDays > 7;
          }
          
          const prevBucket = searchFacets.age.buckets[searchFacets.age.buckets.indexOf(bucket) - 1];
          const minDays = prevBucket ? prevBucket.days : 0;
          
          return ageInDays >= minDays && ageInDays < bucket.days;
        });
        if (!matchesAge) return false;
      }
      
      // Features filter
      if (selectedFacets.features?.length) {
        const matchesFeatures = selectedFacets.features.every(feature => {
          switch (feature) {
            case 'verified':
              return token.isVerified;
            case 'hasLiquidity':
              return token.hasLiquidity;
            case 'graduated':
              return token.isGraduated;
            default:
              return true;
          }
        });
        if (!matchesFeatures) return false;
      }
      
      return true;
    });
    
    // Apply sorting
    if (sortBy !== 'relevance' || !searchQuery) {
      results.sort((a, b) => {
        switch (sortBy) {
          case 'marketCap':
            return b.marketCap - a.marketCap;
          case 'volume':
            return b.volume24h - a.volume24h;
          case 'priceChange':
            return b.priceChange24h - a.priceChange24h;
          case 'age':
            return b.launchDate.getTime() - a.launchDate.getTime();
          default:
            return 0;
        }
      });
    }
    
    return results;
  }, [tokens, searchQuery, selectedFacets, sortBy]);
  
  // Calculate facet counts
  const facetCounts = useMemo(() => {
    if (!tokens) return {};
    
    const counts: Record<string, Record<string, number>> = {
      marketCapRange: {},
      volumeRange: {},
      priceChange: {},
      age: {},
      features: {}
    };
    
    tokens.forEach(token => {
      // Market cap counts
      searchFacets.marketCapRange.buckets.forEach(bucket => {
        if (token.marketCap >= bucket.min && token.marketCap < bucket.max) {
          counts.marketCapRange[bucket.label] = (counts.marketCapRange[bucket.label] || 0) + 1;
        }
      });
      
      // Volume counts
      searchFacets.volumeRange.buckets.forEach(bucket => {
        if (token.volume24h >= bucket.min && token.volume24h < bucket.max) {
          counts.volumeRange[bucket.label] = (counts.volumeRange[bucket.label] || 0) + 1;
        }
      });
      
      // Price change counts
      searchFacets.priceChange.buckets.forEach(bucket => {
        if (token.priceChange24h >= bucket.min && token.priceChange24h < bucket.max) {
          counts.priceChange[bucket.label] = (counts.priceChange[bucket.label] || 0) + 1;
        }
      });
      
      // Age counts
      const now = new Date();
      const ageInDays = (now.getTime() - token.launchDate.getTime()) / (1000 * 60 * 60 * 24);
      
      searchFacets.age.buckets.forEach((bucket, index) => {
        const prevBucket = searchFacets.age.buckets[index - 1];
        const minDays = prevBucket ? prevBucket.days : 0;
        
        if (bucket.days === Infinity ? ageInDays > 7 : (ageInDays >= minDays && ageInDays < bucket.days)) {
          counts.age[bucket.label] = (counts.age[bucket.label] || 0) + 1;
        }
      });
      
      // Feature counts
      if (token.isVerified) counts.features['verified'] = (counts.features['verified'] || 0) + 1;
      if (token.hasLiquidity) counts.features['hasLiquidity'] = (counts.features['hasLiquidity'] || 0) + 1;
      if (token.isGraduated) counts.features['graduated'] = (counts.features['graduated'] || 0) + 1;
    });
    
    return counts;
  }, [tokens]);
  
  // Toggle facet selection
  const toggleFacet = useCallback((facetKey: keyof SelectedFacets, value: string) => {
    setSelectedFacets(prev => {
      const current = prev[facetKey] || [];
      const isSelected = current.includes(value);
      
      return {
        ...prev,
        [facetKey]: isSelected 
          ? current.filter(v => v !== value)
          : [...current, value]
      };
    });
  }, []);
  
  // Clear all filters
  const clearFilters = useCallback(() => {
    setSelectedFacets({});
    setSortBy('relevance');
  }, []);
  
  return {
    // Search state
    searchQuery,
    setSearchQuery,
    
    // Filter state
    selectedFacets,
    toggleFacet,
    clearFilters,
    
    // Sort state
    sortBy,
    setSortBy,
    
    // Results
    results: searchResults,
    totalResults: searchResults.length,
    facetCounts,
    
    // Loading state
    isLoading,
    error,
  };
}