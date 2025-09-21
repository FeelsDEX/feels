'use client';

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { BarChart3 } from 'lucide-react';

export interface TokenMetricsProps {
  currentPrice: number;
  currentFloor: number;
  currentGtwap: number;
  allPriceData: Array<{
    timestamp: number;
    open: number;
    high: number;
    low: number;
    close: number;
    volume?: number;
    floor: number;
    gtwap: number;
  }>;
  tokenCreator?: string;
  tokenAddress?: string;
}

export function TokenMetrics({
  currentPrice,
  currentFloor,
  currentGtwap,
  allPriceData
}: TokenMetricsProps) {
  // Calculate 24h metrics
  const last96Candles = allPriceData.slice(-96); // 96 * 15min = 24h
  const dayAgoIndex = Math.max(0, allPriceData.length - 96);
  
  const high24h = last96Candles.length > 0 
    ? Math.max(...last96Candles.map(d => d.high))
    : currentPrice;
  
  const low24h = last96Candles.length > 0
    ? Math.min(...last96Candles.map(d => d.low))
    : currentPrice;
  
  const volume24h = last96Candles.reduce((sum, d) => sum + (d.volume || 0), 0);
  
  const lastDataPoint = allPriceData[allPriceData.length - 1];
  const dayAgoDataPoint = allPriceData[dayAgoIndex];
  
  const floorChange24h = allPriceData.length > dayAgoIndex && 
    dayAgoDataPoint && 
    lastDataPoint &&
    dayAgoDataPoint.floor > 0
    ? ((lastDataPoint.floor - dayAgoDataPoint.floor) / dayAgoDataPoint.floor) * 100
    : 0;
  
  const marketCap = currentPrice * 1000000 * 2.5; // Assuming 1M supply * 2.5 multiplier
  const floorGtwapRatio = currentGtwap > 0 ? (currentFloor / currentGtwap) * 100 : 0;

  return (
    <Card id="token-metrics-container" className="w-full">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <BarChart3 className="h-5 w-5" />
          Token Metrics
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div id="token-metrics-grid" className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-4">
        <div id="metric-24h-high">
          <p className="text-xs text-muted-foreground">24h High</p>
          <p className="text-sm font-semibold">
            {high24h.toFixed(4)}
          </p>
        </div>
        <div id="metric-24h-low">
          <p className="text-xs text-muted-foreground">24h Low</p>
          <p className="text-sm font-semibold">
            {low24h.toFixed(4)}
          </p>
        </div>
        <div id="metric-24h-volume">
          <p className="text-xs text-muted-foreground">Volume 24h</p>
          <p className="text-sm font-semibold">
            {volume24h.toLocaleString()} FeelsSOL
          </p>
        </div>
        <div id="metric-current-floor">
          <p className="text-xs text-muted-foreground">Current Floor</p>
          <p className="text-sm font-semibold">
            {currentFloor.toFixed(4)}
          </p>
        </div>
        <div id="metric-floor-delta-24h">
          <p className="text-xs text-muted-foreground">Floor Δ 24h</p>
          <p className="text-sm font-semibold">
            <span className={floorChange24h >= 0 ? 'text-primary' : 'text-red-500'}>
              {floorChange24h >= 0 ? '+' : ''}{floorChange24h.toFixed(2)}%
            </span>
          </p>
        </div>
        <div id="metric-market-cap">
          <p className="text-xs text-muted-foreground">Market Cap</p>
          <p className="text-sm font-semibold">
            ${marketCap.toFixed(0).toLocaleString()}
          </p>
        </div>
        <div id="metric-gtwap">
          <p className="text-xs text-muted-foreground">GTWAP</p>
          <p className="text-sm font-semibold">
            {currentGtwap.toFixed(4)}
          </p>
        </div>
        <div id="metric-floor-gtwap-ratio">
          <p className="text-xs text-muted-foreground">Floor/GTWAP</p>
          <p className="text-sm font-semibold">
            {floorGtwapRatio.toFixed(1)}%
          </p>
        </div>
        </div>
      </CardContent>
    </Card>
  );
}