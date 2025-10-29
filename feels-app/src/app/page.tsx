'use client';

import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { IndexerErrorBanner } from '@/components/ui/fallback-banner';
import Link from 'next/link';
import Image from 'next/image';
import { getHomepageTokens } from '@/constants/mock-tokens';
import { useDataSource } from '@/contexts/DataSourceContext';
import { useMarkets } from '@/hooks/useIndexer';
import feelsGuyImage from '@/assets/images/feels_guy.png';
import { IndexedMarket } from '@/services/indexer';
import { useState, useEffect } from 'react';

// Helper functions for price display
function formatMetricValue(value: number): string {
  if (value >= 1) return value.toFixed(4);
  else if (value >= 0.01) return value.toFixed(4);
  else if (value >= 0.001) return value.toFixed(5);
  else return value.toFixed(6);
}

function formatPriceRange(low: number, high: number): string {
  return `${formatMetricValue(low)} - ${formatMetricValue(high)}`;
}

// Removed unused formatFloorGtwapDisplay

export default function HomePage() {
  const { dataSource } = useDataSource();
  const { data: markets, loading, error } = useMarkets({ 
    refreshInterval: 30000, // Refresh every 30 seconds
    enabled: dataSource === 'indexer'
  });
  
  // Create homepageTokens once and memoize it
  const [homepageTokens] = useState(() => getHomepageTokens());
  const [displayTokens, setDisplayTokens] = useState<typeof homepageTokens[0][]>(homepageTokens as any);

  // Transform markets data to match homepage tokens format
  useEffect(() => {
    if (dataSource === 'test') {
      setDisplayTokens([...homepageTokens]);
      return;
    }
    
    // If using indexer but no markets, fallback to test data
    if (dataSource === 'indexer' && (!markets || !Array.isArray(markets) || markets.length === 0)) {
      // Instead of showing empty state, fallback to test data for better UX
      setDisplayTokens([...homepageTokens]);
      return;
    }

    // For now, we'll show the raw market data
    // In a real implementation, you'd map these to token metadata
    const marketTokens = (Array.isArray(markets) ? markets : []).slice(0, 4).map((market: IndexedMarket, index: number) => {
      // Extract price from sqrt_price (simplified for demo)
      const sqrtPrice = parseFloat(market.sqrt_price) / 1e9;
      const price = (sqrtPrice * sqrtPrice) / 1e18;
      
      return {
        id: `market-${market.address}`,
        symbol: `MARKET${index + 1}`, // You'd fetch real token metadata
        name: `Market ${index + 1}`,
        address: market.token_1, // Use token_1 as the address (non-FeelsSOL token)
        imageUrl: feelsGuyImage, // Using feels guy as placeholder
        decimals: 9, // Standard SPL token decimals
        price: price,
        priceChange24h: 0, // Would calculate from historical data
        high24h: 0,
        low24h: 0,
        floorPrice: 0,
        gtwapPrice: 0,
        floorRatio: 0,
        floorChange24h: 0,
        floorGtwapRatio: 0,
        isGraduated: false,
        marketCap: '$0', // Would calculate from circulating supply
        volume24h: '$0', // Would get from market stats
        launched: 'Live',
        description: `Market ${index + 1} token`, // Added description
        isFeelsToken: false, // Added isFeelsToken
        creator: 'Unknown' // Added creator (market.creator not available)
      };
    });

    setDisplayTokens(marketTokens);
  }, [dataSource, markets]); // Removed homepageTokens from dependency array

  // Show loading state when fetching indexer data
  if (dataSource === 'indexer' && loading && displayTokens.length === homepageTokens.length) {
    return (
      <div id="home-page" className="container mx-auto px-4 py-4 h-[calc(100vh-10rem)] flex items-center justify-center">
        <div className="flex flex-col items-center space-y-4">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary"></div>
          <p className="text-muted-foreground">Loading market data...</p>
        </div>
      </div>
    );
  }

  // Show error state if indexer fails
  if (dataSource === 'indexer' && error) {
    return (
      <div id="home-page" className="container mx-auto px-4 pt-4 pb-0 -mb-6">
        <IndexerErrorBanner />
        <div id="token-grid" className="grid grid-cols-2 gap-8" style={{ gridTemplateRows: 'repeat(2, 1fr)' }}>
          {homepageTokens.map((token, index) => (
          <Link 
            key={token.id} 
            href={`/token/${token.address}`} 
            className="block group"
            id={`token-card-link-${index}`}
          >
            <Card 
              id={`token-card-${token.symbol.toLowerCase()}`}
              className="h-full hover:shadow-lg hover:border-primary transition-all cursor-pointer flex overflow-hidden border"
              style={{ height: 'calc((100vh - 14rem - 2rem) / 2)' }}
            >
              <div id={`token-image-container-${token.symbol.toLowerCase()}`} className="h-full aspect-square bg-white shrink-0 flex items-center justify-center rounded-lg p-4">
                <div className="relative w-full h-full">
                  {typeof token.imageUrl === 'string' ? (
                    <img 
                      id={`token-image-${token.symbol.toLowerCase()}`}
                      src={token.imageUrl} 
                      alt={token.name}
                      className="w-full h-full object-contain rounded-xl"
                      loading="lazy"
                    />
                  ) : (
                    <Image 
                      id={`token-image-${token.symbol.toLowerCase()}`}
                      src={token.imageUrl} 
                      alt={token.name}
                      fill
                      className="object-contain rounded-xl"
                      sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw"
                      priority={index === 0}
                    />
                  )}
                </div>
              </div>
              <div className="flex-1 flex flex-col p-6">
                {/* Header with token name and badge */}
                <div id={`token-header-${token.symbol.toLowerCase()}`} className="flex items-start justify-between mb-6">
                  <div id={`token-title-container-${token.symbol.toLowerCase()}`} className="flex-1 min-w-0">
                    <h3 id={`token-name-${token.symbol.toLowerCase()}`} className="text-xl font-bold text-foreground leading-tight flex items-center gap-2">
                      <span id={`token-name-text-${token.symbol.toLowerCase()}`}>{token.name}</span>
                      <span id={`token-symbol-${token.symbol.toLowerCase()}`} className="text-lg font-medium text-muted-foreground">${token.symbol}</span>
                    </h3>
                  </div>
                  <Badge 
                    id={`token-launch-badge-${token.symbol.toLowerCase()}`}
                    variant="outline" 
                    className="text-sm font-medium shrink-0 ml-3"
                  >
                    {token.launched}
                  </Badge>
                </div>

                {/* Consistent Grid Layout for All Metrics */}
                <div id={`token-content-${token.symbol.toLowerCase()}`} className="flex-1">
                  <div id={`token-stats-${token.symbol.toLowerCase()}`} className="grid grid-cols-2 gap-x-4 gap-y-3 h-full">
                    {/* Row 1: Price, 24h Change */}
                    <div id={`token-price-row-${token.symbol.toLowerCase()}`} className="flex flex-col justify-center">
                      <span className="text-xs text-muted-foreground font-medium mb-1">Current Price</span>
                      <span id={`token-price-value-${token.symbol.toLowerCase()}`} className="text-lg font-bold text-foreground">${token.price.toFixed(4)}</span>
                    </div>
                    
                    <div id={`token-change-row-${token.symbol.toLowerCase()}`} className="flex flex-col justify-center">
                      <span className="text-xs text-muted-foreground font-medium mb-1">24h Change</span>
                      <span 
                        id={`token-change-value-${token.symbol.toLowerCase()}`}
                        className={`text-lg font-bold ${token.priceChange24h >= 0 ? 'text-primary' : 'text-danger-500'}`}
                      >
                        {token.priceChange24h >= 0 ? '+' : ''}{token.priceChange24h.toFixed(2)}%
                      </span>
                    </div>
                    
                    {/* Row 2: Market Cap, Floor Price */}
                    <div id={`token-market-cap-row-${token.symbol.toLowerCase()}`} className="flex flex-col justify-center">
                      <span className="text-xs text-muted-foreground font-medium mb-1">Market Cap</span>
                      <span id={`token-market-cap-value-${token.symbol.toLowerCase()}`} className="text-sm font-semibold text-foreground">{token.marketCap}</span>
                    </div>
                    
                    <div id={`token-floor-price-row-${token.symbol.toLowerCase()}`} className="flex flex-col justify-center">
                      <span className="text-xs text-muted-foreground font-medium mb-1">Floor Price</span>
                      <span id={`token-floor-price-value-${token.symbol.toLowerCase()}`} className="text-sm font-semibold text-foreground">${formatMetricValue(token.floorPrice)}</span>
                    </div>
                    
                    {/* Row 3: Volume, Range */}
                    <div id={`token-volume-row-${token.symbol.toLowerCase()}`} className="flex flex-col justify-center">
                      <span className="text-xs text-muted-foreground font-medium mb-1">24h Volume</span>
                      <span id={`token-volume-value-${token.symbol.toLowerCase()}`} className="text-sm font-semibold text-foreground">{token.volume24h}</span>
                    </div>
                    
                    <div id={`token-range-row-${token.symbol.toLowerCase()}`} className="flex flex-col justify-center">
                      <span className="text-xs text-muted-foreground font-medium mb-1">24h Range</span>
                      <span id={`token-range-value-${token.symbol.toLowerCase()}`} className="text-sm font-semibold text-foreground">{formatPriceRange(token.low24h, token.high24h)}</span>
                    </div>
                    
                    {/* Row 4: Floor Change */}
                    <div id={`token-floor-change-row-${token.symbol.toLowerCase()}`} className="flex flex-col justify-center col-span-2">
                      <span className="text-xs text-muted-foreground font-medium mb-1">24hr Floor Δ</span>
                      <span id={`token-floor-change-value-${token.symbol.toLowerCase()}`} className={`text-sm font-semibold ${token.floorChange24h >= 0 ? 'text-primary' : 'text-danger-500'}`}>
                        {token.floorChange24h >= 0 ? '+' : ''}{token.floorChange24h.toFixed(2)}%
                      </span>
                    </div>
                  </div>
                </div>
              </div>
            </Card>
          </Link>
        ))}
      </div>
    </div>
    );
  }

  // Main return statement - use displayTokens which will be either test data or indexer data
  return (
    <div id="home-page" className="container mx-auto px-4 pt-4 pb-0 -mb-6">
      
      <div id="token-grid" className="grid grid-cols-2 gap-8" style={{ gridTemplateRows: 'repeat(2, 1fr)' }}>
        {displayTokens.map((token, index) => (
          <Link 
            key={token.id} 
            href={`/token/${token.address}`} 
            className="block group"
            id={`token-card-link-${index}`}
          >
            <Card 
              id={`token-card-${token.symbol.toLowerCase()}`}
              className="h-full hover:shadow-lg hover:border-primary transition-all cursor-pointer flex overflow-hidden border"
              style={{ height: 'calc((100vh - 14rem - 2rem) / 2)' }}
            >
              <div id={`token-image-container-${token.symbol.toLowerCase()}`} className="h-full aspect-square bg-white shrink-0 flex items-center justify-center rounded-lg p-4">
                <div className="relative w-full h-full">
                  {typeof token.imageUrl === 'string' ? (
                    <img 
                      id={`token-image-${token.symbol.toLowerCase()}`}
                      src={token.imageUrl} 
                      alt={token.name}
                      className="w-full h-full object-contain rounded-xl"
                      loading="lazy"
                    />
                  ) : (
                    <Image 
                      id={`token-image-${token.symbol.toLowerCase()}`}
                      src={token.imageUrl} 
                      alt={token.name}
                      fill
                      className="object-contain rounded-xl"
                      sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw"
                      priority={index === 0}
                    />
                  )}
                </div>
              </div>
              <div className="flex-1 flex flex-col p-6">
                {/* Header with token name and badge */}
                <div id={`token-header-${token.symbol.toLowerCase()}`} className="flex items-start justify-between mb-6">
                  <div id={`token-title-container-${token.symbol.toLowerCase()}`} className="flex-1 min-w-0">
                    <h3 id={`token-name-${token.symbol.toLowerCase()}`} className="text-xl font-bold text-foreground leading-tight flex items-center gap-2">
                      <span id={`token-name-text-${token.symbol.toLowerCase()}`}>{token.name}</span>
                      <span id={`token-symbol-${token.symbol.toLowerCase()}`} className="text-lg font-medium text-muted-foreground">${token.symbol}</span>
                    </h3>
                  </div>
                  <Badge 
                    id={`token-launch-badge-${token.symbol.toLowerCase()}`}
                    variant="outline" 
                    className="text-sm font-medium shrink-0 ml-3"
                  >
                    {token.launched}
                  </Badge>
                </div>

                {/* Consistent Grid Layout for All Metrics */}
                <div id={`token-content-${token.symbol.toLowerCase()}`} className="flex-1">
                  <div id={`token-stats-${token.symbol.toLowerCase()}`} className="grid grid-cols-2 gap-x-4 gap-y-3 h-full">
                    {/* Row 1: Price, 24h Change */}
                    <div id={`token-price-row-${token.symbol.toLowerCase()}`} className="flex flex-col justify-center">
                      <span className="text-xs text-muted-foreground font-medium mb-1">Current Price</span>
                      <span id={`token-price-value-${token.symbol.toLowerCase()}`} className="text-lg font-bold text-foreground">${token.price.toFixed(4)}</span>
                    </div>
                    
                    <div id={`token-change-row-${token.symbol.toLowerCase()}`} className="flex flex-col justify-center">
                      <span className="text-xs text-muted-foreground font-medium mb-1">24h Change</span>
                      <span 
                        id={`token-change-value-${token.symbol.toLowerCase()}`}
                        className={`text-lg font-bold ${token.priceChange24h >= 0 ? 'text-primary' : 'text-danger-500'}`}
                      >
                        {token.priceChange24h >= 0 ? '+' : ''}{token.priceChange24h.toFixed(2)}%
                      </span>
                    </div>
                    
                    {/* Row 2: Market Cap, Floor Price */}
                    <div id={`token-market-cap-row-${token.symbol.toLowerCase()}`} className="flex flex-col justify-center">
                      <span className="text-xs text-muted-foreground font-medium mb-1">Market Cap</span>
                      <span id={`token-market-cap-value-${token.symbol.toLowerCase()}`} className="text-sm font-semibold text-foreground">{token.marketCap}</span>
                    </div>
                    
                    <div id={`token-floor-price-row-${token.symbol.toLowerCase()}`} className="flex flex-col justify-center">
                      <span className="text-xs text-muted-foreground font-medium mb-1">Floor Price</span>
                      <span id={`token-floor-price-value-${token.symbol.toLowerCase()}`} className="text-sm font-semibold text-foreground">${formatMetricValue(token.floorPrice)}</span>
                    </div>
                    
                    {/* Row 3: Volume, Range */}
                    <div id={`token-volume-row-${token.symbol.toLowerCase()}`} className="flex flex-col justify-center">
                      <span className="text-xs text-muted-foreground font-medium mb-1">24h Volume</span>
                      <span id={`token-volume-value-${token.symbol.toLowerCase()}`} className="text-sm font-semibold text-foreground">{token.volume24h}</span>
                    </div>
                    
                    <div id={`token-range-row-${token.symbol.toLowerCase()}`} className="flex flex-col justify-center">
                      <span className="text-xs text-muted-foreground font-medium mb-1">24h Range</span>
                      <span id={`token-range-value-${token.symbol.toLowerCase()}`} className="text-sm font-semibold text-foreground">{formatPriceRange(token.low24h, token.high24h)}</span>
                    </div>
                    
                    {/* Row 4: Floor Change */}
                    <div id={`token-floor-change-row-${token.symbol.toLowerCase()}`} className="flex flex-col justify-center col-span-2">
                      <span className="text-xs text-muted-foreground font-medium mb-1">24hr Floor Δ</span>
                      <span id={`token-floor-change-value-${token.symbol.toLowerCase()}`} className={`text-sm font-semibold ${token.floorChange24h >= 0 ? 'text-primary' : 'text-danger-500'}`}>
                        {token.floorChange24h >= 0 ? '+' : ''}{token.floorChange24h.toFixed(2)}%
                      </span>
                    </div>
                  </div>
                </div>
              </div>
            </Card>
          </Link>
        ))}
      </div>
    </div>
  );
}