'use client';

import { useState, useEffect } from 'react';
import { Program, AnchorProvider, Idl } from '@coral-xyz/anchor';
import { useWallet } from '@solana/wallet-adapter-react';
import { getConnection } from '@/services/connection';
import { createFeelsProgram } from '@/sdk/program-workaround';

import { MarketExplorer } from '@/components/market/MarketExplorer';
import { FeelsMetrics } from '@/components/market/FeelsMetrics';

// Import shadcn/ui components
import { Button } from '@/components/ui/button';

export default function InfoPage() {
  const { publicKey, signTransaction, signAllTransactions } = useWallet();
  const [program, setProgram] = useState<Program<Idl> | null>(null);
  const [loading, setLoading] = useState(false); // Start with false since connection is instant
  const [error, setError] = useState<string | null>(null);
  
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
            setError(`Program initialization failed: ${programError instanceof Error ? programError.message : 'Unknown error'}`);
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

  if (error) {
    return (
      <div className="container mx-auto px-4 py-8">
        <div className="bg-card text-card-foreground p-6 pixel-container max-w-2xl mx-auto">
          <h2 className="text-xl font-medium mb-4 flex items-center gap-2">
            <span className="text-xl">Warning</span>
            Connection Error
          </h2>
          <p className="text-muted-foreground mb-4">{error}</p>
          <Button 
            onClick={() => window.location.reload()} 
            className="btn btn-outline"
          >
            Retry Connection
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-8 space-y-8">
      
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
