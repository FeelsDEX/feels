'use client';

import { useState, useEffect } from 'react';
import { LAMPORTS_PER_SOL } from '@solana/web3.js';
import { useWallet } from '@solana/wallet-adapter-react';
import { getConnection } from '@/services/connection';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { RefreshCw, Droplets } from 'lucide-react';

export default function FaucetPage() {
  const { publicKey, connected } = useWallet();
  const connection = getConnection();
  const [airdropping, setAirdropping] = useState(false);
  const [balance, setBalance] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Fetch balance when wallet connects
  useEffect(() => {
    const fetchBalance = async () => {
      if (connection && publicKey) {
        try {
          const bal = await connection.getBalance(publicKey);
          setBalance(bal / LAMPORTS_PER_SOL);
        } catch (err) {
          console.error('Failed to fetch balance:', err);
        }
      } else {
        setBalance(null);
      }
    };

    fetchBalance();
  }, [connection, publicKey, connected]);

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
    setError(null);
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

  return (
    <div className="container mx-auto px-4 pt-4 pb-8">
      <div className="max-w-6xl mx-auto">
        <div className="max-w-2xl mx-auto">
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
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-1">
                    <span className="text-sm font-medium text-muted-foreground">Wallet Address</span>
                    <div>
                      <Badge variant="outline" className="font-mono text-xs">
                        {publicKey?.toBase58().slice(0, 4)}...{publicKey?.toBase58().slice(-4)}
                      </Badge>
                    </div>
                  </div>
                  <div className="space-y-1">
                    <span className="text-sm font-medium text-muted-foreground">Current Balance</span>
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

                {error && (
                  <div className="p-4 bg-destructive/10 text-destructive rounded-lg text-sm">
                    {error}
                  </div>
                )}
              </>
            )}
          </CardContent>
        </Card>
        </div>
      </div>
    </div>
  );
}

