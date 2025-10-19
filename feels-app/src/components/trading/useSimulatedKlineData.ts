// Provides K-line data from unified historical data for the price chart.
import { useUnifiedHistoricalData } from '@/hooks/useUnifiedHistoricalData';

export interface OverlayPoint {
  timestamp: number;
  value: number;
}

interface UseSimulatedKlineDataParams {
  tokenAddress: string;
  timeRange: string;
}

export function useSimulatedKlineData({ tokenAddress, timeRange }: UseSimulatedKlineDataParams) {
  // Use the unified historical data
  return useUnifiedHistoricalData({ tokenAddress, timeRange });
}
