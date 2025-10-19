// React hook that wraps KLineChart and keeps chart configuration in sync with UI state
import { useCallback, useEffect, useRef, useState } from 'react';
import type { Chart as KLineChart } from 'klinecharts';
import type { KLineData } from '@/types/trading';
import { 
  buildChartStyles, 
  createFloorIndicator, 
  createGtwapIndicator,
  AXIS_NAME,
  type AxisType 
} from './chart-config';

// Indicator state tracking
interface IndicatorState {
  floorIndicatorId?: string;
  gtwapIndicatorId?: string;
}

// Hook parameters - what the consumer provides
interface UseChartAdapterParams {
  container: HTMLDivElement | null;
  timezone: string;
  priceAxisType: 'normal' | 'logarithm' | 'percentage';
  formatDate: (timestamp: number, format: string, type: number) => string;
  formatBigNumber: (value: string | number) => string;
  showUSD?: boolean;
  usdConversionFactor?: number;
}

// Hook return value - the API exposed to consumers
interface UseChartAdapterResult {
  chart: KLineChart | null;
  isReady: boolean;
  applyChartData: (config: {
    data: KLineData[];
    floor?: { visible: boolean; series: Array<{ timestamp: number; value: number }> };
    gtwap?: { visible: boolean; series: Array<{ timestamp: number; value: number }> };
  }) => void;
  applyTimezone: (tz: string) => void;
  applyAxisType: (type: 'normal' | 'logarithm' | 'percentage') => Promise<void>;
  setLastPriceVisibility: (visible: boolean) => Promise<void>;
  setCrosshairVisibility: (visible: boolean) => Promise<void>;
  setFloorVisibility: (visible: boolean) => Promise<void>;
  setGtwapVisibility: (visible: boolean) => Promise<void>;
  resetVisibleRange: () => void;
}

/**
 * Main hook for integrating KLineChart with React
 * Handles chart initialization, data updates, and configuration
 */
