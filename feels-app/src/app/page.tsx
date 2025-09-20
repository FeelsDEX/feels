'use client';

import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import Link from 'next/link';
import { HOMEPAGE_TOKENS } from '@/lib/homepageTokens';
import { useDataSource } from '@/contexts/DataSourceContext';
import { useMarkets } from '@/hooks/useIndexer';
import feelsGuyImage from '@/assets/images/feels_guy.png';
import { IndexedMarket } from '@/lib/indexer-client';
import { useState, useEffect } from 'react';

export default function HomePage() {
  const { dataSource } = useDataSource();
  const { data: markets, loading, error } = useMarkets({ 
    refreshInterval: 30000, // Refresh every 30 seconds
    enabled: dataSource === 'indexer'
  });
  const [displayTokens, setDisplayTokens] = useState<typeof HOMEPAGE_TOKENS[0][]>(HOMEPAGE_TOKENS as any);

  // Transform markets data to match HOMEPAGE_TOKENS format
  useEffect(() => {
    if (dataSource === 'test') {
      setDisplayTokens([...HOMEPAGE_TOKENS]);
      return;
    }
    
    // If using indexer but no markets, show empty state
    if (dataSource === 'indexer' && (!markets || markets.length === 0)) {
      setDisplayTokens([]);
      return;
    }

    // For now, we'll show the raw market data
    // In a real implementation, you'd map these to token metadata
    const marketTokens = markets!.slice(0, 8).map((market: IndexedMarket, index: number) => {
      // Extract price from sqrt_price (simplified for demo)
      const sqrtPrice = parseFloat(market.sqrt_price) / 1e9;
      const price = (sqrtPrice * sqrtPrice) / 1e18;
      
      return {
        id: `market-${market.address}`,
        symbol: `MARKET${index + 1}`, // You'd fetch real token metadata
        name: `Market ${index + 1}`,
        address: market.token_1, // Use token_1 as the address (non-FeelsSOL token)
        imageUrl: feelsGuyImage.src, // Using feels guy as placeholder
        price: price,
        priceChange24h: 0, // Would calculate from historical data
        marketCap: '$0', // Would calculate from circulating supply
        volume24h: '$0', // Would get from market stats
        launched: 'Live'
      };
    });

    setDisplayTokens(marketTokens);
  }, [dataSource, markets]);

  // Show loading state when fetching indexer data
  if (dataSource === 'indexer' && loading && displayTokens === HOMEPAGE_TOKENS) {
    return (
      <div id="home-page" className="container mx-auto px-4 pt-4 pb-8">
        <div className="flex items-center justify-center p-8">
          <div className="flex flex-col items-center space-y-4">
            <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary"></div>
            <p className="text-muted-foreground">Loading market data...</p>
          </div>
        </div>
      </div>
    );
  }

  // Show error state if indexer fails
  if (dataSource === 'indexer' && error) {
    return (
      <div id="home-page" className="container mx-auto px-4 pt-4 pb-8">
        <div className="text-center p-8">
          <p className="text-muted-foreground">Failed to load market data. Showing test data instead.</p>
        </div>
        <div id="token-grid" className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {HOMEPAGE_TOKENS.map((token, index) => (
          <Link 
            key={token.id} 
            href={`/token/${token.address}`} 
            className="block group"
            id={`token-card-link-${index}`}
          >
            <Card 
              id={`token-card-${token.symbol.toLowerCase()}`}
              className="h-full hover:shadow-lg transition-shadow cursor-pointer"
            >
              <CardHeader id={`token-header-${token.symbol.toLowerCase()}`} className="pb-2">
                <div id={`token-header-content-${token.symbol.toLowerCase()}`} className="flex items-center justify-between mb-1">
                  <div id={`token-image-container-${token.symbol.toLowerCase()}`} className="w-14 h-14 rounded-full overflow-hidden bg-muted">
                    <img 
                      id={`token-image-${token.symbol.toLowerCase()}`}
                      src={token.imageUrl} 
                      alt={token.name}
                      className="w-full h-full object-cover"
                      loading="lazy"
                    />
                  </div>
                  <Badge 
                    id={`token-launch-badge-${token.symbol.toLowerCase()}`}
                    variant="outline" 
                    className="text-xs"
                  >
                    {token.launched}
                  </Badge>
                </div>
                <div id={`token-title-container-${token.symbol.toLowerCase()}`}>
                  <h3 id={`token-name-${token.symbol.toLowerCase()}`} className="font-semibold flex items-center gap-2">
                    <span id={`token-name-text-${token.symbol.toLowerCase()}`}>{token.name}</span>
                    <span id={`token-symbol-${token.symbol.toLowerCase()}`} className="text-sm text-muted-foreground/70">${token.symbol}</span>
                  </h3>
                </div>
              </CardHeader>
              <CardContent id={`token-content-${token.symbol.toLowerCase()}`} className="pt-0">
                <div id={`token-stats-${token.symbol.toLowerCase()}`} className="space-y-1">
                  <div id={`token-price-row-${token.symbol.toLowerCase()}`} className="flex justify-between items-center">
                    <span className="text-xs text-muted-foreground">Price</span>
                    <span id={`token-price-value-${token.symbol.toLowerCase()}`} className="text-sm font-semibold">${token.price.toFixed(4)}</span>
                  </div>
                  <div id={`token-change-row-${token.symbol.toLowerCase()}`} className="flex justify-between items-center">
                    <span className="text-xs text-muted-foreground">24h</span>
                    <span 
                      id={`token-change-value-${token.symbol.toLowerCase()}`}
                      className={`text-sm font-medium ${token.priceChange24h >= 0 ? 'text-primary' : 'text-red-500'}`}
                    >
                      {token.priceChange24h >= 0 ? '+' : ''}{token.priceChange24h.toFixed(2)}%
                    </span>
                  </div>
                  <div id={`token-market-cap-row-${token.symbol.toLowerCase()}`} className="flex justify-between items-center">
                    <span className="text-xs text-muted-foreground">Market Cap</span>
                    <span id={`token-market-cap-value-${token.symbol.toLowerCase()}`} className="text-sm font-medium">{token.marketCap}</span>
                  </div>
                  <div id={`token-volume-row-${token.symbol.toLowerCase()}`} className="flex justify-between items-center">
                    <span className="text-xs text-muted-foreground">Volume</span>
                    <span id={`token-volume-value-${token.symbol.toLowerCase()}`} className="text-sm font-medium">{token.volume24h}</span>
                  </div>
                </div>
              </CardContent>
            </Card>
          </Link>
        ))}
      </div>
    </div>
    );
  }

  // Main return statement - use displayTokens which will be either test data or indexer data
  return (
    <div id="home-page" className="container mx-auto px-4 pt-4 pb-8">
      {dataSource === 'indexer' && displayTokens.length === 0 ? (
        <div className="text-center p-8">
          <h3 className="text-lg font-semibold mb-2">No Markets Available</h3>
          <p className="text-muted-foreground">
            The indexer is connected but no markets have been created yet.
          </p>
          <p className="text-muted-foreground text-sm mt-2">
            Create markets through the protocol to see them here.
          </p>
        </div>
      ) : (
        <div id="token-grid" className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {displayTokens.map((token, index) => (
          <Link 
            key={token.id} 
            href={`/token/${token.address}`} 
            className="block group"
            id={`token-card-link-${index}`}
          >
            <Card 
              id={`token-card-${token.symbol.toLowerCase()}`}
              className="h-full hover:shadow-lg transition-shadow cursor-pointer"
            >
              <CardHeader id={`token-header-${token.symbol.toLowerCase()}`} className="pb-2">
                <div id={`token-header-content-${token.symbol.toLowerCase()}`} className="flex items-center justify-between mb-1">
                  <div id={`token-image-container-${token.symbol.toLowerCase()}`} className="w-14 h-14 rounded-full overflow-hidden bg-muted">
                    <img 
                      id={`token-image-${token.symbol.toLowerCase()}`}
                      src={token.imageUrl} 
                      alt={token.name}
                      className="w-full h-full object-cover"
                      loading="lazy"
                    />
                  </div>
                  <Badge 
                    id={`token-launch-badge-${token.symbol.toLowerCase()}`}
                    variant="outline" 
                    className="text-xs"
                  >
                    {token.launched}
                  </Badge>
                </div>
                <div id={`token-title-container-${token.symbol.toLowerCase()}`}>
                  <h3 id={`token-name-${token.symbol.toLowerCase()}`} className="font-semibold flex items-center gap-2">
                    <span id={`token-name-text-${token.symbol.toLowerCase()}`}>{token.name}</span>
                    <span id={`token-symbol-${token.symbol.toLowerCase()}`} className="text-sm text-muted-foreground/70">${token.symbol}</span>
                  </h3>
                </div>
              </CardHeader>
              <CardContent id={`token-content-${token.symbol.toLowerCase()}`} className="pt-0">
                <div id={`token-stats-${token.symbol.toLowerCase()}`} className="space-y-1">
                  <div id={`token-price-row-${token.symbol.toLowerCase()}`} className="flex justify-between items-center">
                    <span className="text-xs text-muted-foreground">Price</span>
                    <span id={`token-price-value-${token.symbol.toLowerCase()}`} className="text-sm font-semibold">${token.price.toFixed(4)}</span>
                  </div>
                  <div id={`token-change-row-${token.symbol.toLowerCase()}`} className="flex justify-between items-center">
                    <span className="text-xs text-muted-foreground">24h</span>
                    <span 
                      id={`token-change-value-${token.symbol.toLowerCase()}`}
                      className={`text-sm font-medium ${token.priceChange24h >= 0 ? 'text-primary' : 'text-red-500'}`}
                    >
                      {token.priceChange24h >= 0 ? '+' : ''}{token.priceChange24h.toFixed(2)}%
                    </span>
                  </div>
                  <div id={`token-market-cap-row-${token.symbol.toLowerCase()}`} className="flex justify-between items-center">
                    <span className="text-xs text-muted-foreground">Market Cap</span>
                    <span id={`token-market-cap-value-${token.symbol.toLowerCase()}`} className="text-sm font-medium">{token.marketCap}</span>
                  </div>
                  <div id={`token-volume-row-${token.symbol.toLowerCase()}`} className="flex justify-between items-center">
                    <span className="text-xs text-muted-foreground">Volume</span>
                    <span id={`token-volume-value-${token.symbol.toLowerCase()}`} className="text-sm font-medium">{token.volume24h}</span>
                  </div>
                </div>
              </CardContent>
            </Card>
          </Link>
        ))}
        </div>
      )}
    </div>
  );
}