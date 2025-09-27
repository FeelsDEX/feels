// Unified filter component with organized sections and sub-headers for token search
'use client';

import { searchFacets, SelectedFacets } from '@/utils/token-search';
import { Checkbox } from '@/components/ui/checkbox';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ChevronDown, ChevronUp } from 'lucide-react';
import { useState } from 'react';

interface TokenSearchFiltersProps {
  selectedFacets: SelectedFacets;
  toggleFacet: (facetKey: keyof SelectedFacets, value: string) => void;
  clearFilters: () => void;
  facetCounts: Record<string, Record<string, number>>;
}

interface FilterSectionProps {
  title: string;
  children: React.ReactNode;
  defaultOpen?: boolean;
  collapsible?: boolean;
}

// Reusable filter section component with collapsible functionality
function FilterSection({ 
  title, 
  children, 
  defaultOpen = true,
  collapsible = true 
}: FilterSectionProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  return (
    <div className="border-b pb-4 last:border-b-0">
      <button
        onClick={() => collapsible && setIsOpen(!isOpen)}
        className={`w-full flex items-center justify-between mb-3 ${
          collapsible ? 'cursor-pointer hover:text-primary' : 'cursor-default'
        }`}
        type="button"
      >
        <span className="font-medium text-sm uppercase tracking-wide text-muted-foreground">
          {title}
        </span>
        {collapsible && (
          <span className="text-muted-foreground">
            {isOpen ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
          </span>
        )}
      </button>
      {isOpen && (
        <div className="space-y-2 animate-in fade-in-0 slide-in-from-top-1 duration-200">
          {children}
        </div>
      )}
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
    <div className="bg-background rounded-lg border p-4">
      {/* Header */}
      <div className="flex items-center justify-between mb-4 pb-3 border-b">
        <h2 className="text-lg font-semibold">Filters</h2>
        <Button
          variant="ghost"
          size="sm"
          onClick={clearFilters}
          className={`text-xs py-0.5 transition-all duration-200 hover:bg-destructive/10 hover:text-destructive ${
            hasActiveFilters 
              ? 'opacity-100 visible' 
              : 'opacity-0 invisible pointer-events-none'
          }`}
        >
          Clear all ({activeFilterCount})
        </Button>
      </div>
      
      <div className="space-y-4">
        {/* Market Cap Filter */}
        <FilterSection title="Market Cap" defaultOpen={true}>
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
        <FilterSection title="24h Volume" defaultOpen={true}>
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
        <FilterSection title="24h Change" defaultOpen={true}>
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
        <FilterSection title="Age" defaultOpen={true}>
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
        <FilterSection title="Features" defaultOpen={true}>
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