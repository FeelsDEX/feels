'use client';

import { useState, useEffect } from 'react';
import { Program, AnchorProvider, Idl } from '@coral-xyz/anchor';
import { useWallet } from '@solana/wallet-adapter-react';
import { getConnection } from '@/services/connection';
import { createFeelsProgram } from '@/program/program-workaround';

import { MarketExplorer } from '@/components/market/MarketExplorer';
import { FeelsMetrics } from '@/components/market/FeelsMetrics';

// Import shadcn/ui components
// import { Button } from '@/components/ui/button';

export default function InfoPage() {
  const { publicKey, signTransaction, signAllTransactions } = useWallet();
  const [program, setProgram] = useState<Program<Idl> | null>(null);
  const [loading, setLoading] = useState(false); // Start with false since connection is instant
  const [, setError] = useState<string | null>(null);
  const [fallback, setFallback] = useState<boolean>(false);
  
  // Use singleton connection
  const connection = getConnection();

  // Initialize program
  useEffect(() => {
    async function initializeProgram() {
      try {
        setLoading(true);
        setError(null);

        if (publicKey && signTransaction && signAllTransactions) {
          try {
            // Create provider and program
            const provider = new AnchorProvider(
              connection,
              {
                publicKey,
                signTransaction,
                signAllTransactions,
              } as any,
              { commitment: 'confirmed' }
            );
            
            // Create program with proper PublicKey
            const feelProgram = createFeelsProgram(provider);
            setProgram(feelProgram);
          } catch (programError) {
            console.error('Failed to create program:', programError);
            // Enter fallback mode (test data) instead of blocking the page
            setFallback(true);
            setProgram(null);
            setError(null);
          }
        } else {
          // Clear program if wallet is disconnected
          setProgram(null);
        }
        
        setLoading(false);
      } catch (err) {
        console.error('Failed to initialize:', err);
        setError(err instanceof Error ? err.message : 'Failed to initialize');
        setLoading(false);
      }
    }

    initializeProgram();
  }, [publicKey, signTransaction, signAllTransactions, connection]);

  if (loading) {
    return (
      <div className="container mx-auto px-4 py-8">
        <div className="bg-card text-card-foreground p-6 pixel-container max-w-md mx-auto">
          <div className="flex items-center justify-center p-8">
            <div className="flex flex-col items-center space-y-4">
              <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary"></div>
              <p className="text-muted-foreground text-base">Initializing Feels Protocol...</p>
            </div>
          </div>
        </div>
      </div>
    );
  }

  // Do not hard-block the page on errors; show a small fallback banner instead

  return (
    <div className="container mx-auto px-4 py-8 space-y-8">
      {fallback && (
        <div className="relative p-3 rounded-md bg-amber-50 text-amber-800 border border-amber-200">
          <div className="pr-6">Feels program not yet initialized. Falling back to test data.</div>
          <button
            type="button"
            aria-label="Close"
            onClick={() => setFallback(false)}
            className="absolute right-4 top-[calc(50%-2px)] -translate-y-1/2 text-amber-800/80 hover:text-amber-900"
          >
            ×
          </button>
        </div>
      )}
      
      {/* Feels Metrics - Full Width */}
      {connection && (
        <FeelsMetrics 
          program={program} 
          connection={connection} 
        />
      )}

      {/* Market Explorer */}
      {connection && (
        <MarketExplorer 
          program={program} 
          connection={connection} 
        />
      )}

    </div>
  );
}
