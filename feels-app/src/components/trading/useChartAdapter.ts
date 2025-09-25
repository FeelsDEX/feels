// Manages lifecycle and updates for the klinecharts instance used by PriceChart.
import { useCallback, useEffect, useRef, useState } from 'react';
import type { Chart as KLineChart } from 'klinecharts';
import { KLineData } from '@/types/trading';

interface UseChartAdapterParams {
  container: HTMLDivElement | null;
  timezone: string;
  priceAxisType: 'normal' | 'logarithm' | 'percentage';
  formatDate: (timestamp: number, format: string, type: number) => string;
  formatBigNumber: (value: string | number) => string;
}

interface UseChartAdapterResult {
  chart: KLineChart | null;
  isReady: boolean;
  applyData: (data: KLineData[]) => void;
  applyTimezone: (tz: string) => void;
  applyAxisType: (type: 'normal' | 'logarithm' | 'percentage') => void;
  createLineOverlay: (id: string, points: { timestamp: number; value: number }[], color: string) => void;
  setLastPriceVisibility: (visible: boolean) => void;
  setCrosshairVisibility: (visible: boolean) => void;
}

const AXIS_NAME = {
  normal: 'normal',
  logarithm: 'logarithmic',
  percentage: 'percentage'
} as const;

const PLOT_BACKGROUND = '#f8f8f8';
const OVERLAY_NAME = 'feels_line';
let overlaysRegistered = false;

async function ensureCustomOverlayRegistered() {
  if (overlaysRegistered || typeof window === 'undefined') {
    return;
  }

  const { registerOverlay, LineType } = await import('klinecharts');
  
  registerOverlay({
    name: OVERLAY_NAME,
    totalStep: 2,
    needDefaultPointFigure: false,
    needDefaultXAxisFigure: false,
    needDefaultYAxisFigure: false,
    createPointFigures: ({ coordinates, overlay }) => {
      if (!coordinates || coordinates.length < 2) {
        return [];
      }

      const lineStyle = overlay.styles?.line ?? {};

      return coordinates.slice(1).map((coord, index) => ({
        type: 'line',
        attrs: {
          coordinates: [coordinates[index], coord]
        },
        styles: {
          style: lineStyle.style ?? LineType.Solid,
          color: lineStyle.color ?? '#3B82F6',
          size: lineStyle.size ?? 2,
          dashedValue: lineStyle.dashedValue
        }
      }));
    }
  });

  overlaysRegistered = true;
}

