'use client';

import { useState } from 'react';
import { Connection } from '@solana/web3.js';
import { Program, Idl } from '@coral-xyz/anchor';
import { useMarkets } from '@/hooks/useIndexer';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { FallbackBanner } from '@/components/ui/fallback-banner';
import { DollarSign, Percent, Clock, Hash, Copy } from 'lucide-react';

interface MarketExplorerProps {
  program: Program<Idl> | null;
  connection: Connection;
}

export function MarketExplorer({}: MarketExplorerProps) {
  const [selectedMarket, setSelectedMarket] = useState<string | null>(null);
  const marketsData = useMarkets({
    refreshInterval: 15000, // Refresh every 15 seconds
  });

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  if (marketsData.loading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Market Explorer</CardTitle>
          <CardDescription>Browse and explore active markets on Feels Protocol</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="animate-pulse space-y-4">
            <div className="h-10 bg-muted rounded w-full"></div>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {[...Array(3)].map((_, i) => (
                <div key={i} className="h-32 bg-muted rounded"></div>
              ))}
            </div>
          </div>
        </CardContent>
      </Card>
    );
  }

  if (marketsData.error) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Market Explorer</CardTitle>
          <CardDescription>Browse and explore active markets on Feels Protocol</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="text-destructive text-sm">
            Error loading markets: {marketsData.error}
          </div>
        </CardContent>
      </Card>
    );
  }

  const markets = marketsData.data || [];
  const selectedMarketData = markets.find(m => m.address === selectedMarket);

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>Market Explorer</CardTitle>
            <CardDescription>Browse and explore active markets on Feels Protocol</CardDescription>
          </div>
          {marketsData.lastUpdated && (
            <div className="flex items-center text-xs text-muted-foreground">
              <Clock className="h-3 w-3 mr-1" />
              Updated: {new Date(marketsData.lastUpdated).toLocaleTimeString()}
            </div>
          )}
        </div>
      </CardHeader>
      <CardContent>
        {markets.length === 0 ? (
          <div className="text-center py-12">
            <div className="w-20 h-20 flex items-center justify-center mx-auto mb-4">
              <span className="text-4xl text-muted-foreground font-mono">[MARKETS]</span>
            </div>
            <h3 className="text-lg font-medium mb-2">No Markets Found</h3>
            <p className="text-muted-foreground mb-6 max-w-md mx-auto">
              No active markets were found on this network. This is expected in a test environment.
            </p>
            <div className="max-w-md mx-auto">
              <FallbackBanner
                variant="warning"
                title="Test Environment"
                message="To see markets, you would need to deploy the protocol and create markets on devnet."
                dismissible={false}
              />
            </div>
          </div>
        ) : (
          <Tabs defaultValue="grid" className="w-full">
            <TabsList className="mb-4">
              <TabsTrigger value="grid">Grid View</TabsTrigger>
              <TabsTrigger value="list">List View</TabsTrigger>
            </TabsList>
            
            <TabsContent value="grid">
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {markets.map((market, index) => (
                  <div
                    key={market.address}
                    onClick={() => setSelectedMarket(market.address)}
                    className={`p-4 rounded-lg border-2 transition-all cursor-pointer ${
                      selectedMarket === market.address
                        ? 'border-primary bg-primary/5'
                        : 'border-border hover:border-primary/50 hover:bg-muted/50'
                    }`}
                  >
                    <div className="flex items-start justify-between mb-3">
                      <div>
                        <h3 className="font-medium">Market #{index + 1}</h3>
                        <p className="text-xs text-muted-foreground mt-1">
                          {market.address.slice(0, 8)}...{market.address.slice(-6)}
                        </p>
                      </div>
                      <div className="flex gap-1">
                        <Badge variant={market.is_paused ? "destructive" : "default"} className={`text-xs ${!market.is_paused && 'bg-primary/10 text-primary border-primary/20'}`}>
                          {market.is_paused ? 'Paused' : 'Active'}
                        </Badge>
                      </div>
                    </div>
                    
                    <div className="space-y-2 text-sm">
                      <div className="flex items-center justify-between">
                        <span className="text-muted-foreground">Tokens:</span>
                        <span className="font-mono text-xs">
                          {market.token_0.slice(0, 6)}.../{market.token_1.slice(0, 6)}...
                        </span>
                      </div>
                      <div className="flex items-center justify-between">
                        <span className="text-muted-foreground">Fee:</span>
                        <span className="flex items-center">
                          <Percent className="h-3 w-3 mr-1" />
                          {market.fee_bps / 100}%
                        </span>
                      </div>
                      <div className="flex items-center justify-between">
                        <span className="text-muted-foreground">Phase:</span>
                        <Badge variant="outline" className="text-xs">
                          {market.phase}
                        </Badge>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
              
              {selectedMarketData && (
                <div className="mt-6 p-6 rounded-lg border-2 border-primary bg-primary/5">
                  <h3 className="text-lg font-medium mb-4 flex items-center">
                    <Hash className="h-5 w-5 mr-2" />
                    Selected Market Details
                  </h3>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div className="space-y-3">
                      <div>
                        <p className="text-sm text-muted-foreground mb-1">Market Address</p>
                        <div className="flex items-center gap-2">
                          <code className="text-xs bg-muted px-2 py-1 rounded flex-1 truncate">
                            {selectedMarketData.address}
                          </code>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              copyToClipboard(selectedMarketData.address);
                            }}
                            className="p-1 hover:bg-muted rounded"
                          >
                            <Copy className="h-3 w-3" />
                          </button>
                        </div>
                      </div>
                      <div>
                        <p className="text-sm text-muted-foreground mb-1">Token 0</p>
                        <div className="flex items-center gap-2">
                          <code className="text-xs bg-muted px-2 py-1 rounded flex-1 truncate">
                            {selectedMarketData.token_0}
                          </code>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              copyToClipboard(selectedMarketData.token_0);
                            }}
                            className="p-1 hover:bg-muted rounded"
                          >
                            <Copy className="h-3 w-3" />
                          </button>
                        </div>
                      </div>
                      <div>
                        <p className="text-sm text-muted-foreground mb-1">Token 1</p>
                        <div className="flex items-center gap-2">
                          <code className="text-xs bg-muted px-2 py-1 rounded flex-1 truncate">
                            {selectedMarketData.token_1}
                          </code>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              copyToClipboard(selectedMarketData.token_1);
                            }}
                            className="p-1 hover:bg-muted rounded"
                          >
                            <Copy className="h-3 w-3" />
                          </button>
                        </div>
                      </div>
                    </div>
                    <div className="space-y-3">
                      <div>
                        <p className="text-sm text-muted-foreground mb-1">Current Tick</p>
                        <p className="font-mono text-lg">{selectedMarketData.current_tick}</p>
                      </div>
                      <div>
                        <p className="text-sm text-muted-foreground mb-1">Total Liquidity</p>
                        <p className="font-mono text-lg flex items-center">
                          <DollarSign className="h-4 w-4 mr-1" />
                          {selectedMarketData.liquidity}
                        </p>
                      </div>
                      <div>
                        <p className="text-sm text-muted-foreground mb-1">Last Updated</p>
                        <p className="text-sm">
                          {new Date(selectedMarketData.last_updated_timestamp * 1000).toLocaleString()}
                        </p>
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </TabsContent>
            
            <TabsContent value="list">
              <div className="space-y-3">
                {markets.map((market, index) => (
                  <div
                    key={market.address}
                    onClick={() => setSelectedMarket(market.address)}
                    className={`p-4 rounded-lg border transition-all cursor-pointer ${
                      selectedMarket === market.address
                        ? 'border-primary bg-primary/5'
                        : 'border-border hover:border-primary/50'
                    }`}
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex-1">
                        <div className="flex items-center gap-3 mb-2">
                          <h3 className="font-medium">Market #{index + 1}</h3>
                          <Badge variant={market.is_paused ? "destructive" : "default"} className={`text-xs ${!market.is_paused && 'bg-primary/10 text-primary border-primary/20'}`}>
                            {market.is_paused ? 'Paused' : 'Active'}
                          </Badge>
                          <Badge variant="outline" className="text-xs">
                            {market.phase}
                          </Badge>
                        </div>
                        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                          <div>
                            <span className="text-muted-foreground">Address:</span>
                            <p className="font-mono text-xs">{market.address.slice(0, 8)}...</p>
                          </div>
                          <div>
                            <span className="text-muted-foreground">Tokens:</span>
                            <p className="font-mono text-xs">
                              {market.token_0.slice(0, 6)}.../...{market.token_1.slice(-6)}
                            </p>
                          </div>
                          <div>
                            <span className="text-muted-foreground">Fee:</span>
                            <p>{market.fee_bps / 100}%</p>
                          </div>
                          <div>
                            <span className="text-muted-foreground">Liquidity:</span>
                            <p className="font-mono">{market.liquidity}</p>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </TabsContent>
          </Tabs>
        )}
      </CardContent>
    </Card>
  );
}