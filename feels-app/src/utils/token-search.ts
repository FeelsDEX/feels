// Token search types and configuration
import feelsGuyImage from '@/assets/images/feels_guy.png';

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

export const searchFacets: FacetConfig = {
  marketCapRange: {
    type: 'range',
    field: 'marketCap',
    buckets: [
      { label: 'Micro (<$100k)', min: 0, max: 100000 },
      { label: 'Small ($100k-$1M)', min: 100000, max: 1000000 },
      { label: 'Medium ($1M-$10M)', min: 1000000, max: 10000000 },
      { label: 'Large (>$10M)', min: 10000000, max: Infinity }
    ]
  },
  
  volumeRange: {
    type: 'range',
    field: 'volume24h',
    buckets: [
      { label: 'Low (<$50k)', min: 0, max: 50000 },
      { label: 'Medium ($50k-$500k)', min: 50000, max: 500000 },
      { label: 'High ($500k-$2M)', min: 500000, max: 2000000 },
      { label: 'Very High (>$2M)', min: 2000000, max: Infinity }
    ]
  },
  
  priceChange: {
    type: 'range',
    field: 'priceChange24h',
    buckets: [
      { label: 'Dumping (<-20%)', min: -Infinity, max: -20 },
      { label: 'Down (-20% to 0%)', min: -20, max: 0 },
      { label: 'Up (0% to +20%)', min: 0, max: 20 },
      { label: 'Mooning (>+20%)', min: 20, max: Infinity }
    ]
  },
  
  age: {
    type: 'dateRange',
    field: 'launchDate',
    buckets: [
      { label: 'Just Launched (<1hr)', days: 0.04 },
      { label: 'Fresh (<1 day)', days: 1 },
      { label: 'New (1-7 days)', days: 7 },
      { label: 'Established (>7 days)', days: Infinity }
    ]
  },
  
  features: {
    type: 'multiSelect',
    options: [
      { value: 'verified', label: 'Verified' },
      { value: 'hasLiquidity', label: 'Has Liquidity' },
      { value: 'graduated', label: 'Graduated' }
    ]
  }
};

export interface SelectedFacets {
  marketCapRange?: string[];
  volumeRange?: string[];
  priceChange?: string[];
  age?: string[];
  features?: string[];
}

// Convert token data to search format
export function convertToSearchResult(token: any): TokenSearchResult {
  const marketCapNum = parseFloat(token.marketCap?.replace(/[$,M]/g, '')) || 0;
  const volumeNum = parseFloat(token.volume24h?.replace(/[$,K]/g, '')) || 0;
  
  // Parse launch date from "X days ago" format
  const launchMatch = token.launched?.match(/(\d+)\s*(hour|day|week|month)/);
  let launchDate = new Date();
  if (launchMatch) {
    const [, num, unit] = launchMatch;
    const amount = parseInt(num);
    switch (unit) {
      case 'hour':
        launchDate.setHours(launchDate.getHours() - amount);
        break;
      case 'day':
        launchDate.setDate(launchDate.getDate() - amount);
        break;
      case 'week':
        launchDate.setDate(launchDate.getDate() - amount * 7);
        break;
      case 'month':
        launchDate.setMonth(launchDate.getMonth() - amount);
        break;
    }
  }
  
  return {
    address: token.address,
    symbol: token.symbol,
    name: token.name,
    imageUrl: token.imageUrl || token.logoURI || feelsGuyImage.src,
    marketCap: marketCapNum * (token.marketCap?.includes('M') ? 1000000 : token.marketCap?.includes('K') ? 1000 : 1),
    marketCapFormatted: token.marketCap || '$0',
    volume24h: volumeNum * (token.volume24h?.includes('M') ? 1000000 : token.volume24h?.includes('K') ? 1000 : 1),
    volume24hFormatted: token.volume24h || '$0',
    priceChange24h: token.priceChange24h || 0,
    price: token.price || 0,
    launchDate,
    launched: token.launched || 'Unknown',
    isVerified: token.isVerified || false,
    hasLiquidity: token.hasLiquidity !== false,
    isGraduated: token.isGraduated || false,
    description: token.description,
    decimals: token.decimals || 9,
  };
}