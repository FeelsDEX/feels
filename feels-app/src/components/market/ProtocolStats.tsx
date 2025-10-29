'use client';

import { useState, useEffect } from 'react';
import { Connection, PublicKey } from '@solana/web3.js';
import { Program, Idl } from '@coral-xyz/anchor';
import { useProtocolStats } from '@/hooks/useIndexer';

interface ProtocolStatsProps {
  program: Program<Idl> | null;
  connection: Connection;
}

interface ProtocolData {
  configExists: boolean;
  totalMarkets: number;
  protocolVersion: string;
}

export function ProtocolStats({ program, connection }: ProtocolStatsProps) {
  const [onChainData, setOnChainData] = useState<ProtocolData | null>(null);
  const [onChainLoading, setOnChainLoading] = useState(true);
  const [onChainError, setOnChainError] = useState<string | null>(null);
  
  // Use indexer data
  const indexerStats = useProtocolStats({
    refreshInterval: 10000, // Refresh every 10 seconds
  });

  useEffect(() => {
    async function fetchOnChainData() {
      // Skip if program or connection is not available
      if (!program || !connection || !program.programId) {
        setOnChainLoading(false);
        return;
      }

      try {
        setOnChainLoading(true);
        setOnChainError(null);

        // Derive protocol config PDA with error handling
        let protocolConfigPDA: PublicKey;
        try {
          [protocolConfigPDA] = PublicKey.findProgramAddressSync(
            [Buffer.from('protocol_config')],
            program.programId
          );
        } catch (pdaError) {
          console.error('Failed to derive PDA:', pdaError);
          throw new Error('Invalid program ID or PDA derivation failed');
        }

        // Check if protocol config exists
        let configExists = false;
        try {
          const configAccount = await connection.getAccountInfo(protocolConfigPDA);
          configExists = configAccount !== null;
        } catch (err) {
          console.log('Protocol config not found:', err);
        }

        const protocolData: ProtocolData = {
          configExists,
          totalMarkets: indexerStats.data?.total_markets || 0,
          protocolVersion: '0.1.0',
        };

        setOnChainData(protocolData);
      } catch (err) {
        console.error('Failed to fetch on-chain data:', err);
        setOnChainError(err instanceof Error ? err.message : 'Failed to fetch on-chain data');
      } finally {
        setOnChainLoading(false);
      }
    }

    fetchOnChainData();
  }, [program, connection, indexerStats.data?.total_markets]);

  const loading = onChainLoading || indexerStats.loading;
  const error = onChainError || indexerStats.error;
  const data = onChainData;

  if (loading) {
    return (
      <div className="card">
        <h2 className="text-2xl font-semibold text-feels-gray-900 mb-4">
          Protocol Statistics
        </h2>
        <div className="animate-pulse space-y-4">
          <div className="h-4 bg-feels-gray-200 rounded w-1/4"></div>
          <div className="h-4 bg-feels-gray-200 rounded w-1/2"></div>
          <div className="h-4 bg-feels-gray-200 rounded w-1/3"></div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="card">
        <h2 className="text-2xl font-semibold text-feels-gray-900 mb-4">
          Protocol Statistics
        </h2>
        <div className="text-feels-red text-sm">
          Error: {error}
        </div>
      </div>
    );
  }

  return (
    <div className="card">
      <h2 className="text-2xl font-semibold text-feels-gray-900 mb-6">
        Protocol Statistics
      </h2>
      
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <div className="text-center p-4 bg-feels-gray-50 rounded-lg">
          <div className="text-2xl font-bold mb-2">
            {data?.configExists ? (
              <span className="text-feels-purple">[OK]</span>
            ) : (
              <span className="text-black">[<span className="text-danger-600">X</span>]</span>
            )}
          </div>
          <div className="text-sm font-medium text-feels-gray-900">
            Protocol Config
          </div>
          <div className="text-xs text-feels-gray-600 mt-1">
            {data?.configExists ? 'Initialized' : 'Not Found'}
          </div>
        </div>

        <div className="text-center p-4 bg-feels-gray-50 rounded-lg">
          <div className="text-2xl font-bold text-feels-blue mb-2">
            {indexerStats.data?.total_markets || 0}
          </div>
          <div className="text-sm font-medium text-feels-gray-900">
            Total Markets
          </div>
          <div className="text-xs text-feels-gray-600 mt-1">
            Active trading pairs
          </div>
        </div>

        <div className="text-center p-4 bg-feels-gray-50 rounded-lg">
          <div className="text-2xl font-bold text-feels-green mb-2">
            ${indexerStats.data?.total_volume_24h ? 
              (indexerStats.data.total_volume_24h / 1000000).toFixed(1) + 'M' : 
              '0'
            }
          </div>
          <div className="text-sm font-medium text-feels-gray-900">
            24h Volume
          </div>
          <div className="text-xs text-feels-gray-600 mt-1">
            Total trading volume
          </div>
        </div>

        <div className="text-center p-4 bg-feels-gray-50 rounded-lg">
          <div className="text-2xl font-bold text-feels-orange mb-2">
            {indexerStats.data?.active_positions || 0}
          </div>
          <div className="text-sm font-medium text-feels-gray-900">
            Active Positions
          </div>
          <div className="text-xs text-feels-gray-600 mt-1">
            Open LP positions
          </div>
        </div>
      </div>

      {/* Additional indexer stats */}
      {indexerStats.data && (
        <div className="mt-6 p-4 bg-feels-blue/5 rounded-lg border border-feels-blue/20">
          <h3 className="text-lg font-medium text-feels-gray-900 mb-3">
            Real-time Protocol Metrics
          </h3>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
            <div>
              <span className="text-feels-gray-600">Total Liquidity:</span>
              <span className="ml-2 font-medium">
                ${indexerStats.data.total_liquidity ? 
                  (parseFloat(indexerStats.data.total_liquidity) / 1000000).toFixed(2) + 'M' : 
                  '0'
                }
              </span>
            </div>
            <div>
              <span className="text-feels-gray-600">24h Fees:</span>
              <span className="ml-2 font-medium">
                ${indexerStats.data.total_fees_24h ? 
                  (indexerStats.data.total_fees_24h / 1000).toFixed(1) + 'K' : 
                  '0'
                }
              </span>
            </div>
            <div>
              <span className="text-feels-gray-600">Last Updated:</span>
              <span className="ml-2 font-medium">
                {indexerStats.lastUpdated ? 
                  new Date(indexerStats.lastUpdated).toLocaleTimeString() : 
                  'Never'
                }
              </span>
            </div>
          </div>
        </div>
      )}

      <div className="mt-6 p-4 bg-feels-blue/5 rounded-lg border border-feels-blue/20">
        <div className="flex items-start space-x-3">
          <div className="w-5 h-5 text-feels-blue mt-0.5">i</div>
          <div>
            <h3 className="text-sm font-medium text-feels-gray-900 mb-1">
              Test Environment
            </h3>
            <p className="text-sm text-feels-gray-600">
              This application is connected to Solana Devnet for testing purposes. 
              Protocol statistics are limited in the test environment.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
