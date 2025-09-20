'use client';

import { searchFacets, SelectedFacets } from '@/utils/token-search';
import { Checkbox } from '@/components/ui/checkbox';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';

interface TokenSearchFiltersProps {
  selectedFacets: SelectedFacets;
  toggleFacet: (facetKey: keyof SelectedFacets, value: string) => void;
  clearFilters: () => void;
  facetCounts: Record<string, Record<string, number>>;
}

export function TokenSearchFilters({
  selectedFacets,
  toggleFacet,
  clearFilters,
  facetCounts
}: TokenSearchFiltersProps) {
  const hasActiveFilters = Object.values(selectedFacets).some(arr => arr.length > 0);
  
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between h-10">
        <h2 className="text-lg font-semibold">Filters</h2>
        <Button
          variant="ghost"
          size="sm"
          onClick={clearFilters}
          className={`text-xs transition-opacity ${hasActiveFilters ? 'opacity-100' : 'opacity-0 pointer-events-none'}`}
        >
          Clear all
        </Button>
      </div>
      
      {/* Market Cap Filter */}
      <div className="border rounded-lg p-4">
        <span className="font-medium block mb-3">Market Cap</span>
        <div className="space-y-2">
          {searchFacets.marketCapRange.buckets.map(bucket => (
            <label
              key={bucket.label}
              className="flex items-center space-x-2 cursor-pointer hover:text-primary"
            >
              <Checkbox
                checked={selectedFacets.marketCapRange?.includes(bucket.label) || false}
                onCheckedChange={() => toggleFacet('marketCapRange', bucket.label)}
              />
              <span className="text-sm flex-1">{bucket.label}</span>
              {facetCounts.marketCapRange?.[bucket.label] > 0 && (
                <Badge variant="secondary" className="text-xs">
                  {facetCounts.marketCapRange[bucket.label]}
                </Badge>
              )}
            </label>
          ))}
        </div>
      </div>
      
      {/* Volume Filter */}
      <div className="border rounded-lg p-4">
        <span className="font-medium block mb-3">24h Volume</span>
        <div className="space-y-2">
          {searchFacets.volumeRange.buckets.map(bucket => (
            <label
              key={bucket.label}
              className="flex items-center space-x-2 cursor-pointer hover:text-primary"
            >
              <Checkbox
                checked={selectedFacets.volumeRange?.includes(bucket.label) || false}
                onCheckedChange={() => toggleFacet('volumeRange', bucket.label)}
              />
              <span className="text-sm flex-1">{bucket.label}</span>
              {facetCounts.volumeRange?.[bucket.label] > 0 && (
                <Badge variant="secondary" className="text-xs">
                  {facetCounts.volumeRange[bucket.label]}
                </Badge>
              )}
            </label>
          ))}
        </div>
      </div>
      
      {/* Price Change Filter */}
      <div className="border rounded-lg p-4">
        <span className="font-medium block mb-3">24h Change</span>
        <div className="space-y-2">
          {searchFacets.priceChange.buckets.map(bucket => (
            <label
              key={bucket.label}
              className="flex items-center space-x-2 cursor-pointer hover:text-primary"
            >
              <Checkbox
                checked={selectedFacets.priceChange?.includes(bucket.label) || false}
                onCheckedChange={() => toggleFacet('priceChange', bucket.label)}
              />
              <span className="text-sm flex-1">{bucket.label}</span>
              {facetCounts.priceChange?.[bucket.label] > 0 && (
                <Badge variant="secondary" className="text-xs">
                  {facetCounts.priceChange[bucket.label]}
                </Badge>
              )}
            </label>
          ))}
        </div>
      </div>
      
      {/* Age Filter */}
      <div className="border rounded-lg p-4">
        <span className="font-medium block mb-3">Age</span>
        <div className="space-y-2">
          {searchFacets.age.buckets.map(bucket => (
            <label
              key={bucket.label}
              className="flex items-center space-x-2 cursor-pointer hover:text-primary"
            >
              <Checkbox
                checked={selectedFacets.age?.includes(bucket.label) || false}
                onCheckedChange={() => toggleFacet('age', bucket.label)}
              />
              <span className="text-sm flex-1">{bucket.label}</span>
              {facetCounts.age?.[bucket.label] > 0 && (
                <Badge variant="secondary" className="text-xs">
                  {facetCounts.age[bucket.label]}
                </Badge>
              )}
            </label>
          ))}
        </div>
      </div>
      
      {/* Features Filter */}
      <div className="border rounded-lg p-4">
        <span className="font-medium block mb-3">Features</span>
        <div className="space-y-2">
          {searchFacets.features.options.map(option => (
            <label
              key={option.value}
              className="flex items-center space-x-2 cursor-pointer hover:text-primary"
            >
              <Checkbox
                checked={selectedFacets.features?.includes(option.value) || false}
                onCheckedChange={() => toggleFacet('features', option.value)}
              />
              <span className="text-sm flex-1">{option.label}</span>
              {facetCounts.features?.[option.value] > 0 && (
                <Badge variant="secondary" className="text-xs">
                  {facetCounts.features[option.value]}
                </Badge>
              )}
            </label>
          ))}
        </div>
      </div>
    </div>
  );
}