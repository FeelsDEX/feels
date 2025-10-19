'use client';

import React, { useState, useEffect, Suspense } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import dynamic from 'next/dynamic';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
// Alert components removed - no longer needed
import { Loader2 } from 'lucide-react';
import { useRouter } from 'next/navigation';

// Lazy load the CreateMarket component
const CreateMarket = dynamic(
  () => import('@/components/market/CreateMarket').then(mod => ({ default: mod.CreateMarket })),
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
  const [mounted, setMounted] = useState(false);
  const { publicKey } = useWallet();
  const router = useRouter();
  const [connection, setConnection] = useState<any>(null);

  // Ensure component is mounted before using wallet
  useEffect(() => {
    setMounted(true);
  }, []);

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

  // Don't render wallet-dependent content until mounted
  if (!mounted) {
    return (
      <div className="container mx-auto px-4 pt-4 pb-8">
        <Card className="max-w-2xl mx-auto">
          <CardContent className="p-12">
            <div className="flex flex-col items-center justify-center space-y-4">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
              <p className="text-sm text-muted-foreground">Loading...</p>
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div id="launch-page-container" className="container mx-auto px-4 pt-4 pb-8">
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