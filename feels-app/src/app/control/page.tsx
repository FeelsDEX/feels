'use client';

import { useState, useEffect } from 'react';
import { LAMPORTS_PER_SOL } from '@solana/web3.js';
import { Program, AnchorProvider, Idl } from '@coral-xyz/anchor';
import { useWallet } from '@solana/wallet-adapter-react';
import { getConnection } from '@/services/connection';
import { FEELS_IDL, FEELS_PROGRAM_ID } from '@/sdk/sdk';
import { createFeelsProgram } from '@/sdk/program-workaround';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { RefreshCw, Droplets } from 'lucide-react';
import { ProtocolParametersAdmin } from '@/components/market/ProtocolParametersAdmin';
import { NetworkConnection } from '@/components/common/NetworkConnection';

export default function ControlPage() {
  const { publicKey, signTransaction, signAllTransactions, connected } = useWallet();
  const connection = getConnection(); // Use singleton connection
  const [program, setProgram] = useState<Program<Idl> | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [airdropping, setAirdropping] = useState(false);
  const [balance, setBalance] = useState<number | null>(null);

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
              throw new Error('IDL not loaded properly. Please run: just idl-build');
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

            // Get initial balance
            const bal = await connection.getBalance(publicKey);
            setBalance(bal / LAMPORTS_PER_SOL);
          } catch (programError) {
            console.error('Failed to create program:', programError);
            setError(`Program initialization failed: ${programError instanceof Error ? programError.message : 'Unknown error'}`);
          }
        } else {
          // Clear program if wallet is disconnected
          setProgram(null);
          setBalance(null);
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

  // Refresh balance
  const refreshBalance = async () => {
    if (!connection || !publicKey) return;
    try {
      const bal = await connection.getBalance(publicKey);
      setBalance(bal / LAMPORTS_PER_SOL);
    } catch (err) {
      console.error('Failed to fetch balance:', err);
    }
  };

  // Handle airdrop
  const handleAirdrop = async () => {
    if (!connection || !publicKey) return;

    setAirdropping(true);
    try {
      // Request airdrop of 2 SOL
      const signature = await connection.requestAirdrop(
        publicKey,
        2 * LAMPORTS_PER_SOL
      );
      
      // Wait for confirmation
      await connection.confirmTransaction(signature, 'confirmed');
      
      // Refresh balance
      await refreshBalance();
      
      // Show success
      console.log('Airdrop successful:', signature);
    } catch (err) {
      console.error('Airdrop failed:', err);
      setError('Airdrop failed. Please try again.');
    } finally {
      setAirdropping(false);
    }
  };

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
          {/* Network Connection */}
          <NetworkConnection />

          {/* Devnet Faucet */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Droplets className="h-5 w-5" />
                Devnet Faucet
              </CardTitle>
              <CardDescription>
                Request test SOL for development and testing
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {!connected ? (
                <div className="text-center py-8">
                  <p className="text-muted-foreground mb-4">Connect your wallet to use the faucet</p>
                </div>
              ) : (
                <>
                  <div className="space-y-2">
                    <div className="flex justify-between items-center">
                      <span className="text-sm font-medium">Wallet Address</span>
                      <Badge variant="outline" className="font-mono text-xs">
                        {publicKey?.toBase58().slice(0, 4)}...{publicKey?.toBase58().slice(-4)}
                      </Badge>
                    </div>
                    <div className="flex justify-between items-center">
                      <span className="text-sm font-medium">Current Balance</span>
                      <div className="flex items-center gap-2">
                        <span className="font-mono">{balance?.toFixed(4) ?? '0.0000'} SOL</span>
                        <button
                          onClick={refreshBalance}
                          className="p-1 hover:bg-muted rounded transition-colors"
                          disabled={!connection || !publicKey}
                        >
                          <RefreshCw className="h-3 w-3" />
                        </button>
                      </div>
                    </div>
                  </div>

                  <div className="pt-4">
                    <Button
                      onClick={handleAirdrop}
                      disabled={airdropping || !connection}
                      className="w-full"
                    >
                      {airdropping ? (
                        <div className="flex items-center gap-2">
                          <RefreshCw className="h-4 w-4 animate-spin" />
                          Airdropping...
                        </div>
                      ) : (
                        'Request 2 SOL'
                      )}
                    </Button>
                    <p className="text-xs text-muted-foreground text-center mt-2">
                      Devnet only • Max 2 SOL per request
                    </p>
                  </div>
                </>
              )}
            </CardContent>
          </Card>

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
      </div>

      {error && (
        <div className="mt-6 p-4 bg-destructive/10 text-destructive rounded-lg">
          {error}
        </div>
      )}
    </div>
  );
}