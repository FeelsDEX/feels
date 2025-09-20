'use client';

import { useMarkets } from '@/hooks/useIndexer';

interface MarketSelectorProps {
  selectedMarket: string | null;
  onMarketSelect: (marketAddress: string) => void;
}

export function MarketSelector({ selectedMarket, onMarketSelect }: MarketSelectorProps) {
  const marketsData = useMarkets({
    refreshInterval: 30000, // Refresh every 30 seconds
  });

  if (marketsData.loading) {
    return (
      <div className="card">
        <h2 className="text-xl font-semibold text-feels-gray-900 mb-4">
          Select Market
        </h2>
        <div className="animate-pulse space-y-2">
          {[...Array(3)].map((_, i) => (
            <div key={i} className="h-12 bg-feels-gray-200 rounded"></div>
          ))}
        </div>
      </div>
    );
  }

  if (marketsData.error) {
    return (
      <div className="card">
        <h2 className="text-xl font-semibold text-feels-gray-900 mb-4">
          Select Market
        </h2>
        <div className="text-feels-red text-sm">
          Error loading markets: {marketsData.error}
        </div>
      </div>
    );
  }

  return (
    <div className="card">
      <h2 className="text-xl font-semibold text-feels-gray-900 mb-4">
        Select Market
      </h2>
      
      {!marketsData.data || marketsData.data.length === 0 ? (
        <div className="text-center py-6">
          <div className="w-12 h-12 bg-feels-gray-100 rounded-full flex items-center justify-center mx-auto mb-3">
            <span className="text-lg">#</span>
          </div>
          <p className="text-feels-gray-600 text-sm">
            No markets available
          </p>
        </div>
      ) : (
        <div className="space-y-2">
          {marketsData.data.map((market) => (
            <button
              key={market.address}
              onClick={() => onMarketSelect(market.address)}
              className={`w-full text-left p-3 rounded-lg border transition-all ${
                selectedMarket === market.address
                  ? 'border-feels-purple bg-feels-purple/5 text-feels-purple'
                  : 'border-feels-gray-200 hover:border-feels-gray-300 hover:bg-feels-gray-50'
              }`}
            >
              <div className="flex items-center justify-between mb-1">
                <div className="font-medium text-sm">
                  Market {market.address.slice(0, 8)}...
                </div>
                <div className="flex items-center space-x-1">
                  <span className={`w-2 h-2 rounded-full ${
                    market.is_paused ? 'bg-feels-red' : 'bg-feels-green'
                  }`}></span>
                  <span className="text-xs text-feels-gray-500">
                    {market.is_paused ? 'Paused' : 'Active'}
                  </span>
                </div>
              </div>
              
              <div className="text-xs text-feels-gray-600 space-y-1">
                <div>Tokens: {market.token_0.slice(0, 6)}.../{market.token_1.slice(0, 6)}...</div>
                <div>Phase: {market.phase}</div>
                <div>Fee: {market.fee_bps / 100}%</div>
              </div>
            </button>
          ))}
        </div>
      )}
      
      {marketsData.lastUpdated && (
        <div className="mt-4 pt-4 border-t border-feels-gray-200">
          <div className="text-xs text-feels-gray-500">
            Last updated: {new Date(marketsData.lastUpdated).toLocaleTimeString()}
          </div>
        </div>
      )}
    </div>
  );
}
