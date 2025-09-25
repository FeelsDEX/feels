// Displays the interactive token price chart with simulated data and overlay controls.
'use client';

import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { PriceChartToolbar } from '@/components/trading/PriceChartToolbar';
import { useSimulatedKlineData } from '@/components/trading/useSimulatedKlineData';
import { useChartAdapter } from '@/components/trading/useChartAdapter';
import { useChartIndicators } from '@/components/trading/useChartIndicators';
import { KLineData } from '@/types/trading';

interface PriceChartProps {
  tokenAddress: string;
  tokenSymbol?: string;
  tokenImage?: string | any;
  isFeelsToken?: boolean;
  onPriceDataUpdate?: (data: {
    currentPrice: number;
    currentFloor: number;
    currentGtwap: number;
    allPriceData: KLineData[];
  }) => void;
}

const USD_RATE = 0.004; // Mock conversion rate for USD display

// Available timezone options with GMT offsets
const TIMEZONE_OPTIONS = [
  { value: 'Pacific/Midway', label: 'Midway', offset: -11 },
  { value: 'Pacific/Honolulu', label: 'Honolulu', offset: -10 },
  { value: 'America/Anchorage', label: 'Anchorage', offset: -9 },
  { value: 'America/Los_Angeles', label: 'Los Angeles', offset: -8 },
  { value: 'America/Denver', label: 'Denver', offset: -7 },
  { value: 'America/Chicago', label: 'Chicago', offset: -6 },
  { value: 'America/New_York', label: 'New York', offset: -5 },
  { value: 'America/Caracas', label: 'Caracas', offset: -4 },
  { value: 'America/Sao_Paulo', label: 'São Paulo', offset: -3 },
  { value: 'Atlantic/South_Georgia', label: 'South Georgia', offset: -2 },
  { value: 'Atlantic/Cape_Verde', label: 'Cape Verde', offset: -1 },
  { value: 'UTC', label: 'London', offset: 0 },
  { value: 'Europe/Paris', label: 'Paris', offset: 1 },
  { value: 'Europe/Athens', label: 'Athens', offset: 2 },
  { value: 'Europe/Moscow', label: 'Moscow', offset: 3 },
  { value: 'Asia/Dubai', label: 'Dubai', offset: 4 },
  { value: 'Asia/Karachi', label: 'Karachi', offset: 5 },
  { value: 'Asia/Dhaka', label: 'Dhaka', offset: 6 },
  { value: 'Asia/Bangkok', label: 'Bangkok', offset: 7 },
  { value: 'Asia/Hong_Kong', label: 'Hong Kong', offset: 8 },
  { value: 'Asia/Tokyo', label: 'Tokyo', offset: 9 },
  { value: 'Australia/Sydney', label: 'Sydney', offset: 10 },
  { value: 'Pacific/Noumea', label: 'Nouméa', offset: 11 },
  { value: 'Pacific/Auckland', label: 'Auckland', offset: 12 }
] as const;

// ========================================
// Main Component
// ========================================