export function useChartAdapter({ container, timezone, priceAxisType, formatDate, formatBigNumber }: UseChartAdapterParams): UseChartAdapterResult {
  const chartRef = useRef<KLineChart | null>(null);
  const [isReady, setIsReady] = useState(false);
  const overlayDataRef = useRef<Map<string, { timestamp: number; value: number }[]>>(new Map());

  // ========================================
  // Y-Axis Range Management
  // ========================================
  
  const recalculateYAxisRange = useCallback(() => {
    const chart = chartRef.current;
    if (!chart) return;
    
    const visibleRange = chart.getVisibleRange();
    if (!visibleRange) return;
    
    // Get the current data list to find min/max in visible range
    const dataList = chart.getDataList();
    let minPrice = Infinity;
    let maxPrice = -Infinity;
    
    // Find min/max from candlestick data in visible range
    dataList.forEach((candle: KLineData) => {
      if (candle.timestamp >= visibleRange.from && candle.timestamp <= visibleRange.to) {
        minPrice = Math.min(minPrice, candle.low);
        maxPrice = Math.max(maxPrice, candle.high);
      }
    });
    
    // Include overlay data in range calculation
    overlayDataRef.current.forEach((overlayPoints, overlayId) => {
      overlayPoints.forEach((point) => {
        if (point.timestamp >= visibleRange.from && point.timestamp <= visibleRange.to) {
          minPrice = Math.min(minPrice, point.value);
          maxPrice = Math.max(maxPrice, point.value);
        }
      });
    });
    
    // If we found valid data, apply custom range
    if (minPrice !== Infinity && maxPrice !== -Infinity) {
      const range = maxPrice - minPrice;
      const padding = range * 0.1; // 10% padding
      
      console.log(`[recalculateYAxisRange] Current range: ${minPrice.toFixed(2)} - ${maxPrice.toFixed(2)}, with padding: ${(minPrice - padding).toFixed(2)} - ${(maxPrice + padding).toFixed(2)}`);
      
      // Try to directly manipulate the Y-axis range
      try {
        // Method 1: Try using the chart's internal pane API
        const panes = (chart as any).getPanes?.() || (chart as any)._chartPane?._panes || [];
        const candlePane = panes.find((p: any) => p.id === 'candle_pane' || p._id === 'candle_pane');
        
        if (candlePane) {
          // Try to access the axis component
          const axis = candlePane.getAxisComponent?.() || candlePane._axis || candlePane.axis;
          if (axis && axis.setExtremum) {
            console.log('[recalculateYAxisRange] Using setExtremum method');
            axis.setExtremum(minPrice - padding, maxPrice + padding);
          } else if (axis && axis.setRange) {
            console.log('[recalculateYAxisRange] Using setRange method');
            axis.setRange({ from: minPrice - padding, to: maxPrice + padding });
          } else {
            console.log('[recalculateYAxisRange] No direct axis manipulation method found');
          }
        }
        
        // Method 2: Use data update to force range recalculation
        // Get current data and add temporary extreme points
        const currentData = chart.getDataList();
        if (currentData.length > 0) {
          const firstTimestamp = currentData[0].timestamp;
          const lastTimestamp = currentData[currentData.length - 1].timestamp;
          
          // Create temporary extreme data points
          const tempData = [
            {
              timestamp: firstTimestamp - 1000,
              open: minPrice - padding,
              high: minPrice - padding,
              low: minPrice - padding,
              close: minPrice - padding,
              volume: 0
            },
            ...currentData,
            {
              timestamp: lastTimestamp + 1000,
              open: maxPrice + padding,
              high: maxPrice + padding,
              low: maxPrice + padding,
              close: maxPrice + padding,
              volume: 0
            }
          ];
          
          // Apply the temporary data
          console.log('[recalculateYAxisRange] Applying temporary data with extremes');
          chart.applyNewData(tempData as any, false);
          
          // Restore original data after a brief delay
          setTimeout(() => {
            chart.applyNewData(currentData as any, false);
          }, 50);
        }
        
        // Force a resize
        chart.resize();
      } catch (error) {
        console.error('[recalculateYAxisRange] Error adjusting range:', error);
      }
    }
  }, []);

  // ========================================
  // Chart Initialization
  // ========================================
  
  useEffect(() => {
    ensureCustomOverlayRegistered().catch(console.error);
  }, []);

  useEffect(() => {
    if (!container || chartRef.current || typeof window === 'undefined') {
      return;
    }

    const initChart = async () => {
      const { init, DomPosition, LineType, CandleType, TooltipShowRule, TooltipShowType, dispose } = await import('klinecharts');
      
      // Initialize KLineChart with custom formatting functions
      const chart = init(container, {
        locale: 'en-US',
        timezone
      });

      // Set chart properties
      chartRef.current = chart;
      
      if (!chart) return;
    
    chart.setCustomApi({ formatDate, formatBigNumber });
    chart.setZoomEnabled(true);
    chart.setScrollEnabled(true);
    chart.setBarSpace(8);
    chart.setOffsetRightDistance(80);

    // Set chart background color
    const plotPane = chart.getDom('candle_pane', DomPosition.Main);
    if (plotPane) {
      plotPane.style.backgroundColor = PLOT_BACKGROUND;
    }

    // Store chart reference on container for debugging
    (container as any).__chart__ = chart;

    // ========================================
    // Chart Styling Configuration
    // ========================================
    
    chart.setStyles({
      // Grid lines configuration
      grid: {
        horizontal: {
          show: true,
          size: 1,
          color: 'rgba(150, 150, 150, 0.15)',
          style: LineType.Solid,
          // dashedValue: [4, 4]
        },
        vertical: {
          show: true,
          size: 1,
          color: 'rgba(150, 150, 150, 0.15)',
          style: LineType.Solid,
          // dashedValue: [4, 4]
        }
      },
      // Candlestick styling
      candle: {
        type: CandleType.CandleSolid,
        bar: {
          upColor: '#5cca39',
          downColor: '#ef5350',
          noChangeColor: '#666666',
          upBorderColor: '#5cca39',
          downBorderColor: '#ef5350',
          noChangeBorderColor: '#666666',
          upWickColor: '#5cca39',
          downWickColor: '#ef5350',
          noChangeWickColor: '#666666'
        },
        // Price marks (high/low/last price indicators)
        priceMark: {
          show: true,
          high: { show: true, color: '#a6a6a6', textSize: 10, textFamily: 'monospace' },
          low: { show: true, color: '#a6a6a6', textSize: 10, textFamily: 'monospace' },
          last: {
            show: true,
            upColor: '#a0a0a0',
            downColor: '#a0a0a0',
            noChangeColor: '#a0a0a0',
            line: {
              show: true,
              style: LineType.Dashed,
              size: 1,
              color: '#a0a0a0',
              dashedValue: [4, 4]
            },
            text: {
              show: true,
              size: 12,
              paddingLeft: 4,
              paddingTop: 4,
              paddingRight: 4,
              paddingBottom: 4,
              borderRadius: 2,
              borderSize: 1,
              borderColor: '#8f8f8f',
              backgroundColor: '#a0a0a0',
              color: '#ffffff'
            }
          }
        },
        // Tooltip configuration
        tooltip: {
          showRule: TooltipShowRule.Always,
          showType: TooltipShowType.Standard,
          rect: {
            paddingLeft: 8,
            paddingRight: 8,
            paddingTop: 8,
            paddingBottom: 8,
            offsetLeft: 8,
            offsetTop: 8,
            offsetRight: 8,
            borderRadius: 4,
            borderSize: 1,
            borderColor: 'rgba(128, 128, 128, 0.3)'
          },
          text: {
            size: 11,
            family: 'var(--font-terminal-grotesque), monospace',
            weight: 'normal',
            color: '#4B4B4B'
          }
        }
      },
      // X-axis (time) styling
      xAxis: {
        axisLine: { show: true, color: 'rgba(128, 128, 128, 0.3)', size: 1 },
        tickLine: { show: true, size: 1, length: 3, color: 'rgba(128, 128, 128, 0.3)' },
        tickText: {
          show: true,
          color: '#4B4B4B',
          family: 'var(--font-terminal-grotesque), monospace',
          weight: 'normal',
          size: 11
        }
      },
      // Y-axis (price) styling
      yAxis: {
        axisLine: { show: true, color: 'rgba(128, 128, 128, 0.3)', size: 1 },
        tickLine: { show: true, size: 1, length: 3, color: 'rgba(128, 128, 128, 0.3)' },
        tickText: {
          show: true,
          color: '#4B4B4B',
          family: 'var(--font-terminal-grotesque), monospace',
          weight: 'normal',
          size: 11
        }
      },
      // Crosshair cursor styling
      crosshair: {
        show: true,
        horizontal: {
          show: true,
          line: {
            show: true,
            style: LineType.Dashed,
            size: 1,
            color: '#262626',
            dashedValue: [4, 2]
          },
          text: {
            show: true,
            size: 12,
            family: 'Helvetica Neue',
            weight: 'normal',
            color: '#ffffff',
            paddingLeft: 4,
            paddingRight: 4,
            paddingTop: 4,
            paddingBottom: 4,
            borderRadius: 2,
            borderSize: 1,
            borderColor: '#c0c0c0',
            backgroundColor: '#a0a0a0'
          }
        },
        vertical: {
          show: true,
          line: {
            show: true,
            style: LineType.Dashed,
            size: 1,
            color: '#262626',
            dashedValue: [4, 2]
          },
          text: {
            show: true,
            size: 12,
            family: 'Helvetica Neue',
            weight: 'normal',
            color: '#ffffff',
            paddingLeft: 4,
            paddingRight: 4,
            paddingTop: 4,
            paddingBottom: 4,
            borderRadius: 2,
            borderSize: 1,
            borderColor: '#c0c0c0',
            backgroundColor: '#a0a0a0'
          }
        }
      }
    } as any);

    setIsReady(true);
    
    // Subscribe to visible range changes to recalculate Y-axis
    chart.subscribeAction('onVisibleRangeChange', () => {
      // Debounce the recalculation to avoid too many updates
      if (overlayDataRef.current.size > 0) {
        setTimeout(() => recalculateYAxisRange(), 50);
      }
    });
      
      return dispose;
    };

    let disposeFunc: ((chart: any) => void) | null = null;
    
    initChart().then(dispose => {
      if (dispose) disposeFunc = dispose;
    }).catch(console.error);

    return () => {
      if (chartRef.current && disposeFunc) {
        disposeFunc(chartRef.current);
        chartRef.current = null;
        setIsReady(false);
      }
    };
  }, [container, formatBigNumber, formatDate, timezone, recalculateYAxisRange]);

  // ========================================
  // Chart Data Management
  // ========================================

  const applyData = useCallback((data: KLineData[]) => {
    const chart = chartRef.current;
    if (!chart || data.length === 0) {
      return;
    }
    // Replace all chart data with new dataset
    chart.clearData();
    chart.applyNewData(data as any);
  }, []);

  const applyTimezone = useCallback((tz: string) => {
    // Update chart timezone for time axis formatting
    chartRef.current?.setTimezone(tz);
  }, []);

  const applyAxisType = useCallback((type: 'normal' | 'logarithm' | 'percentage') => {
    // Switch price axis scaling (linear/log/percentage)
    const axisName = AXIS_NAME[type];
    chartRef.current?.setPaneOptions({ id: 'candle_pane', axis: { name: axisName } });
  }, []);


  // ========================================
  // Effect Handlers for Settings
  // ========================================
  useEffect(() => {
    if (!chartRef.current) {
      return;
    }
    chartRef.current.setCustomApi({ formatDate, formatBigNumber });
  }, [formatBigNumber, formatDate]);

  useEffect(() => {
    if (!chartRef.current) {
      return;
    }
    // Apply timezone changes to existing chart
    applyTimezone(timezone);
  }, [applyTimezone, timezone]);

  useEffect(() => {
    if (!chartRef.current) {
      return;
    }
    // Apply axis type changes to existing chart
    applyAxisType(priceAxisType);
  }, [applyAxisType, priceAxisType]);

  // ========================================
  // Overlay and Visual Controls
  // ========================================

  const createLineOverlay = useCallback(async (id: string, points: { timestamp: number; value: number }[], color: string) => {
    if (typeof window === 'undefined') return;
    const chart = chartRef.current;
    if (!chart || points.length === 0) {
      return;
    }

    // Remove existing overlay with same ID if it exists
    try {
      chart.removeOverlay({ id });
      overlayDataRef.current.delete(id);
      console.log(`[useChartAdapter] Overlay removed: ${id}`);
      
      // Force recalculation after removal
      setTimeout(() => {
        recalculateYAxisRange();
      }, 100);
    } catch {
      // Overlay might not exist; ignore removal failures.
    }

    // Import LineType for styling
    const { LineType } = await import('klinecharts');
    
    // Transform points to chart format
    const overlayPoints = points.map((point) => ({ timestamp: point.timestamp, value: point.value }));

    chart.createOverlay({
      name: OVERLAY_NAME,
      id,
      lock: true,
      points: overlayPoints,
      styles: {
        line: {
          color,
          size: 2, // Increased size for visibility
          style: LineType.Solid
        }
      }
    } as any);

    // Store overlay data for range calculation
    overlayDataRef.current.set(id, points);
    console.log(`[useChartAdapter] Overlay created: ${id}, points: ${points.length}`);
    
    // Force chart to update Y-axis range to include overlay data
    setTimeout(() => {
      console.log(`[useChartAdapter] Updating Y-axis range for overlay: ${id}`);
      
      // Recalculate and apply custom Y-axis range
      recalculateYAxisRange();
    }, 100);
  }, [recalculateYAxisRange]);

  const setLastPriceVisibility = useCallback(async (visible: boolean) => {
    if (typeof window === 'undefined') return;
    const { LineType } = await import('klinecharts');
    // Toggle visibility of last price horizontal line and label
    chartRef.current?.setStyles({
      candle: {
        priceMark: {
          last: {
            show: visible,
            upColor: '#a0a0a0',
            downColor: '#a0a0a0',
            noChangeColor: '#a0a0a0',
            line: { show: visible, color: '#a0a0a0', style: LineType.Dashed },
            text: {
              show: visible,
              size: 12,
              paddingLeft: 4,
              paddingTop: 4,
              paddingRight: 4,
              paddingBottom: 4,
              borderRadius: 2,
              borderSize: 1,
              borderColor: '#8f8f8f',
              backgroundColor: '#a0a0a0',
              color: '#ffffff'
            }
          }
        }
      }
    } as any);
  }, []);

  const setCrosshairVisibility = useCallback(async (visible: boolean) => {
    if (typeof window === 'undefined') return;
    const { LineType } = await import('klinecharts');
    // Toggle crosshair cursor visibility
    chartRef.current?.setStyles({
      crosshair: {
        show: visible,
        horizontal: {
          show: visible,
          line: {
            show: visible,
            style: LineType.Dashed,
            size: 1,
            color: '#262626',
            dashedValue: [4, 2]
          },
          text: {
            show: visible,
            size: 12,
            family: 'Helvetica Neue',
            weight: 'normal',
            color: '#ffffff',
            paddingLeft: 4,
            paddingRight: 4,
            paddingTop: 4,
            paddingBottom: 4,
            borderRadius: 2,
            borderSize: 1,
            borderColor: '#c0c0c0',
            backgroundColor: '#a0a0a0'
          }
        },
        vertical: {
          show: visible,
          line: {
            show: visible,
            style: LineType.Dashed,
            size: 1,
            color: '#262626',
            dashedValue: [4, 2]
          },
          text: {
            show: visible,
            size: 12,
            family: 'Helvetica Neue',
            weight: 'normal',
            color: '#ffffff',
            paddingLeft: 4,
            paddingRight: 4,
            paddingTop: 4,
            paddingBottom: 4,
            borderRadius: 2,
            borderSize: 1,
            borderColor: '#c0c0c0',
            backgroundColor: '#a0a0a0'
          }
        }
      }
    } as any);
  }, []);

  return {
    chart: chartRef.current,
    isReady,
    applyData,
    applyTimezone,
    applyAxisType,
    createLineOverlay,
    setLastPriceVisibility,
    setCrosshairVisibility
  };
}

