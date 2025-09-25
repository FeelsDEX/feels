// Provides simulated K-line data and derived overlay series for the price chart.
import { useCallback, useEffect, useState } from 'react';
import { KLineData } from '@/types/trading';

// ========================================
// Configuration Constants
// ========================================

// Time range patterns define data points, intervals, and market behavior
const KLINE_TIME_RANGE_PATTERNS: Record<string, { points: number; interval: number; volatility: number; trend: number }> = {
  '1m': { points: 60, interval: 60 * 1000, volatility: 0.001, trend: 0.00001 },
  '1h': { points: 72, interval: 60 * 60 * 1000, volatility: 0.005, trend: 0.00005 },
  '1D': { points: 30, interval: 24 * 60 * 60 * 1000, volatility: 0.015, trend: 0.0002 },
  '1W': { points: 52, interval: 7 * 24 * 60 * 60 * 1000, volatility: 0.025, trend: 0.0005 },
  '1M': { points: 30, interval: 30 * 24 * 60 * 60 * 1000, volatility: 0.03, trend: 0.001 },
  all: { points: 200, interval: 7 * 24 * 60 * 60 * 1000, volatility: 0.05, trend: 0.003 }
};

// Different price scenarios for realistic token price ranges
const KLINE_PRICE_SCENARIOS = [
  { base: 0.00001, description: 'micro' },
  { base: 0.001, description: 'small' },
  { base: 1.5, description: 'medium' },
  { base: 125.75, description: 'large' },
  { base: 15420.5, description: 'huge' }
] as const;

const VOLUME_BASE_UNITS = 1_000_000; // Base volume for simulation

export interface OverlayPoint {
  timestamp: number;
  value: number;
}

interface UseSimulatedKlineDataParams {
  tokenAddress: string;
  timeRange: string;
}

interface GeneratedSeries {
  data: KLineData[];
  floorPrice: number;
  gtwap: number;
  floorPrices: OverlayPoint[];
  gtwapPrices: OverlayPoint[];
}

