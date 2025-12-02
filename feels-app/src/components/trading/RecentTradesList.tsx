'use client';

import { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Activity } from 'lucide-react';
import { useProtocolStats } from '@/hooks/useIndexer';
import Link from 'next/link';

interface Trade {
  id: string;
  timestamp: Date;
  type: 'buy' | 'sell';
  usdAmount: number;
  feelsAmount: number;
  marketCap: number;
  account: string;
  accountFull: string; // Full account address for linking
  tokenSymbol: string;
}

interface RecentTradesListProps {
  tokenSymbol?: string;
  tokenAddress?: string;
}

export function RecentTradesList({ tokenSymbol }: RecentTradesListProps) {
  const [trades, setTrades] = useState<Trade[]>([]);
  useProtocolStats({ refreshInterval: 5000 });

  // Generate mock trades data
  useEffect(() => {
    // In production, this would fetch from indexer API
    const mockTrades: Trade[] = [];
    const now = new Date();
    
    // Generate 20 mock trades
    for (let i = 0; i < 20; i++) {
      const isBuy = Math.random() > 0.5;
      const feelsAmount = Math.random() * 10 + 0.1; // 0.1 to 10 SOL
      const usdAmount = feelsAmount * 50; // $50 per SOL
      
      // Generate a mock Solana address (base58 format)
      const accountPrefix = ['Gm1z', 'Ape9', 'Dgen', 'Wojk', 'Chad', 'Anon'][Math.floor(Math.random() * 6)];
      const accountSuffix = Math.random().toString(36).substring(2, 10).toUpperCase();
      const accountMiddle = Math.random().toString(36).substring(2, 26).toUpperCase();
      const fullAccount = `${accountPrefix}${accountMiddle}${accountSuffix}`;
      
      mockTrades.push({
        id: `trade-${i}`,
        timestamp: new Date(now.getTime() - i * 60000 - Math.random() * 30000), // Random times in past hour
        type: isBuy ? 'buy' : 'sell',
        usdAmount,
        feelsAmount,
        marketCap: 420000 + Math.random() * 100000, // $420K - $520K market cap
        account: `${accountPrefix}...${accountSuffix.substring(0, 4)}`,
        accountFull: fullAccount,
        tokenSymbol: tokenSymbol || 'WOJAK'
      });
    }
    
    // Sort by timestamp descending
    mockTrades.sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime());
    setTrades(mockTrades);
  }, [tokenSymbol]);

  // Format time ago
  const formatTimeAgo = (date: Date) => {
    const seconds = Math.floor((new Date().getTime() - date.getTime()) / 1000);
    
    if (seconds < 60) return `${seconds}s ago`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  };

  // Format USD amount
  const formatUSD = (amount: number) => {
    if (amount >= 1000) {
      return `$${(amount / 1000).toFixed(1)}K`;
    }
    return `$${amount.toFixed(2)}`;
  };

  // Format market cap
  const formatMarketCap = (mcap: number) => {
    if (mcap >= 1000000) {
      return `$${(mcap / 1000000).toFixed(2)}M`;
    }
    return `$${(mcap / 1000).toFixed(0)}K`;
  };

  return (
    <Card className="w-full">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Activity className="h-5 w-5" />
          Recent Trades
        </CardTitle>
      </CardHeader>
      <CardContent>
        {/* Table Header */}
        <div className="grid grid-cols-6 gap-2 pb-2 border-b text-xs font-medium text-muted-foreground">
          <div>Time</div>
          <div>Direction</div>
          <div className="text-right">USD</div>
          <div className="text-right">SOL</div>
          <div className="text-right">Mcap</div>
          <div className="text-right">Account</div>
        </div>
        
        {/* Trades List */}
        <div className="space-y-1 mt-2 max-h-[400px] overflow-y-auto">
          {trades.map((trade) => (
            <div 
              key={trade.id} 
              className="grid grid-cols-6 gap-2 py-2 hover:bg-muted/30 rounded px-1 transition-colors"
            >
              {/* Time */}
              <div className="text-sm text-muted-foreground">
                {formatTimeAgo(trade.timestamp)}
              </div>
              
              {/* Direction */}
              <div className="flex items-center h-full">
                <Badge 
                  variant={trade.type === 'buy' ? 'default' : 'destructive'} 
                  className="px-2 py-0 h-5 w-fit"
                  style={{ fontSize: '11px' }}
                >
                  {trade.type}
                </Badge>
              </div>
              
              {/* USD Amount */}
              <div className="text-sm font-medium text-right">
                {formatUSD(trade.usdAmount)}
              </div>
              
              {/* SOL Amount */}
              <div className="text-sm text-right">
                {trade.feelsAmount.toFixed(2)}
              </div>
              
              {/* Market Cap */}
              <div className="text-sm text-right text-muted-foreground">
                {formatMarketCap(trade.marketCap)}
              </div>
              
              {/* Account */}
              <div className="text-sm text-right font-mono">
                <Link
                  href={`/account/${trade.accountFull}`}
                  className="text-muted-foreground hover:text-primary transition-colors"
                >
                  {trade.account}
                </Link>
              </div>
            </div>
          ))}
        </div>
        
        {/* Summary Stats */}
        <div className="mt-4 pt-4 border-t grid grid-cols-3 gap-4 text-sm">
          <div>
            <span className="text-muted-foreground">Total Volume:</span>
            <div className="font-medium">
              {formatUSD(trades.reduce((sum, t) => sum + t.usdAmount, 0))}
            </div>
          </div>
          <div>
            <span className="text-muted-foreground">Buy/Sell Ratio:</span>
            <div className="font-medium">
              {trades.filter(t => t.type === 'buy').length} / {trades.filter(t => t.type === 'sell').length}
            </div>
          </div>
          <div>
            <span className="text-muted-foreground">Avg Trade Size:</span>
            <div className="font-medium">
              {formatUSD(trades.reduce((sum, t) => sum + t.usdAmount, 0) / trades.length)}
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}