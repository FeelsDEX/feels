'use client';

import { useMarketSwaps } from '@/hooks/useIndexer';

interface RecentSwapsProps {
  marketAddress?: string;
  limit?: number;
}

export function RecentSwaps({ marketAddress, limit = 10 }: RecentSwapsProps) {
  const swapsData = useMarketSwaps(marketAddress || '', {
    limit,
    refreshInterval: 3000, // Refresh every 3 seconds for recent activity
    enabled: !!marketAddress,
  });

  if (!marketAddress) {
    return (
      <div className="card">
        <h2 className="text-2xl font-semibold text-feels-gray-900 mb-4">
          Recent Swaps
        </h2>
        <div className="text-center py-8">
          <div className="w-16 h-16 bg-feels-gray-100 rounded-full flex items-center justify-center mx-auto mb-4">
            <span className="text-2xl">~</span>
          </div>
          <p className="text-feels-gray-600">
            Select a market to view recent swaps
          </p>
        </div>
      </div>
    );
  }

  if (swapsData.loading) {
    return (
      <div className="card">
        <h2 className="text-2xl font-semibold text-feels-gray-900 mb-4">
          Recent Swaps
        </h2>
        <div className="animate-pulse space-y-4">
          {[...Array(5)].map((_, i) => (
            <div key={i} className="h-16 bg-feels-gray-200 rounded"></div>
          ))}
        </div>
      </div>
    );
  }

  if (swapsData.error) {
    return (
      <div className="card">
        <h2 className="text-2xl font-semibold text-feels-gray-900 mb-4">
          Recent Swaps
        </h2>
        <div className="text-feels-red text-sm">
          Error: {swapsData.error}
        </div>
      </div>
    );
  }

  return (
    <div className="card">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-2xl font-semibold text-feels-gray-900">
          Recent Swaps
        </h2>
        <div className="flex items-center space-x-2 text-sm text-feels-gray-500">
          <div className="w-2 h-2 bg-feels-green rounded-full animate-pulse"></div>
          <span>Live updates</span>
        </div>
      </div>

      {!swapsData.data || swapsData.data.length === 0 ? (
        <div className="text-center py-8">
          <div className="w-16 h-16 bg-feels-gray-100 rounded-full flex items-center justify-center mx-auto mb-4">
            <span className="text-2xl">^</span>
          </div>
          <h3 className="text-lg font-medium text-feels-gray-900 mb-2">
            No Recent Swaps
          </h3>
          <p className="text-feels-gray-600">
            No swap activity found for this market recently.
          </p>
        </div>
      ) : (
        <div className="space-y-3">
          {swapsData.data.map((swap) => (
            <div 
              key={swap.signature} 
              className="border border-feels-gray-200 rounded-lg p-4 hover:bg-feels-gray-50 transition-colors"
            >
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center space-x-2">
                  <div className="text-sm font-medium text-feels-gray-900">
                    Swap #{swap.signature.slice(0, 8)}...
                  </div>
                  <div className="text-xs text-feels-gray-500">
                    {new Date(swap.timestamp * 1000).toLocaleTimeString()}
                  </div>
                </div>
                <div className="text-sm font-medium text-feels-purple">
                  ${((swap.amount_in * swap.price_before) / 1000000).toFixed(2)}
                </div>
              </div>
              
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span className="text-feels-gray-600">Amount In:</span>
                  <div className="font-medium">
                    {(swap.amount_in / 1000000).toFixed(6)}
                  </div>
                </div>
                <div>
                  <span className="text-feels-gray-600">Amount Out:</span>
                  <div className="font-medium">
                    {(swap.amount_out / 1000000).toFixed(6)}
                  </div>
                </div>
                <div>
                  <span className="text-feels-gray-600">Price Impact:</span>
                  <div className={`font-medium ${
                    Math.abs(swap.price_after - swap.price_before) / swap.price_before > 0.01
                      ? 'text-feels-orange'
                      : 'text-feels-green'
                  }`}>
                    {(((swap.price_after - swap.price_before) / swap.price_before) * 100).toFixed(2)}%
                  </div>
                </div>
                <div>
                  <span className="text-feels-gray-600">Fee:</span>
                  <div className="font-medium">
                    ${(swap.fee_amount / 1000000).toFixed(4)}
                  </div>
                </div>
              </div>

              <div className="mt-3 pt-3 border-t border-feels-gray-100">
                <div className="flex items-center justify-between text-xs text-feels-gray-500">
                  <span>User: {swap.user.slice(0, 8)}...{swap.user.slice(-4)}</span>
                  <a 
                    href={`https://explorer.solana.com/tx/${swap.signature}?cluster=devnet`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-feels-blue hover:underline"
                  >
                    View on Explorer
                  </a>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {swapsData.data && swapsData.data.length > 0 && (
        <div className="mt-6 p-4 bg-feels-green/5 rounded-lg border border-feels-green/20">
          <div className="flex items-start space-x-3">
            <div className="w-5 h-5 text-feels-green mt-0.5">#</div>
            <div>
              <h3 className="text-sm font-medium text-feels-gray-900 mb-1">
                Real-time Swap Data
              </h3>
              <p className="text-sm text-feels-gray-600">
                Showing the last {swapsData.data.length} swaps from the indexer. 
                Data updates every 3 seconds with new on-chain activity.
              </p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
