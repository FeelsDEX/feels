// Hook to manage unified historical data for a token
import { useEffect, useState, useMemo } from 'react';
import {
  generateUnifiedHistoricalData,
  getDataForTimeRange,
  UnifiedHistoricalData,
} from '@/utils/generateHistoricalData';
import { KLineData } from '@/types/trading';

interface UseUnifiedHistoricalDataParams {
  tokenAddress: string;
  timeRange: string;
}

interface UseUnifiedHistoricalDataResult {
  loading: boolean;
  error: string | null;
  klineData: KLineData[];
  floorPrice: number;
  gtwapPrice: number;
  floorSeries: { timestamp: number; value: number }[];
  gtwapSeries: { timestamp: number; value: number }[];
  currentPrice: number;
}

// Cache for generated data per token
const dataCache = new Map<string, UnifiedHistoricalData>();

export function useUnifiedHistoricalData({
  tokenAddress,
  timeRange,
}: UseUnifiedHistoricalDataParams): UseUnifiedHistoricalDataResult {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [historicalData, setHistoricalData] = useState<UnifiedHistoricalData | null>(null);

  // Generate or retrieve cached data
  useEffect(() => {
    const loadData = async () => {
      setLoading(true);
      setError(null);

      try {
        // Check cache first
        let data = dataCache.get(tokenAddress);

        if (!data) {
          // Generate new data
          await new Promise((resolve) => setTimeout(resolve, 100)); // Simulate async load

          // Use deterministic parameters based on token address
          const seed = tokenAddress.split('').reduce((sum, char) => sum + char.charCodeAt(0), 0);
          const volatilityFactor = (seed % 100) / 100; // 0-1
          const trendFactor = (seed % 200) / 100 - 1; // -1 to 1

          data = generateUnifiedHistoricalData({
            tokenAddress,
            volatility: 0.02 + volatilityFactor * 0.03, // 2-5% volatility
            trend: 0.0005 + trendFactor * 0.0005, // 0% to 0.1% daily trend (always positive)
          });

          // Cache the data
          dataCache.set(tokenAddress, data);
        }

        setHistoricalData(data);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to generate historical data');
      } finally {
        setLoading(false);
      }
    };

    loadData();
  }, [tokenAddress]);

  // Extract data for current time range
  const timeRangeData = useMemo(() => {
    if (!historicalData) {
      return {
        klineData: [],
        floorSeries: [],
        gtwapSeries: [],
      };
    }

    return getDataForTimeRange(historicalData, timeRange);
  }, [historicalData, timeRange]);

  return {
    loading,
    error,
    klineData: timeRangeData.klineData,
    floorPrice: historicalData?.floorPrice || 0,
    gtwapPrice: historicalData?.gtwapPrice || 0,
    floorSeries: timeRangeData.floorSeries,
    gtwapSeries: timeRangeData.gtwapSeries,
    currentPrice: historicalData?.currentPrice || 0,
  };
}

// Clear cache when needed (e.g., on page navigation)
export function clearHistoricalDataCache() {
  dataCache.clear();
}

// Get raw historical data if needed elsewhere
export function getRawHistoricalData(tokenAddress: string): UnifiedHistoricalData | undefined {
  return dataCache.get(tokenAddress);
}