export function useChartAdapter(params: UseChartAdapterParams): UseChartAdapterResult {
  const { 
    container, 
    timezone, 
    formatDate, 
    formatBigNumber,
    showUSD = false,
    usdConversionFactor = 1
  } = params;

  // Chart instance and state
  const [chart, setChart] = useState<KLineChart | null>(null);
  const [isReady, setIsReady] = useState(false);
  const chartDataRef = useRef<KLineData[]>([]);
  const floorDataRef = useRef<Map<number, number>>(new Map());
  const gtwapDataRef = useRef<Map<number, number>>(new Map());
  const showFloorRef = useRef(false);
  const showGtwapRef = useRef(false);
  const indicatorStateRef = useRef<IndicatorState>({});

  // Initialize chart - basic initialization without styling
  const initChart = useCallback(async () => {
    if (!container) return;

    // Clean up any existing chart first
    if (chart) {
      try {
        if (typeof (chart as any).dispose === 'function') {
          (chart as any).dispose();
        }
      } catch (error) {
        console.warn('Error disposing existing chart:', error);
      }
      setChart(null);
      setIsReady(false);
    }

    try {
      // Temporarily suppress console.log to hide klinecharts welcome message
      const originalConsoleLog = console.log;
      console.log = () => {};
      
      const { init, registerIndicator } = await import('klinecharts');
      
      // Register combined overlay indicator
      registerIndicator({
        name: 'FEELS_OVERLAY',
        shortName: '',
        calcParams: [],
        figures: [
          {
            key: 'floorLine',
            title: 'Floor: ',
            type: 'line',
            styles: (data: any) => ({
              color: '#3B82F6',
              size: 1,
              lineWidth: 1,
              solid: true,
            }),
          },
          {
            key: 'gtwapLine',
            title: 'GTWAP: ',
            type: 'line',
            styles: (data: any) => ({
              color: '#5cca39',
              size: 1,
              lineWidth: 1,
              solid: true,
            }),
          },
        ],
        precision: 4,
        shouldOhlc: false,
        shouldFormatBigNumber: true,
        calc: (dataList: any[]) => {
          const floorData = (window as any).__floorPriceData || new Map();
          const gtwapData = (window as any).__gtwapPriceData || new Map();
          const showFloor = (window as any).__showFloorIndicator || false;
          const showGtwap = (window as any).__showGtwapIndicator || false;

          return dataList.map(kline => {
            const result: any = {};
            if (showFloor) {
              result.floorLine = floorData.get(kline.timestamp);
            }
            if (showGtwap) {
              result.gtwapLine = gtwapData.get(kline.timestamp);
            }
            return result;
          });
        }
      });
      
      const chartInstance = init(container);
      
      // Restore console.log after chart initialization
      console.log = originalConsoleLog;
      
      if (!chartInstance) {
        console.error('Failed to initialize chart');
        return;
      }

      setChart(chartInstance);
      setIsReady(true);

    } catch (error) {
      console.error('Chart initialization error:', error);
      setIsReady(false);
    }
  }, [container]);

  // Apply styles separately when chart configuration changes
  useEffect(() => {
    if (!chart || !isReady) return;

    const styles = buildChartStyles({
      timezone,
      formatDate,
      formatBigNumber,
      showUSD,
      usdConversionFactor,
    });

    chart.setStyles(styles);
  }, [chart, isReady, timezone, formatDate, formatBigNumber, showUSD, usdConversionFactor]);

  // Initialize on mount and when container changes
  useEffect(() => {
    if (!container) return;
    
    initChart();

    return () => {
      // Cleanup - dispose of the chart instance to prevent multiple charts
      if (chart) {
        try {
          // KLineChart disposal method
          if (typeof (chart as any).dispose === 'function') {
            (chart as any).dispose();
          }
        } catch (error) {
          console.warn('Error disposing chart:', error);
        }
      }
      setChart(null);
      setIsReady(false);
    };
  }, [container]); // Only re-run when container changes, not when initChart changes

  // Apply chart data and overlay indicators
  const applyChartData = useCallback((config: {
    data: KLineData[];
    floor?: { visible: boolean; series: Array<{ timestamp: number; value: number }> };
    gtwap?: { visible: boolean; series: Array<{ timestamp: number; value: number }> };
  }) => {
    if (!chart) return;

    const { data, floor, gtwap } = config;
    chartDataRef.current = data;

    // Update floor data
    if (floor) {
      const floorMap = new Map(floor.series.map(item => [item.timestamp, item.value]));
      floorDataRef.current = floorMap;
      showFloorRef.current = floor.visible;
    }

    // Update gtwap data
    if (gtwap) {
      const gtwapMap = new Map(gtwap.series.map(item => [item.timestamp, item.value]));
      gtwapDataRef.current = gtwapMap;
      showGtwapRef.current = gtwap.visible;
    }

    // Apply data to chart
    chart.applyNewData(data);

    // Update global data for the indicator
    (window as any).__floorPriceData = floorDataRef.current;
    (window as any).__gtwapPriceData = gtwapDataRef.current;
    (window as any).__showFloorIndicator = showFloorRef.current;
    (window as any).__showGtwapIndicator = showGtwapRef.current;

    // Remove existing indicator if both are hidden
    if (!showFloorRef.current && !showGtwapRef.current) {
      if (indicatorStateRef.current.floorIndicatorId) {
        chart.removeIndicator(indicatorStateRef.current.floorIndicatorId);
        indicatorStateRef.current.floorIndicatorId = undefined;
      }
    } else {
      // Create or update the overlay indicator
      if (!indicatorStateRef.current.floorIndicatorId) {
        const id = chart.createIndicator({
          name: 'FEELS_OVERLAY',
          id: 'overlay_indicator',
        }, false, { id: 'candle_pane' });
        
        if (id) {
          indicatorStateRef.current.floorIndicatorId = id;
        }
      }
    }

    // Force update by re-applying data
    if (data.length > 0) {
      chart.applyNewData(data);
    }
    
  }, [chart]);

  // Apply timezone
  const applyTimezone = useCallback((tz: string) => {
    if (!chart) return;
    chart.setTimezone(tz);
  }, [chart]);

  // Apply axis type
  const applyAxisType = useCallback(async (type: AxisType) => {
    if (!chart) return;
    
    (chart as any).setStyles({
      yAxis: {
        type: AXIS_NAME[type],
      },
    });
  }, [chart]);

  // Toggle last price visibility
  const setLastPriceVisibility = useCallback(async (visible: boolean) => {
    if (!chart) return;

    (chart as any).setStyles({
      candle: {
        priceMark: {
          last: {
            show: visible,
          },
        },
      },
    });
  }, [chart]);

  // Toggle crosshair visibility
  const setCrosshairVisibility = useCallback(async (visible: boolean) => {
    if (!chart) return;

    (chart as any).setStyles({
      crosshair: {
        show: visible,
        horizontal: { show: visible },
        vertical: { show: visible },
      },
    });
  }, [chart]);

  // Toggle floor visibility
  const setFloorVisibility = useCallback(async (visible: boolean) => {
    if (!chart) return;
    showFloorRef.current = visible;
    
    // Re-apply chart data to update indicators
    if (chartDataRef.current.length > 0) {
      applyChartData({
        data: chartDataRef.current,
        floor: { visible: showFloorRef.current, series: Array.from(floorDataRef.current).map(([timestamp, value]) => ({ timestamp, value })) },
        gtwap: { visible: showGtwapRef.current, series: Array.from(gtwapDataRef.current).map(([timestamp, value]) => ({ timestamp, value })) }
      });
    }
  }, [chart, applyChartData]);

  // Toggle gtwap visibility
  const setGtwapVisibility = useCallback(async (visible: boolean) => {
    if (!chart) return;
    showGtwapRef.current = visible;
    
    // Re-apply chart data to update indicators
    if (chartDataRef.current.length > 0) {
      applyChartData({
        data: chartDataRef.current,
        floor: { visible: showFloorRef.current, series: Array.from(floorDataRef.current).map(([timestamp, value]) => ({ timestamp, value })) },
        gtwap: { visible: showGtwapRef.current, series: Array.from(gtwapDataRef.current).map(([timestamp, value]) => ({ timestamp, value })) }
      });
    }
  }, [chart, applyChartData]);

  // Reset visible range
  const resetVisibleRange = useCallback(() => {
    if (!chart) return;
    chart.scrollToRealTime();
  }, [chart]);

  return {
    chart,
    isReady,
    applyChartData,
    applyTimezone,
    applyAxisType,
    setLastPriceVisibility,
    setCrosshairVisibility,
    setFloorVisibility,
    setGtwapVisibility,
    resetVisibleRange,
  };
}

