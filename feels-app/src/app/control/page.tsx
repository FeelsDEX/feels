'use client';

import { useState, useEffect } from 'react';
import { Program, AnchorProvider, Idl } from '@coral-xyz/anchor';
import { useWallet } from '@solana/wallet-adapter-react';
import { getConnection } from '@/services/connection';
import { createFeelsProgram } from '@/program/program-workaround';
import { FEELS_IDL } from '@/program/sdk';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { ProtocolParametersAdmin } from '@/components/market/ProtocolParametersAdmin';
import { NetworkConnection } from '@/components/common/NetworkConnection';
import { ProgramStatus } from '@/components/admin/ProgramStatus';
import { useDeveloperMode } from '@/contexts/DeveloperModeContext';
import { Switch } from '@/components/ui/switch';

export default function ControlPage() {
  const { publicKey, signTransaction, signAllTransactions } = useWallet();
  const { isDeveloperMode, setDeveloperMode } = useDeveloperMode();
  const connection = getConnection(); // Use singleton connection
  const [program, setProgram] = useState<Program<Idl> | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [fallback, setFallback] = useState<boolean>(false);

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
              { publicKey, signTransaction, signAllTransactions } as any,
              { commitment: 'confirmed' }
            );
            
            // Create program with proper PublicKey
            // Check if FEELS_IDL is properly loaded
            if (!FEELS_IDL || typeof FEELS_IDL !== 'object') {
              throw new Error('IDL not loaded properly. Please run: just build idl');
            }
            
            // Debug: Let's see what's in the IDL
            console.log('IDL structure:', {
              hasTypes: !!(FEELS_IDL as any).types,
              typesLength: (FEELS_IDL as any).types?.length,
              firstType: (FEELS_IDL as any).types?.[0],
              hasAccounts: !!(FEELS_IDL as any).accounts,
              accountsLength: (FEELS_IDL as any).accounts?.length,
              firstAccount: (FEELS_IDL as any).accounts?.[0]
            });
            
            // Let's check if the account names match between accounts and types
            const accountNames = (FEELS_IDL as any).accounts?.map((a: any) => a.name) || [];
            const typeNames = (FEELS_IDL as any).types?.map((t: any) => t.name) || [];
            console.log('Account names:', accountNames);
            console.log('Type names:', typeNames);
            
            // Check if each account has a corresponding type
            for (const account of (FEELS_IDL as any).accounts || []) {
              const matchingType = (FEELS_IDL as any).types?.find((t: any) => t.name === account.name);
              console.log(`Account ${account.name} has matching type:`, !!matchingType);
              if (matchingType) {
                console.log(`Type definition for ${account.name}:`, matchingType);
              }
            }
            
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
        <div className="flex items-center justify-center p-8">
          <div className="flex flex-col items-center space-y-4">
            <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary"></div>
            <p className="text-muted-foreground">Initializing Control Panel...</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-8">
      <div className="mb-8">
        <h1 className="text-3xl font-bold mb-2">Control Panel</h1>
        <p className="text-muted-foreground">Manage protocol parameters and test utilities</p>
      </div>

      {/* Main Layout - Protocol Parameters (Left) and Components Stack (Right) */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Left Side - Protocol Parameters (Half Width) */}
        <div>
          {connection ? (
            <ProtocolParametersAdmin 
              program={program} 
              connection={connection} 
              fallback={fallback}
            />
          ) : (
            <Card>
              <CardHeader>
                <CardTitle className="text-xl">Protocol Parameters</CardTitle>
                <CardDescription>Connect wallet to view protocol configuration</CardDescription>
              </CardHeader>
              <CardContent className="py-8">
                <div className="text-center text-muted-foreground">
                  <p>Wallet connection required to load protocol parameters</p>
                </div>
              </CardContent>
            </Card>
          )}
        </div>

        {/* Right Side - Stacked Components */}
        <div className="space-y-6">
          {/* Program Status */}
          {connection && (
            <ProgramStatus 
              connection={connection} 
              program={program}
              fallback={fallback}
            />
          )}

          {/* Developer Mode Toggle */}
          <Card>
            <CardHeader>
              <CardTitle>Developer Mode</CardTitle>
              <CardDescription>
                Toggle developer features and debugging information
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <div className="text-sm font-medium">Enable Developer Mode</div>
                  <div className="text-xs text-muted-foreground">
                    Shows additional debugging information like connection status badges
                  </div>
                </div>
                <Switch
                  checked={isDeveloperMode}
                  onCheckedChange={setDeveloperMode}
                />
              </div>
            </CardContent>
          </Card>

          {/* Network Connection */}
          <NetworkConnection />
        </div>
      </div>

      {error && (
        <div className="mt-6 p-4 bg-destructive/10 text-destructive rounded-lg">
          {error}
        </div>
      )}
    </div>
  );
}