export function PriceChart({ tokenAddress, tokenSymbol = 'TOKEN', tokenImage, onPriceDataUpdate }: PriceChartProps) {
  // ========================================
  // State Management
  // ========================================
  const [timeRange, setTimeRange] = useState('1D');
  const [timezone, setTimezone] = useState(() => {
    // Auto-detect user's timezone
    try {
      return Intl.DateTimeFormat().resolvedOptions().timeZone;
    } catch {
      return 'UTC';
    }
  });
  const [showUSD, setShowUSD] = useState(false);
  const [showFloorPrice, setShowFloorPrice] = useState(true);
  const [showGTWAPPrice, setShowGTWAPPrice] = useState(true);
  const [showLastPrice, setShowLastPrice] = useState(true);
  const [showCrosshair] = useState(true);
  const [priceAxisType, setPriceAxisType] = useState<'normal' | 'logarithm' | 'percentage'>('normal');
  const [currentTime, setCurrentTime] = useState(new Date());
  const [chartContainer, setChartContainer] = useState<HTMLDivElement | null>(null);

  // Update current time every second for live timezone display
  useEffect(() => {
    const timer = setInterval(() => setCurrentTime(new Date()), 1000);
    return () => clearInterval(timer);
  }, []);

  // ========================================
  // Data Fetching
  // ========================================
  const { loading, error, klineData, floorPrice, gtwapPrice, floorSeries, gtwapSeries } = useSimulatedKlineData({
    tokenAddress,
    timeRange
  });

  // Notify parent component of price updates
  useEffect(() => {
    if (!onPriceDataUpdate || klineData.length === 0) return;
    const lastCandle = klineData[klineData.length - 1];
    if (!lastCandle) return;
    
      onPriceDataUpdate({
      currentPrice: showUSD ? lastCandle.close * USD_RATE : lastCandle.close,
      currentFloor: showUSD ? floorPrice * USD_RATE : floorPrice,
      currentGtwap: showUSD ? gtwapPrice * USD_RATE : gtwapPrice,
      allPriceData: klineData
    });
  }, [floorPrice, gtwapPrice, klineData, onPriceDataUpdate, showUSD]);

  // ========================================
  // Data Transformations
  // ========================================

  // Convert price data to USD if needed
  const priceData = useMemo(() => {
    if (!showUSD) return klineData;
    return klineData.map((candle) => ({
      ...candle,
      open: candle.open * USD_RATE,
      high: candle.high * USD_RATE,
      low: candle.low * USD_RATE,
      close: candle.close * USD_RATE,
      volume: candle.volume * USD_RATE,
      turnover: candle.turnover * USD_RATE
    }));
  }, [klineData, showUSD]);

  // ========================================
  // Formatting Functions
  // ========================================

  // Format timestamps for chart display based on time range
  const formatDate = useCallback(
    (timestamp: number, _format: string, type: number) => {
          const date = new Date(timestamp);
          if (type === 2) { // X-axis labels
        // Short timeframes: show time with seconds
        if (['1m', '5m', '15m', '30m'].includes(timeRange)) {
              return date.toLocaleTimeString('en-US', {
                hour: '2-digit',
                minute: '2-digit',
                second: '2-digit',
                hour12: false,
                timeZone: timezone
              });
        }
        // Medium timeframes: show time without seconds
        if (['1h', '6h', '12h'].includes(timeRange)) {
              return date.toLocaleTimeString('en-US', {
                hour: '2-digit',
                minute: '2-digit',
                hour12: false,
                timeZone: timezone
              });
        }
        // Daily timeframes: show date with hour
        if (['1D', '3D'].includes(timeRange)) {
              return date.toLocaleDateString('en-US', {
                month: 'short',
                day: 'numeric',
                hour: '2-digit',
                hour12: false,
                timeZone: timezone
              });
        }
        // Long timeframes: show date only
              return date.toLocaleDateString('en-US', {
                month: 'short',
                day: 'numeric',
                timeZone: timezone
              });
            }
            // Tooltip format: full date and time
          return date.toLocaleString('en-US', {
            timeZone: timezone,
            dateStyle: 'medium',
            timeStyle: 'medium'
          });
        },
    [timeRange, timezone]
  );

  // Format numbers for chart display (prices, volumes, etc.)
  const formatBigNumber = useCallback(
    (value: string | number) => {
      // Convert string to number if needed
      const numValue = typeof value === 'string' ? parseFloat(value) : value;
      // Ensure value is a valid number
      if (typeof numValue !== 'number' || isNaN(numValue)) {
        return '0';
      }
      
      // Handle percentage axis
      if (priceAxisType === 'percentage') {
        return `${numValue.toFixed(2)}%`;
      }
      
      // Handle USD display
      if (showUSD) {
        return `$${numValue.toFixed(2)}`;
      }
      
      // Format based on magnitude for readability
      const abs = Math.abs(numValue);
      if (abs === 0) return '0';
      if (abs < 0.000001) return numValue.toExponential(2); // Very small numbers
      if (abs < 0.00001) return numValue.toFixed(6);
      if (abs < 0.0001) return numValue.toFixed(5);
      if (abs < 0.001) return numValue.toFixed(4);
      if (abs < 0.01) return numValue.toFixed(4);
      if (abs < 0.1) return numValue.toFixed(3);
      if (abs < 1) return numValue.toFixed(3);
      if (abs < 10) return numValue % 1 === 0 ? numValue.toFixed(0) : numValue.toFixed(2);
      if (abs < 100) return numValue % 1 === 0 ? numValue.toFixed(0) : numValue.toFixed(1);
      if (abs < 1000) return numValue.toFixed(0);
      if (abs >= 1_000_000) return `${(numValue / 1_000_000).toFixed(1)}M`; // Millions
      if (abs >= 1_000) return `${(numValue / 1_000).toFixed(1)}K`; // Thousands
      return numValue.toFixed(0);
    },
    [priceAxisType, showUSD]
  );

  // ========================================
  // Chart Integration
  // ========================================

  // Initialize chart adapter with formatting functions
  const { chart, isReady, applyData, applyTimezone, applyAxisType, createLineOverlay, setLastPriceVisibility, setCrosshairVisibility } = useChartAdapter({
    container: chartContainer,
    timezone,
    priceAxisType,
    formatDate,
    formatBigNumber
  });
  
  // Initialize chart indicators
  const chartRef = React.useRef(chart);
  React.useEffect(() => {
    chartRef.current = chart;
  }, [chart]);
  
  const { createFloorIndicator, removeFloorIndicator, createGTWAPIndicator, removeGTWAPIndicator } = useChartIndicators(chartRef);

  // ========================================
  // Chart Configuration Effects
  // ========================================

  // Apply timezone changes to chart
  useEffect(() => {
    if (!isReady) return;
    applyTimezone(timezone);
  }, [applyTimezone, isReady, timezone]);

  // Apply price axis type changes
  useEffect(() => {
    if (!isReady) return;
    applyAxisType(priceAxisType);
  }, [applyAxisType, isReady, priceAxisType]);

  // Toggle last price line visibility
  useEffect(() => {
    if (!isReady) return;
    setLastPriceVisibility(showLastPrice);
  }, [isReady, setLastPriceVisibility, showLastPrice]);

  // Toggle crosshair visibility
  useEffect(() => {
    if (!isReady) return;
    setCrosshairVisibility(showCrosshair);
  }, [isReady, setCrosshairVisibility, showCrosshair]);

  // Update chart with new price data
  useEffect(() => {
    if (!isReady || priceData.length === 0) return;
    applyData(priceData);
  }, [applyData, isReady, priceData]);

  // ========================================
  // Overlay Management
  // ========================================

  // Manage floor price indicator
  useEffect(() => {
    console.log(`[PriceChart] Floor indicator effect triggered - ready: ${isReady}, show: ${showFloorPrice}, points: ${floorSeries.length}`);
    if (!isReady) return;
    
    if (showFloorPrice && floorSeries.length > 0) {
      createFloorIndicator(
        floorSeries.map((point) => ({
          timestamp: point.timestamp,
          value: showUSD ? point.value * USD_RATE : point.value
        }))
      );
    } else {
      removeFloorIndicator();
    }
  }, [createFloorIndicator, removeFloorIndicator, floorSeries, isReady, showFloorPrice, showUSD]);

  // Manage GTWAP indicator
  useEffect(() => {
    console.log(`[PriceChart] GTWAP indicator effect triggered - ready: ${isReady}, show: ${showGTWAPPrice}, points: ${gtwapSeries.length}`);
    if (!isReady) return;
    
    if (showGTWAPPrice && gtwapSeries.length > 0) {
      createGTWAPIndicator(
        gtwapSeries.map((point) => ({
          timestamp: point.timestamp,
          value: showUSD ? point.value * USD_RATE : point.value
        }))
      );
    } else {
      removeGTWAPIndicator();
    }
  }, [createGTWAPIndicator, removeGTWAPIndicator, gtwapSeries, isReady, showGTWAPPrice, showUSD]);

  // ========================================
  // Price Change Calculation
  // ========================================

  // Calculate price change percentage for display
  const priceChange = useMemo(() => {
    if (priceData.length < 2) return null;
    const first = priceData[0]?.close;
    const last = priceData[priceData.length - 1]?.close;
    if (!first || !last) return null;
    const change = last - first;
          return {
      value: change,
      percent: (change / first) * 100,
      isPositive: change >= 0
    };
  }, [priceData]);

  // ========================================
  // Render Component
  // ========================================

  return (
    <Card className="w-full">
      <CardHeader className="pb-3">
        <div className="flex items-start justify-between">
          {/* Token info and price change */}
          <div className="flex items-center gap-3">
            {/* Token avatar */}
            <div className="w-10 h-10 rounded-md bg-muted flex items-center justify-center overflow-hidden">
              {tokenImage ? (
                <img 
                  src={typeof tokenImage === 'string' ? tokenImage : tokenImage.src || tokenImage}
                  alt={tokenSymbol}
                  className="w-full h-full object-cover"
                  onError={(e) => {
                    (e.target as HTMLImageElement).style.display = 'none';
                    (e.target as HTMLImageElement).parentElement!.innerHTML = `<span class="text-sm font-medium">${tokenSymbol?.[0] || 'T'}</span>`;
                  }}
                />
              ) : (
                <span className="text-sm font-medium">{tokenSymbol?.[0] || 'T'}</span>
              )}
            </div>
            {/* Token symbol and price change */}
            <CardTitle className="text-xl font-medium flex items-center gap-2">
              {tokenSymbol}/FeelsSOL
              {priceChange && (
                <span className={`text-sm font-normal ${priceChange.isPositive ? 'text-[#5cca39]' : 'text-red-600'}`}>
                  {priceChange.isPositive ? '+' : ''}{priceChange.percent.toFixed(2)}%
                </span>
              )}
            </CardTitle>
          </div>
          {/* Chart controls toolbar */}
          <PriceChartToolbar
            timeRange={timeRange}
            onTimeRangeChange={setTimeRange}
            timezone={timezone}
            onTimezoneChange={setTimezone}
            currentTime={currentTime}
            timezones={[...TIMEZONE_OPTIONS]}
            showUSD={showUSD}
            onToggleUSD={() => setShowUSD((prev) => !prev)}
            showFloorPrice={showFloorPrice}
            onToggleFloor={() => setShowFloorPrice((prev) => !prev)}
            showGTWAPPrice={showGTWAPPrice}
            onToggleGTWAP={() => setShowGTWAPPrice((prev) => !prev)}
            showLastPrice={showLastPrice}
            onToggleLastPrice={() => setShowLastPrice((prev) => !prev)}
            priceAxisType={priceAxisType}
            onPriceAxisTypeChange={setPriceAxisType}
          />
                </div>
      </CardHeader>
      <CardContent className="pl-8 pr-2 pt-2 pb-4">
        {/* Error display */}
        {error && (
          <div className="mb-4 rounded border border-red-300 bg-red-50 px-3 py-2 text-sm text-red-700">
            {error}
                </div>
        )}
        
        {/* Chart container */}
        <div id="chart-canvas-container" className="w-full relative overflow-visible">
          <div 
            ref={setChartContainer}
            id="kline-chart"
            className="klinechart-container"
            style={{ 
              width: '100%', 
              height: '500px',
              backgroundColor: '#ffffff',
              opacity: isReady ? 1 : 0, // Fade in when ready
              transition: 'opacity 0.2s ease-in-out',
              position: 'relative'
            }} 
          />
          {/* Loading spinner overlay */}
          {(loading || !isReady) && (
            <div id="chart-loading-spinner" className="absolute inset-0 flex items-center justify-center bg-background">
              <div className="animate-spin h-8 w-8 rounded-full border-2 border-muted border-t-primary"></div>
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
