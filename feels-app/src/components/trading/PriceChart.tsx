// Displays the interactive token price chart with integrated metrics and configuration controls.
'use client';

import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { PriceChartToolbar } from '@/components/trading/PriceChartToolbar';
import { useSimulatedKlineData } from '@/components/trading/useSimulatedKlineData';
import { useChartAdapter } from '@/hooks/useChartAdapter';
import { PLOT_BACKGROUND } from '@/hooks/chart-config';

// ============================================================================
// CONSTANTS
// ============================================================================

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
  { value: 'Pacific/Auckland', label: 'Auckland', offset: 12 },
] as const;

// ============================================================================
// TYPE DEFINITIONS
// ============================================================================

interface PriceChartProps {
  tokenAddress: string;
  tokenSymbol?: string;
  tokenImage?: string | any;
  isFeelsToken?: boolean;
  tokenCreator?: string;
  isGraduated?: boolean;
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/**
 * Formats large numbers with K/M/B suffixes
 */
function formatCompactNumber(value: number, showUSD: boolean): string {
  const prefix = showUSD ? '$' : '';
  
  if (value >= 1_000_000_000) {
    // Billions
    return `${prefix}${(value / 1_000_000_000).toFixed(2)}B`;
  } else if (value >= 1_000_000) {
    // Millions - show 1 decimal if < 10M, otherwise no decimals
    const millions = value / 1_000_000;
    return `${prefix}${millions >= 10 ? millions.toFixed(0) : millions.toFixed(1)}M`;
  } else if (value >= 1_000) {
    // Thousands - show 1 decimal if < 10K, otherwise no decimals
    const thousands = value / 1_000;
    return `${prefix}${thousands >= 10 ? thousands.toFixed(0) : thousands.toFixed(1)}K`;
  } else {
    // Less than 1K - show full number
    return `${prefix}${value.toFixed(0)}`;
  }
}

// ============================================================================
// MAIN COMPONENT
// ============================================================================

export function PriceChart({
  tokenAddress,
  tokenSymbol = 'TOKEN',
  tokenImage,
  isGraduated = true,
}: PriceChartProps) {
  // --------------------------------------------------------------------------
  // State Management
  // --------------------------------------------------------------------------
  
  // Chart display settings
  const [timeRange, setTimeRange] = useState('1D');
  const [priceAxisType, setPriceAxisType] = useState<'normal' | 'logarithm' | 'percentage'>('normal');
  const [showUSD, setShowUSD] = useState(false);
  
  // Overlay visibility
  const [showFloorPrice, setShowFloorPrice] = useState(true);
  const [showGTWAPPrice, setShowGTWAPPrice] = useState(true);
  const [showLastPrice, setShowLastPrice] = useState(true);
  const [showCrosshair] = useState(true);
  
  // Timezone and time display
  const [timezone, setTimezone] = useState(() => {
    // Auto-detect user's timezone
    try {
      return Intl.DateTimeFormat().resolvedOptions().timeZone;
    } catch {
      return 'UTC';
    }
  });
  const [currentTime, setCurrentTime] = useState(new Date());
  
  // Chart DOM reference
  const [chartContainer, setChartContainer] = useState<HTMLDivElement | null>(null);

  // --------------------------------------------------------------------------
  // Data Fetching
  // --------------------------------------------------------------------------
  
  const { loading, error, klineData, floorPrice, gtwapPrice, floorSeries, gtwapSeries } =
    useSimulatedKlineData({
      tokenAddress,
      timeRange,
    });

  // --------------------------------------------------------------------------
  // USD Conversion Calculations
  // --------------------------------------------------------------------------
  
  const usdConversionFactor = useMemo(() => {
    if (klineData.length === 0) return 1;
    const lastClose = klineData[klineData.length - 1]?.close || 0;
    if (lastClose <= 0) return 1;
    // Normalise prices so that USD view sits around the hundreds range for visibility
    const desiredMagnitude = 2; // 10^2 ~= 100 USD
    const magnitude = Math.log10(lastClose);
    return Math.pow(10, desiredMagnitude - magnitude);
  }, [klineData]);

  const activeConversion = showUSD ? usdConversionFactor : 1;

  // --------------------------------------------------------------------------
  // Formatting Functions
  // --------------------------------------------------------------------------

  /**
   * Formats timestamps for chart display based on time range
   */
  const formatDate = useCallback(
    (timestamp: number, _format: string, type: number) => {
      const date = new Date(timestamp);
      if (type === 2) {
        // X-axis labels
        // Short timeframes: show time with seconds
        if (['1m', '5m', '15m', '30m'].includes(timeRange)) {
          return date.toLocaleTimeString('en-US', {
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit',
            hour12: false,
            timeZone: timezone,
          });
        }
        // Medium timeframes: show time without seconds
        if (['1h', '6h', '12h'].includes(timeRange)) {
          return date.toLocaleTimeString('en-US', {
            hour: '2-digit',
            minute: '2-digit',
            hour12: false,
            timeZone: timezone,
          });
        }
        // Daily timeframes: show date with hour
        if (['1D', '3D'].includes(timeRange)) {
          return date.toLocaleDateString('en-US', {
            month: 'short',
            day: 'numeric',
            hour: '2-digit',
            hour12: false,
            timeZone: timezone,
          });
        }
        // Long timeframes: show date only
        return date.toLocaleDateString('en-US', {
          month: 'short',
          day: 'numeric',
          timeZone: timezone,
        });
      }
      // Tooltip format: full date and time
      return date.toLocaleString('en-US', {
        timeZone: timezone,
        dateStyle: 'medium',
        timeStyle: 'medium',
      });
    },
    [timeRange, timezone]
  );

  /**
   * Formats numbers for chart display (prices, volumes, etc.)
   */
  const formatBigNumber = useCallback(
    (value: string | number) => {
      const rawValue = typeof value === 'string' ? parseFloat(value) : value;
      if (typeof rawValue !== 'number' || Number.isNaN(rawValue)) {
        return '0';
      }

      if (priceAxisType === 'percentage') {
        return `${rawValue.toFixed(2)}%`;
      }

      // For logarithmic and percentage modes when USD is enabled,
      // the value is the raw token value, so we need to convert for display
      const shouldApplyConversion = showUSD;
      const displayValue = shouldApplyConversion ? rawValue * activeConversion : rawValue;

      if (showUSD) {
        // For logarithmic axis with USD, format the converted value
        if (priceAxisType === 'logarithm') {
          return `$${displayValue.toFixed(2)}`;
        }
        return `$${displayValue.toFixed(2)}`;
      }

      const abs = Math.abs(displayValue);
      if (abs === 0) return '0';
      if (abs < 0.000001) return displayValue.toExponential(2);
      if (abs < 0.00001) return displayValue.toFixed(6);
      if (abs < 0.0001) return displayValue.toFixed(5);
      if (abs < 0.001) return displayValue.toFixed(4);
      if (abs < 0.01) return displayValue.toFixed(4);
      if (abs < 0.1) return displayValue.toFixed(3);
      if (abs < 1) return displayValue.toFixed(3);
      if (abs < 10)
        return displayValue % 1 === 0 ? displayValue.toFixed(0) : displayValue.toFixed(2);
      if (abs < 100)
        return displayValue % 1 === 0 ? displayValue.toFixed(0) : displayValue.toFixed(1);
      if (abs < 1000) return displayValue.toFixed(0);
      if (abs >= 1_000_000) return `${(displayValue / 1_000_000).toFixed(1)}M`;
      if (abs >= 1_000) return `${(displayValue / 1_000).toFixed(1)}K`;
      return displayValue.toFixed(0);
    },
    [activeConversion, priceAxisType, showUSD]
  );

  // --------------------------------------------------------------------------
  // Chart Integration
  // --------------------------------------------------------------------------

  // Initialize chart adapter
  const {
    // chart,
    isReady,
    applyChartData,
    applyTimezone,
    applyAxisType,
    setLastPriceVisibility,
    setCrosshairVisibility,
    setFloorVisibility,
    setGtwapVisibility,
    resetVisibleRange,
  } = useChartAdapter({
    container: chartContainer,
    timezone,
    priceAxisType,
    formatDate,
    formatBigNumber,
    showUSD,
    usdConversionFactor,
  });

  // Note: Chart indicators are now managed declaratively through applyChartData

  // --------------------------------------------------------------------------
  // Effects - Time Updates
  // --------------------------------------------------------------------------

  // Update current time every second for live timezone display
  useEffect(() => {
    const timer = setInterval(() => setCurrentTime(new Date()), 1000);
    return () => clearInterval(timer);
  }, []);

  // --------------------------------------------------------------------------
  // Effects - Chart Configuration
  // --------------------------------------------------------------------------

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
    if (!isReady || klineData.length === 0) return;

    // Only transform for normal axis type with USD display
    const shouldTransformData = priceAxisType === 'normal' && showUSD;
    
    const dataToApply = shouldTransformData
      ? klineData.map((candle) => ({
          ...candle,
          open: candle.open * activeConversion,
          high: candle.high * activeConversion,
          low: candle.low * activeConversion,
          close: candle.close * activeConversion,
          turnover: candle.turnover * activeConversion,
        }))
      : klineData;

    // Apply data and indicators atomically
    applyChartData({
      data: dataToApply,
      floor: {
        visible: showFloorPrice,
        series: floorSeries.map((point) => ({
          timestamp: point.timestamp,
          value: shouldTransformData ? point.value * activeConversion : point.value,
        })),
      },
      gtwap: {
        visible: showGTWAPPrice,
        series: gtwapSeries.map((point) => ({
          timestamp: point.timestamp,
          value: shouldTransformData ? point.value * activeConversion : point.value,
        })),
      },
    });

    // If "All" view is selected, reset the visible range to show all data
    if (timeRange === 'all') {
      setTimeout(() => resetVisibleRange(), 100);
    }
  }, [
    applyChartData,
    isReady,
    klineData,
    activeConversion,
    priceAxisType,
    showUSD,
    timeRange,
    resetVisibleRange,
    showFloorPrice,
    showGTWAPPrice,
    floorSeries,
    gtwapSeries,
  ]);

  // --------------------------------------------------------------------------
  // Effects - Overlay Management
  // --------------------------------------------------------------------------

  // Sync floor indicator visibility
  useEffect(() => {
    if (!isReady) return;
    setFloorVisibility(showFloorPrice);
  }, [isReady, showFloorPrice, setFloorVisibility]);

  // Sync GTWAP indicator visibility
  useEffect(() => {
    if (!isReady) return;
    setGtwapVisibility(showGTWAPPrice);
  }, [isReady, showGTWAPPrice, setGtwapVisibility]);

  // --------------------------------------------------------------------------
  // Metrics Calculations
  // --------------------------------------------------------------------------

  /**
   * Calculates price change percentage for display
   */
  const priceChange = useMemo(() => {
    if (klineData.length < 2) return null;
    const first = klineData[0]?.close;
    const last = klineData[klineData.length - 1]?.close;
    if (!first || !last) return null;
    const change = last - first;
    return {
      value: change * activeConversion,
      percent: (change / first) * 100,
      isPositive: change >= 0,
    };
  }, [activeConversion, klineData]);

  /**
   * Calculates 24h metrics from kline data
   */
  const metrics = useMemo(() => {
    if (klineData.length === 0) {
      return {
        high24h: 0,
        low24h: 0,
        volume24h: 0,
        floorChange24h: 0,
        marketCap: 0,
        floorGtwapRatio: 0,
      };
    }

    const last96Candles = klineData.slice(-96); // 96 * 15min = 24h
    const dayAgoIndex = Math.max(0, klineData.length - 96);
    const lastCandle = klineData[klineData.length - 1];
    const currentPrice = lastCandle?.close || 0;

    const high24h =
      last96Candles.length > 0
        ? Math.max(...last96Candles.map((d) => d.high))
        : currentPrice;

    const low24h =
      last96Candles.length > 0
        ? Math.min(...last96Candles.map((d) => d.low))
        : currentPrice;

    const volume24h = last96Candles.reduce((sum, d) => sum + (d.volume || 0), 0);

    const dayAgoFloor = floorSeries[dayAgoIndex]?.value || floorPrice;
    const floorChange24h =
      dayAgoFloor > 0 ? ((floorPrice - dayAgoFloor) / dayAgoFloor) * 100 : 0;

    const marketCap = currentPrice * 1_000_000 * 2.5;
    const floorGtwapRatio = gtwapPrice > 0 ? (floorPrice / gtwapPrice) * 100 : 0;

    return {
      high24h,
      low24h,
      volume24h,
      floorChange24h,
      marketCap,
      floorGtwapRatio,
    };
  }, [klineData, floorPrice, gtwapPrice, floorSeries]);

  /**
   * Formats metric values with appropriate currency/number formatting
   */
  const formatMetricValue = useCallback(
    (value: number, options: Intl.NumberFormatOptions = {}) => {
      const formatter = new Intl.NumberFormat('en-US', {
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
        ...options,
      });
      const converted = showUSD ? value * activeConversion : value;
      return showUSD ? `$${formatter.format(converted)}` : formatter.format(converted);
    },
    [activeConversion, showUSD]
  );

  // --------------------------------------------------------------------------
  // Render
  // --------------------------------------------------------------------------

  return (
    <Card className="w-full">
      <CardHeader className="pb-3">
        <div className="flex items-center gap-8">
          {/* Token info and price change */}
          <div className="flex items-center gap-3 flex-shrink-0 ml-4">
            {/* Token avatar */}
            <div className="w-10 h-10 rounded-md bg-muted flex items-center justify-center overflow-hidden">
              {tokenImage ? (
                <img
                  src={typeof tokenImage === 'string' ? tokenImage : tokenImage.src || tokenImage}
                  alt={tokenSymbol}
                  className="w-full h-full object-cover"
                  onError={(e) => {
                    (e.target as HTMLImageElement).style.display = 'none';
                    (e.target as HTMLImageElement).parentElement!.innerHTML =
                      `<span class="text-sm font-medium">${tokenSymbol?.[0] || 'T'}</span>`;
                  }}
                />
              ) : (
                <span className="text-sm font-medium">{tokenSymbol?.[0] || 'T'}</span>
              )}
            </div>
            {/* Token symbol and price change */}
            <div>
              <CardTitle className="flex items-center gap-2">
                {tokenSymbol}/FeelsSOL
                <Badge 
                  variant="outline" 
                  className="text-xs px-1.5 py-0 h-5 bg-primary/10 text-primary border-primary/20"
                >
                  {isGraduated ? 'Graduated' : 'Bonding'}
                </Badge>
                {priceChange && (
                  <span
                    className={`text-base font-normal ${priceChange.isPositive ? 'text-[#5cca39]' : 'text-red-600'}`}
                  >
                    {priceChange.isPositive ? '+' : ''}
                    {priceChange.percent.toFixed(2)}%
                  </span>
                )}
              </CardTitle>
            </div>
          </div>

            {/* Token Metrics Grid */}
          <div className="flex-1 overflow-x-auto ml-14">
            <div className="grid grid-cols-6 gap-1 min-w-max">
              <div>
                <p className="text-xs text-muted-foreground">Market Cap</p>
                <p className="text-sm font-semibold">
                  {formatCompactNumber(
                    showUSD ? metrics.marketCap * activeConversion : metrics.marketCap,
                    showUSD
                  )}
                </p>
              </div>
              <div>
                <p className="text-xs text-muted-foreground">24h Volume</p>
                <p className="text-sm font-semibold">
                  {formatCompactNumber(
                    showUSD ? metrics.volume24h * activeConversion : metrics.volume24h,
                    showUSD
                  )}
                </p>
              </div>
              <div>
                <p className="text-xs text-muted-foreground">24h Range</p>
                <p className="text-sm font-semibold">
                  {formatMetricValue(metrics.low24h)} - {formatMetricValue(metrics.high24h)}
                </p>
              </div>
              <div>
                <p className="text-xs text-muted-foreground">Floor Price</p>
                <p className="text-sm font-semibold">
                  {formatMetricValue(floorPrice)}
                </p>
              </div>
              <div>
                <p className="text-xs text-muted-foreground">24hr Floor Δ</p>
                <p className="text-sm font-semibold">
                  <span className={metrics.floorChange24h >= 0 ? 'text-primary' : 'text-red-500'}>
                    {metrics.floorChange24h >= 0 ? '+' : ''}
                    {metrics.floorChange24h.toFixed(2)}%
                  </span>
                </p>
              </div>
              <div>
                <div className="flex items-end gap-1">
                  <p className="text-xs text-muted-foreground">Floor/GTWAP</p>
                </div>
                <div className="flex items-baseline gap-0.5">
                  <p className="text-sm font-semibold">{formatMetricValue(floorPrice)}</p>
                  <span className="text-xs text-muted-foreground">/</span>
                  <p className="text-sm font-semibold">{formatMetricValue(gtwapPrice)}</p>
                  <span className="text-xs font-semibold text-muted-foreground">
                    ({metrics.floorGtwapRatio.toFixed(0)}%)
                  </span>
                </div>
              </div>
            </div>
          </div>
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
              position: 'relative',
            }}
          />
          {/* Loading spinner overlay */}
          {(loading || !isReady) && (
            <div
              id="chart-loading-spinner"
              className="absolute inset-0 flex items-center justify-center bg-background"
            >
              <div className="animate-spin h-8 w-8 rounded-full border-2 border-muted border-t-primary"></div>
            </div>
          )}
        </div>

        {/* Chart Configuration Controls */}
        <div className="pr-12 pb-2 pt-2">
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
      </CardContent>
    </Card>
  );
}
