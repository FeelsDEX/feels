'use client';

import { useState, useEffect } from 'react';
import { Connection, PublicKey } from '@solana/web3.js';
import { Program, Idl } from '@coral-xyz/anchor';
import { useMarkets } from '@/hooks/useIndexer';

interface MarketInfoProps {
  program: Program<Idl> | null;
  connection: Connection;
}

export function MarketInfo({ program, connection }: MarketInfoProps) {
  // Use indexer data for markets
  const marketsData = useMarkets({
    refreshInterval: 15000, // Refresh every 15 seconds
  });

  if (marketsData.loading) {
    return (
      <div className="card">
        <h2 className="text-2xl font-semibold text-feels-gray-900 mb-4">
          Market Information
        </h2>
        <div className="animate-pulse space-y-4">
          <div className="h-4 bg-feels-gray-200 rounded w-full"></div>
          <div className="h-4 bg-feels-gray-200 rounded w-3/4"></div>
          <div className="h-4 bg-feels-gray-200 rounded w-1/2"></div>
        </div>
      </div>
    );
  }

  if (marketsData.error) {
    return (
      <div className="card">
        <h2 className="text-2xl font-semibold text-feels-gray-900 mb-4">
          Market Information
        </h2>
        <div className="text-feels-red text-sm">
          Error: {marketsData.error}
        </div>
      </div>
    );
  }

  return (
    <div className="card">
      <h2 className="text-2xl font-semibold text-feels-gray-900 mb-6">
        Market Information
      </h2>

      {!marketsData.data || marketsData.data.length === 0 ? (
        <div className="text-center py-8">
          <div className="w-16 h-16 bg-feels-gray-100 rounded-full flex items-center justify-center mx-auto mb-4">
            <span className="text-2xl">Markets</span>
          </div>
          <h3 className="text-lg font-medium text-feels-gray-900 mb-2">
            No Markets Found
          </h3>
          <p className="text-feels-gray-600 mb-4">
            No active markets were found on this network. This is expected in a test environment.
          </p>
          <div className="bg-feels-orange/10 border border-feels-orange/20 rounded-lg p-4 max-w-md mx-auto">
            <div className="flex items-start space-x-3">
              <div className="w-5 h-5 text-feels-orange mt-0.5 text-lg">Warning</div>
              <div className="text-left">
                <h4 className="text-sm font-medium text-feels-gray-900 mb-1">
                  Test Environment
                </h4>
                <p className="text-sm text-feels-gray-600">
                  To see markets, you would need to deploy the protocol and create markets on devnet.
                </p>
              </div>
            </div>
          </div>
        </div>
      ) : (
        <div className="space-y-4">
          {marketsData.data?.map((market, index) => (
            <div key={market.address} className="border border-feels-gray-200 rounded-lg p-4">
              <div className="flex items-center justify-between mb-2">
                <div className="font-medium text-feels-gray-900">
                  Market #{index + 1}
                </div>
                <div className="flex space-x-2">
                  <span className={`px-2 py-1 text-xs rounded-full ${
                    market.phase === 'SteadyState' 
                      ? 'bg-feels-green/10 text-feels-green' 
                      : 'bg-feels-orange/10 text-feels-orange'
                  }`}>
                    {market.phase}
                  </span>
                  <span className={`px-2 py-1 text-xs rounded-full ${
                    market.is_paused 
                      ? 'bg-feels-red/10 text-feels-red' 
                      : 'bg-feels-green/10 text-feels-green'
                  }`}>
                    {market.is_paused ? 'Paused' : 'Active'}
                  </span>
                </div>
              </div>
              <div className="text-sm text-feels-gray-600 space-y-1">
                <div>Address: <span className="code">{market.address}</span></div>
                <div>Token 0: <span className="code">{market.token_0}</span></div>
                <div>Token 1: <span className="code">{market.token_1}</span></div>
                <div>Current Tick: <span className="code">{market.current_tick}</span></div>
                <div>Fee: <span className="code">{market.fee_bps / 100}%</span></div>
                <div>Liquidity: <span className="code">{market.liquidity}</span></div>
              </div>
              <div className="mt-3 pt-3 border-t border-feels-gray-100">
                <div className="text-xs text-feels-gray-500">
                  Last updated: {new Date(market.last_updated_timestamp * 1000).toLocaleString()}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      <div className="mt-6 p-4 bg-feels-purple/5 rounded-lg border border-feels-purple/20">
        <div className="flex items-start space-x-3">
          <div className="w-5 h-5 text-feels-purple mt-0.5">Info</div>
          <div>
            <h3 className="text-sm font-medium text-feels-gray-900 mb-1">
              SDK Integration
            </h3>
            <p className="text-sm text-feels-gray-600">
              This component demonstrates how to query market accounts using the generated TypeScript SDK.
              In a production environment, it would display real market data including liquidity, fees, and trading activity.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
