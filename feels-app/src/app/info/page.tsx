'use client';

import { useState, useEffect } from 'react';
import { Program, AnchorProvider, Idl } from '@coral-xyz/anchor';
import { useWallet } from '@solana/wallet-adapter-react';
import { getConnection } from '@/services/connection';
import { createFeelsProgram } from '@/program/program-workaround';

import { MarketExplorer } from '@/components/market/MarketExplorer';
import { FeelsMetrics } from '@/components/market/FeelsMetrics';
import { ProtocolParametersAdmin } from '@/components/market/ProtocolParametersAdmin';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { FEELS_IDL, FEELS_PROGRAM_ID } from '@/program/sdk';

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
      {/* Main Layout - Two Columns */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Left Column - Protocol Parameters and SDK Information */}
        <div className="space-y-6">
          {/* Protocol Parameters */}
          {connection && (
            <ProtocolParametersAdmin 
              program={program} 
              connection={connection} 
              fallback={fallback}
            />
          )}

          {/* SDK Information */}
          <Card>
            <CardHeader>
              <CardTitle className="text-xl">SDK Information</CardTitle>
              <CardDescription className="text-base">
                Details about the Feels Protocol program and available instructions
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {/* Program Details - Compact Layout */}
                <div className="grid grid-cols-1 gap-4 text-sm">
                  <div>
                    <span className="text-muted-foreground">Program ID:</span>
                    <div className="font-mono text-xs mt-1 break-all">
                      {FEELS_PROGRAM_ID}
                    </div>
                  </div>
                  <div>
                    <span className="text-muted-foreground">Version:</span>
                    <div className="mt-1">{(FEELS_IDL as any)?.metadata?.version || (FEELS_IDL as any)?.version || 'Unknown'}</div>
                  </div>
                </div>
                
                {/* Instructions - Compact Grid */}
                <div>
                  <h3 className="text-base font-medium mb-2">Instructions ({FEELS_IDL?.instructions?.length || 0})</h3>
                  <div className="grid grid-cols-2 gap-1">
                    {(FEELS_IDL?.instructions || []).map((instruction: any, index: number) => (
                      <div key={index} className="text-xs font-mono bg-muted px-2 py-1 rounded">
                        {instruction.name}
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Right Column - Feels Metrics */}
        <div className="space-y-6">
          {connection && (
            <FeelsMetrics 
              program={program} 
              connection={connection} 
            />
          )}
        </div>
      </div>

      {/* Market Explorer - Full Width */}
      {connection && (
        <MarketExplorer 
          program={program} 
          connection={connection} 
        />
      )}

    </div>
  );
}
