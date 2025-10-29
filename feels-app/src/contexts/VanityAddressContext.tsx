'use client';

import React, { createContext, useContext, useEffect } from 'react';
import { useVanityAddressMiner, type MinerStatus } from '@/hooks/useVanityAddressMiner';
import { useDataSource } from '@/contexts/DataSourceContext';
import type { Keypair } from '@solana/web3.js';

interface VanityAddressContextType {
  status: MinerStatus;
  startMining: () => void;
  stopMining: () => void;
  resetAndMine: () => void;
  getSolanaKeypair: () => Keypair | null;
}

const VanityAddressContext = createContext<VanityAddressContextType | undefined>(undefined);

export function VanityAddressProvider({ children }: { children: React.ReactNode }) {
  const { dataSource } = useDataSource();
  const isTestDataMode = dataSource === 'test';
  const miner = useVanityAddressMiner(isTestDataMode);

  // Start mining automatically when the app loads if we don't have a keypair
  // This allows the system to mine in the background while users navigate the site
  // With optimizations, this should now start much faster
  useEffect(() => {
    if (miner.status.isReady && !miner.status.keypair && !miner.status.isRunning) {
      console.log('Starting optimized vanity address mining in background...');
      miner.startMining();
    }
  }, [miner.status.isReady, miner.status.keypair, miner.status.isRunning, miner.startMining]);

  // Expose miner status and controls to window for debugging
  useEffect(() => {
    if (typeof window !== 'undefined') {
      (window as any).__vanityMinerStatus = miner.status;
      (window as any).__vanityMinerControl = {
        startMining: miner.startMining,
        stopMining: miner.stopMining,
        resetAndMine: miner.resetAndMine,
        getSolanaKeypair: miner.getSolanaKeypair
      };
    }
  }, [miner]);

  return (
    <VanityAddressContext.Provider value={miner}>
      {children}
    </VanityAddressContext.Provider>
  );
}

export function useVanityAddress() {
  const context = useContext(VanityAddressContext);
  if (context === undefined) {
    throw new Error('useVanityAddress must be used within a VanityAddressProvider');
  }
  return context;
}