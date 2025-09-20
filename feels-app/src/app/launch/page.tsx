'use client';

import React, { useState, useEffect, Suspense } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import dynamic from 'next/dynamic';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { AlertCircle, Loader2 } from 'lucide-react';
import { useRouter } from 'next/navigation';
import { useIndexer } from '@/hooks/useIndexer';

// Lazy load the CreateMarket component
const CreateMarket = dynamic(
  () => import('@/components/CreateMarket').then(mod => ({ default: mod.CreateMarket })),
  { 
    ssr: false,
    loading: () => (
      <Card className="max-w-2xl mx-auto">
        <CardContent className="p-12">
          <div className="flex flex-col items-center justify-center space-y-4">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            <p className="text-sm text-muted-foreground">Loading market creator...</p>
          </div>
        </CardContent>
      </Card>
    )
  }
);

export default function LaunchPage() {
  const { publicKey } = useWallet();
  const router = useRouter();
  const { isConnected: indexerConnected } = useIndexer();
  const [connection, setConnection] = useState<any>(null);

  // Lazy load connection
  useEffect(() => {
    import('@/services/connection').then(({ getConnection }) => {
      setConnection(getConnection());
    });
  }, []);

  const handleMarketCreated = (marketAddress: string) => {
    console.log('New market created:', marketAddress);
    // Navigate to the new market page
    router.push(`/token/${marketAddress}`);
  };

  return (
    <div id="launch-page-container" className="container mx-auto px-4 py-8">
      {/* Main Content - Create Market */}
      <div id="launch-content-wrapper" className="max-w-6xl mx-auto">
        {publicKey ? (
          <div id="create-market-section" className="max-w-2xl mx-auto">
            {connection && (
              <Suspense fallback={
                <Card className="max-w-2xl mx-auto">
                  <CardContent className="p-12">
                    <div className="flex flex-col items-center justify-center space-y-4">
                      <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                      <p className="text-sm text-muted-foreground">Initializing...</p>
                    </div>
                  </CardContent>
                </Card>
              }>
                <CreateMarket 
                  connection={connection} 
                  onMarketCreated={handleMarketCreated}
                />
              </Suspense>
            )}
            
            <Alert id="market-requirements-alert" className="mt-6">
              <AlertDescription>
                <strong>Requirements:</strong>
                <ul id="requirements-list" className="list-disc list-inside mt-2 space-y-1">
                  <li>You need FeelsSOL tokens for initial liquidity</li>
                  <li>The local validator must be running</li>
                  <li>Ensure the program is deployed</li>
                </ul>
              </AlertDescription>
            </Alert>
            
            {!indexerConnected && (
              <Alert id="indexer-warning-alert" className="mt-4" variant="destructive">
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>
                  The indexer is not running. Your market will be created on-chain but won&apos;t appear in the explorer until the indexer is started.
                </AlertDescription>
              </Alert>
            )}
          </div>
        ) : (
          <Card id="wallet-connect-card" className="max-w-2xl mx-auto">
            <CardHeader>
              <CardTitle id="wallet-connect-title">Connect Wallet to Launch</CardTitle>
              <CardDescription id="wallet-connect-description">
                You need to connect your wallet to create a new market on Feels Protocol
              </CardDescription>
            </CardHeader>
            <CardContent>
              <p id="wallet-connect-message" className="text-center text-muted-foreground">
                Please connect your wallet using the button in the top navigation bar
              </p>
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  );
}