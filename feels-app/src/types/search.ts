// Search and faceting type definitions

export interface TokenSearchResult {
  address: string;
  symbol: string;
  name: string;
  imageUrl: string;
  
  // Facet data
  marketCap: number;
  marketCapFormatted: string;
  volume24h: number;
  volume24hFormatted: string;
  priceChange24h: number;
  price: number;
  launchDate: Date;
  launched: string;
  
  // Features
  isVerified: boolean;
  hasLiquidity: boolean;
  isGraduated: boolean;
  
  // Metadata
  description?: string;
  decimals: number;
  
  // Search relevance
  _score?: number;
}

export interface FacetBucket {
  label: string;
  min: number;
  max: number;
  count?: number;
}

export interface DateFacetBucket {
  label: string;
  days: number;
  count?: number;
}

export interface FacetOption {
  value: string;
  label: string;
  count?: number;
}

export type FacetType = 'range' | 'dateRange' | 'multiSelect';

export interface FacetConfig {
  marketCapRange: {
    type: 'range';
    field: 'marketCap';
    buckets: FacetBucket[];
  };
  volumeRange: {
    type: 'range';
    field: 'volume24h';
    buckets: FacetBucket[];
  };
  priceChange: {
    type: 'range';
    field: 'priceChange24h';
    buckets: FacetBucket[];
  };
  age: {
    type: 'dateRange';
    field: 'launchDate';
    buckets: DateFacetBucket[];
  };
  features: {
    type: 'multiSelect';
    options: FacetOption[];
  };
}

export interface SelectedFacets {
  marketCapRange?: string[];
  volumeRange?: string[];
  priceChange?: string[];
  age?: string[];
  features?: string[];
}