export function useSimulatedKlineData({ tokenAddress, timeRange }: UseSimulatedKlineDataParams) {
  // ========================================
  // State Management
  // ========================================
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [klineData, setKlineData] = useState<KLineData[]>([]);
  const [floorSeries, setFloorSeries] = useState<OverlayPoint[]>([]);
  const [gtwapSeries, setGtwapSeries] = useState<OverlayPoint[]>([]);
  const [floorPrice, setFloorPrice] = useState(0);
  const [gtwapPrice, setGtwapPrice] = useState(0);

  // ========================================
  // Data Generation Logic
  // ========================================

  const generateSeries = useCallback((): GeneratedSeries => {
    const now = Date.now();
    const data: KLineData[] = [];

    // Select time pattern based on range
    const pattern = KLINE_TIME_RANGE_PATTERNS[timeRange] || KLINE_TIME_RANGE_PATTERNS['1D'];
    if (!pattern) {
      return { data: [], floorPrice: 0, gtwap: 0, floorPrices: [], gtwapPrices: [] };
    }

    // Choose price scenario based on token address (deterministic)
    const scenarioIndex = tokenAddress.charCodeAt(0) % KLINE_PRICE_SCENARIOS.length;
    const scenario = KLINE_PRICE_SCENARIOS[scenarioIndex];
    if (!scenario) {
      return { data: [], floorPrice: 0, gtwap: 0, floorPrices: [], gtwapPrices: [] };
    }

    // Initialize price with some randomness
    let basePrice = scenario.base * (0.8 + Math.random() * 0.4);
    let currentPrice = basePrice;
    const trendDirection = Math.random() > 0.5 ? 1 : -1;

    // Generate candlestick data points (working backwards from now)
    for (let i = pattern.points - 1; i >= 0; i--) {
      const timestamp = now - i * pattern.interval;
      // Create realistic price movements with cycles and trends
      const cycleFactor = Math.sin((i / pattern.points) * Math.PI * 4) * 0.1; // Longer cycles
      const microCycle = Math.sin((i / pattern.points) * Math.PI * 20) * 0.02; // Short-term noise
      const randomWalk = (Math.random() - 0.5) * pattern.volatility * basePrice;
      const trendComponent = trendDirection * pattern.trend * basePrice * (i / pattern.points);

      // Apply price evolution with floor protection
      currentPrice = Math.max(basePrice * 0.1, currentPrice + randomWalk + trendComponent * 0.1);
      const cyclePrice = currentPrice * (1 + cycleFactor + microCycle);

      // Build OHLC values for this candle
      const open = i === pattern.points - 1 ? currentPrice : data[data.length - 1]?.close || currentPrice;
      const close = cyclePrice;
      const high = Math.max(open, close) * (1 + Math.random() * pattern.volatility * 0.5);
      const low = Math.min(open, close) * (1 - Math.random() * pattern.volatility * 0.5);
      
      // Generate volume based on price movement
      const priceChange = Math.abs(close - open) / open;
      const baseVolume = VOLUME_BASE_UNITS * (0.5 + Math.random());
      const volume = baseVolume * (1 + priceChange * 10) * (1 + cycleFactor * 0.5);

      data.push({
        timestamp,
        open,
        high: Math.max(high, open, close),
        low: Math.min(low, open, close),
        close,
        volume,
        turnover: volume * close
      });
    }

    // ========================================
    // Generate Overlay Series (Floor & GTWAP)
    // ========================================
    const floorPrices: OverlayPoint[] = [];
    const gtwapPrices: OverlayPoint[] = [];
    let currentFloor = (data[0]?.low || 0) * 0.85; // Start floor below initial low
    const dailyFloorIncrease = 0.001; // 0.1% daily floor increase
    let cumulativeTickTime = 0; // For GTWAP calculation
    let lastTimestamp = data[0]?.timestamp || 0;

    data.forEach((candle) => {
      const timeDelta = candle.timestamp - lastTimestamp;
      const timeWeight = timeDelta / (60 * 1000); // Convert to minutes
      const timeBasedIncrease = currentFloor * dailyFloorIncrease * (timeDelta / (24 * 60 * 60 * 1000));

      // Update floor price (gradually increasing over time)
      currentFloor = Math.max(currentFloor, currentFloor + timeBasedIncrease);
      floorPrices.push({ timestamp: candle.timestamp, value: currentFloor });

      // Calculate GTWAP (Geometric Time-Weighted Average Price)
      const currentTick = Math.log(candle.close) / Math.log(1.0001); // Convert price to tick
      cumulativeTickTime += currentTick * timeWeight;
      const totalTime = (candle.timestamp - (data[0]?.timestamp || 0)) / (60 * 1000);
      const avgTick = totalTime > 0 ? cumulativeTickTime / totalTime : currentTick;
      const gtwapValue = Math.pow(1.0001, avgTick); // Convert back to price

      gtwapPrices.push({ timestamp: candle.timestamp, value: gtwapValue });
      lastTimestamp = candle.timestamp;
    });

    return {
      data,
      floorPrice: floorPrices[floorPrices.length - 1]?.value || 0,
      gtwap: gtwapPrices[gtwapPrices.length - 1]?.value || 0,
      floorPrices,
      gtwapPrices
    };
  }, [timeRange, tokenAddress]);

  // ========================================
  // Data Loading Effect
  // ========================================
  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      setLoading(true);
      setError(null);
      try {
        // Small delay to simulate data loading
        await new Promise((resolve) => setTimeout(resolve, 50));
        const result = generateSeries();
        if (cancelled) return;
        
        // Update all state with generated data
        setKlineData(result.data);
        setFloorSeries(result.floorPrices);
        setGtwapSeries(result.gtwapPrices);
        setFloorPrice(result.floorPrice);
        setGtwapPrice(result.gtwap);
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : 'Failed to load price data');
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };

    load();

    return () => {
      cancelled = true;
    };
  }, [generateSeries]);

  return {
    loading,
    error,
    klineData,
    floorPrice,
    gtwapPrice,
    floorSeries,
    gtwapSeries
  };
}

export { KLINE_TIME_RANGE_PATTERNS, KLINE_PRICE_SCENARIOS, VOLUME_BASE_UNITS };

