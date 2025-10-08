// Unified filter component with organized sections and sub-headers for token search
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

interface FilterSectionProps {
  title: string;
  children: React.ReactNode;
}

// Reusable filter section component
function FilterSection({ 
  title, 
  children
}: FilterSectionProps) {
  return (
    <div className="border-b pb-4 last:border-b-0">
      <div className="mb-3">
        <span className="font-medium text-sm uppercase tracking-wide text-muted-foreground">
          {title}
        </span>
      </div>
      <div className="space-y-2">
        {children}
      </div>
    </div>
  );
}

interface FilterItemProps {
  label: string;
  value: string;
  checked: boolean;
  onChange: () => void;
  count?: number;
}

// Reusable filter item component
function FilterItem({ label, value, checked, onChange, count }: FilterItemProps) {
  return (
    <label className="flex items-center space-x-2 cursor-pointer hover:text-primary group">
      <Checkbox
        checked={checked}
        onCheckedChange={onChange}
        className="data-[state=checked]:bg-primary data-[state=checked]:border-primary"
      />
      <span className="text-sm flex-1 group-hover:text-primary transition-colors">
        {label}
      </span>
      {count !== undefined && count > 0 && (
        <Badge variant="secondary" className="text-xs px-1.5 py-0 h-5">
          {count}
        </Badge>
      )}
    </label>
  );
}

export function TokenSearchFilters({
  selectedFacets,
  toggleFacet,
  clearFilters,
  facetCounts
}: TokenSearchFiltersProps) {
  const hasActiveFilters = Object.values(selectedFacets).some(arr => arr.length > 0);
  
  // Count total active filters
  const activeFilterCount = Object.values(selectedFacets).reduce((total, arr) => total + arr.length, 0);
  
  return (
    <div className="border rounded-lg overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b text-xs font-medium text-muted-foreground">
        <span>Filters</span>
        <button
          onClick={clearFilters}
          className={`text-xs hover:text-foreground transition-colors ${
            hasActiveFilters 
              ? 'opacity-100 visible cursor-pointer' 
              : 'opacity-0 invisible pointer-events-none'
          }`}
        >
          Clear all ({activeFilterCount})
        </button>
      </div>
      
      <div className="p-4 space-y-4">
        {/* Market Cap Filter */}
        <FilterSection title="Market Cap">
          {searchFacets.marketCapRange.buckets.map(bucket => (
            <FilterItem
              key={bucket.label}
              label={bucket.label}
              value={bucket.label}
              checked={selectedFacets.marketCapRange?.includes(bucket.label) || false}
              onChange={() => toggleFacet('marketCapRange', bucket.label)}
              count={facetCounts['marketCapRange']?.[bucket.label]}
            />
          ))}
        </FilterSection>
        
        {/* Volume Filter */}
        <FilterSection title="24h Volume" >
          {searchFacets.volumeRange.buckets.map(bucket => (
            <FilterItem
              key={bucket.label}
              label={bucket.label}
              value={bucket.label}
              checked={selectedFacets.volumeRange?.includes(bucket.label) || false}
              onChange={() => toggleFacet('volumeRange', bucket.label)}
              count={facetCounts['volumeRange']?.[bucket.label]}
            />
          ))}
        </FilterSection>
        
        {/* Price Change Filter */}
        <FilterSection title="24h Change" >
          {searchFacets.priceChange.buckets.map(bucket => (
            <FilterItem
              key={bucket.label}
              label={bucket.label}
              value={bucket.label}
              checked={selectedFacets.priceChange?.includes(bucket.label) || false}
              onChange={() => toggleFacet('priceChange', bucket.label)}
              count={facetCounts['priceChange']?.[bucket.label]}
            />
          ))}
        </FilterSection>
        
        {/* Age Filter */}
        <FilterSection title="Age" >
          {searchFacets.age.buckets.map(bucket => (
            <FilterItem
              key={bucket.label}
              label={bucket.label}
              value={bucket.label}
              checked={selectedFacets.age?.includes(bucket.label) || false}
              onChange={() => toggleFacet('age', bucket.label)}
              count={facetCounts['age']?.[bucket.label]}
            />
          ))}
        </FilterSection>
        
        {/* Features Filter */}
        <FilterSection title="Features" >
          {searchFacets.features.options.map(option => (
            <FilterItem
              key={option.value}
              label={option.label}
              value={option.value}
              checked={selectedFacets.features?.includes(option.value) || false}
              onChange={() => toggleFacet('features', option.value)}
              count={facetCounts['features']?.[option.value]}
            />
          ))}
        </FilterSection>
      </div>
    </div>
  );
}