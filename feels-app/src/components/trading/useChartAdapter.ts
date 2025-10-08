// Provides a React hook that wraps KLineChart and keeps chart configuration in sync with UI state.
import { useCallback, useEffect, useRef, useState } from 'react';
import type { Chart as KLineChart } from 'klinecharts';
import type { KLineData } from '@/types/trading';

// ============================================================================
// CONSTANTS
// ============================================================================

// Chart identifiers
const OVERLAY_NAME = 'feels_line';
const PLOT_BACKGROUND = '#f8f8f8';

// Visual styling constants
const CHART_COLORS = {
  up: '#5cca39',
  down: '#ef5350',
  neutral: '#666666',
  priceMark: '#a6a6a6',
  lastPrice: '#b0b0b0',
  text: '#4B4B4B',
  grid: 'rgba(150, 150, 150, 0.15)',
  axisLine: 'rgba(128, 128, 128, 0.3)',
  crosshair: '#555555',
};

// Font stacks that reference CSS variables from globals.css
const FONT_DEFAULT = 'var(--font-default), Helvetica, ui-sans-serif, system-ui, sans-serif';
const FONT_MONO = 'var(--font-mono), "JetBrains Mono", ui-monospace, monospace';

// ============================================================================
// TYPE DEFINITIONS
// ============================================================================

type AxisType = 'normal' | 'logarithm' | 'percentage';
type StylesLike = Record<string, any>;

// Axis name mapping for KLineChart API
const AXIS_NAME: Record<AxisType, AxisType> = {
  normal: 'normal',
  logarithm: 'logarithm',
  percentage: 'percentage',
};


// Indicator state
interface IndicatorState {
  floorIndicatorId?: string;
  gtwapIndicatorId?: string;
  overlayIndicatorId?: string; // Combined indicator for both Floor and GTWAP
}

// Complete chart configuration state
interface ChartConfigState {
  axisType: AxisType;
  lastPriceVisible: boolean;
  crosshairVisible: boolean;
  data: KLineData[];
  showFloor: boolean;
  showGtwap: boolean;
  floorData: Map<number, number>;
  gtwapData: Map<number, number>;
}

// Partial overrides to apply on top of defaults
interface ChartConfigOverrides {
  axisType?: AxisType;
  lastPriceVisible?: boolean;
  crosshairVisible?: boolean;
  data?: KLineData[];
  showFloor?: boolean;
  showGtwap?: boolean;
  floorData?: Map<number, number>;
  gtwapData?: Map<number, number>;
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

// ============================================================================
// CONFIGURATION OBJECTS
// ============================================================================

// Reusable text styling components
const TEXT_STYLE_BASE = {
  size: 12,
  family: 'Helvetica Neue',
  weight: 'normal',
  color: '#ffffff',
};

const TEXT_PADDING = {
  paddingLeft: 4,
  paddingRight: 4,
  paddingTop: 4,
  paddingBottom: 4,
};

const TEXT_BOX_STYLE = {
  ...TEXT_STYLE_BASE,
  ...TEXT_PADDING,
  borderRadius: 2,
  borderSize: 1,
};

// Tooltip configuration - defines how price/volume info appears on hover
const TOOLTIP_CONFIG = {
  showRule: 'Always' as any, // Will be replaced with TooltipShowRule.Always
  showType: 'Standard' as any, // Will be replaced with TooltipShowType.Standard
  custom: [
    { title: 'Open: ', value: '{open}' },
    { title: 'High: ', value: '{high}' },
    { title: 'Low: ', value: '{low}' },
    { title: 'Close: ', value: '{close}' },
    { title: 'Volume: ', value: '{volume}' }
  ],
  defaultValue: 'n/a',
  text: {
    size: 11,
    family: FONT_DEFAULT,
    weight: 'normal',
    color: CHART_COLORS.text,
    marginLeft: 8,
    marginTop: 4,
    marginRight: 8,
    marginBottom: 4,
  },
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
    borderColor: 'rgba(128, 128, 128, 0.3)',
  },
};

// Shared configuration for both X and Y axes
const AXIS_CONFIG = {
  axisLine: { show: true, color: CHART_COLORS.axisLine, size: 1 },
  tickLine: { show: true, size: 1, length: 3, color: CHART_COLORS.axisLine },
  tickText: {
    show: true,
    color: CHART_COLORS.text,
    family: FONT_DEFAULT,
    weight: 'normal',
    size: 11,
  },
};

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/**
 * Creates price mark configuration showing high/low/last prices on the chart
 */
function createPriceMarkConfig(LineType: any) {
  return {
    show: true,
    high: { show: true, color: CHART_COLORS.priceMark, textSize: 10, textFamily: FONT_MONO },
    low: { show: true, color: CHART_COLORS.priceMark, textSize: 10, textFamily: FONT_MONO },
    last: {
      show: true,
      upColor: CHART_COLORS.lastPrice,
      downColor: CHART_COLORS.lastPrice,
      noChangeColor: CHART_COLORS.lastPrice,
      line: {
        show: true,
        style: LineType.Dashed,
        size: 1,
        color: CHART_COLORS.lastPrice,
        dashedValue: [4, 4],
      },
      text: {
        ...TEXT_BOX_STYLE,
        borderColor: CHART_COLORS.lastPrice,
        backgroundColor: CHART_COLORS.lastPrice,
      },
    },
  };
}

/**
 * Creates crosshair configuration for tracking mouse position on chart
 */
function createCrosshairConfig(visible: boolean, LineType: any) {
  const lineConfig = {
    show: visible,
    style: LineType.Dashed,
    size: 1,
    color: CHART_COLORS.crosshair,
    dashedValue: [4, 2],
  };
  
  const textConfig = {
    show: visible,
    ...TEXT_BOX_STYLE,
    borderColor: CHART_COLORS.crosshair,
    backgroundColor: CHART_COLORS.crosshair,
  };
  
  return {
    show: visible,
    horizontal: {
      show: visible,
      line: lineConfig,
      text: textConfig,
    },
    vertical: {
      show: visible,
      line: lineConfig,
      text: textConfig,
    },
  };
}


/**
 * Deep clones style objects to prevent mutation
 */
function cloneStyles<T>(styles: T): T {
  if (typeof structuredClone === 'function') {
    return structuredClone(styles);
  }
  return JSON.parse(JSON.stringify(styles));
}


/**
 * Merges default configuration with user overrides
 * Overrides take precedence over defaults
 */
function mergeConfigs(
  defaults: ChartConfigState,
  overrides: ChartConfigOverrides
): ChartConfigState {
  const floorData = overrides.floorData || defaults.floorData;
  const gtwapData = overrides.gtwapData || defaults.gtwapData;

  return {
    axisType: overrides.axisType ?? defaults.axisType,
    lastPriceVisible: overrides.lastPriceVisible ?? defaults.lastPriceVisible,
    crosshairVisible: overrides.crosshairVisible ?? defaults.crosshairVisible,
    data: overrides.data ?? defaults.data,
    showFloor: overrides.showFloor ?? defaults.showFloor,
    showGtwap: overrides.showGtwap ?? defaults.showGtwap,
    floorData,
    gtwapData,
  };
}

/**
 * Builds complete style configuration from base styles and current config state
 * This is the core function that applies all visual settings to the chart
 */
function buildStyles(baseStyles: StylesLike, config: ChartConfigState, LineType: any) {
  const styles = cloneStyles(baseStyles);

  const candle = styles['candle'];
  
  // Preserve custom tooltip configuration
  if (candle?.tooltip) {
    candle.tooltip.custom = TOOLTIP_CONFIG.custom;
  }
  
  // Configure last price line visibility and styling
  if (candle?.priceMark?.last) {
    const last = candle.priceMark.last;
    last.show = config.lastPriceVisible;
    last.upColor = CHART_COLORS.lastPrice;
    last.downColor = CHART_COLORS.lastPrice;
    last.noChangeColor = CHART_COLORS.lastPrice;
    if (last.line) {
      last.line.show = config.lastPriceVisible;
      last.line.color = CHART_COLORS.lastPrice;
      last.line.style = LineType.Dashed;
      last.line.dashedValue = [4, 4];
    }
    if (last.text) {
      last.text = {
        ...TEXT_BOX_STYLE,
        show: config.lastPriceVisible,
        borderColor: CHART_COLORS.lastPrice,
        backgroundColor: CHART_COLORS.lastPrice,
      };
    }
  }

  // Apply crosshair configuration
  styles['crosshair'] = createCrosshairConfig(config.crosshairVisible, LineType);

  // Configure Y-axis type (normal, logarithm, or percentage)
  const yAxis = (styles['yAxis'] = styles['yAxis'] ?? {});
  yAxis.type = AXIS_NAME[config.axisType];

  // Percentage axis requires a base value for calculations
  if (config.axisType === 'percentage') {
    const first = config.data[0];
    if (first) {
      yAxis.baseValue = first.close ?? first.open;
    }
  } else if (yAxis.baseValue !== undefined) {
    delete yAxis.baseValue;
  }

  return styles;
}

/**
 * Registers custom indicators for Floor and GTWAP lines
 * Only registers once per page load to avoid conflicts
 */
async function ensureIndicatorsRegistered() {
  if (typeof window === 'undefined') return;

  // Check if already registered
  const registry = (window as any).__feelsIndicatorsRegistered;
  if (registry) {
    return;
  }

  const { registerIndicator } = await import('klinecharts');

  // Register a combined indicator that shows both Floor and GTWAP
  registerIndicator({
    name: 'FEELS_OVERLAY',
    shortName: 'Feels',
    calcParams: [],
    figures: [
      {
        key: 'floorLine',
        title: 'Floor: ',
        type: 'line',
        styles: (data: any) => ({
          color: '#3B82F6',
          size: 2,
          lineWidth: 2,
          solid: true,
        }),
      },
      {
        key: 'gtwapLine',
        title: 'GTWAP: ',
        type: 'line',
        styles: (data: any) => ({
          color: '#5cca39',
          size: 2,
          lineWidth: 2,
          solid: true,
        }),
      },
    ],
    precision: 4,
    shouldOhlc: false,
    shouldFormatBigNumber: true,
    visible: true,
    minValue: null,
    maxValue: null,
    calc: (kLineDataList: any[], indicator: any) => {
      const floorData = (window as any).__floorPriceData || new Map();
      const gtwapData = (window as any).__gtwapPriceData || new Map();
      const showFloor = (window as any).__showFloorIndicator ?? true;
      const showGtwap = (window as any).__showGtwapIndicator ?? true;
      
      const results = kLineDataList.map((kLineData: any) => {
        const floorValue = showFloor ? floorData.get(kLineData.timestamp) : null;
        const gtwapValue = showGtwap ? gtwapData.get(kLineData.timestamp) : null;
        return { 
          floorLine: floorValue ?? null,
          gtwapLine: gtwapValue ?? null
        };
      });
      
      // Debug logging - only log first calculation
      if (!(window as any).__overlayCalcLogged) {
        const firstKline = kLineDataList[0];
        console.log('[Overlay Indicator] Calc called:', {
          klineCount: kLineDataList.length,
          floorDataSize: floorData.size,
          gtwapDataSize: gtwapData.size,
          showFloor,
          showGtwap,
          firstKlineTimestamp: firstKline?.timestamp,
          firstResult: results[0],
          hasFloorValues: results.some(r => r.floorLine !== null),
          hasGtwapValues: results.some(r => r.gtwapLine !== null)
        });
        (window as any).__overlayCalcLogged = true;
      }
      
      return results;
    },
  });

  // Mark as registered
  (window as any).__feelsIndicatorsRegistered = true;
}

// ============================================================================
// MAIN HOOK
// ============================================================================

export function useChartAdapter({
  container,
  timezone,
  priceAxisType,
  formatDate,
  formatBigNumber,
}: UseChartAdapterParams): UseChartAdapterResult {
  // --------------------------------------------------------------------------
  // State & Refs
  // --------------------------------------------------------------------------
  
  // Core chart instance and readiness state
  const chartRef = useRef<KLineChart | null>(null);
  const [isReady, setIsReady] = useState(false);

  // Store formatters in refs to prevent unnecessary re-initialization
  const formatDateRef = useRef(formatDate);
  const formatBigNumberRef = useRef(formatBigNumber);

  // Configuration state management
  const baseStylesRef = useRef<StylesLike | null>(null);  // Original styles from chart init
  const defaultConfigRef = useRef<ChartConfigState>({      // Default configuration
    axisType: 'normal',
    lastPriceVisible: true,
    crosshairVisible: true,
    data: [],
    showFloor: true,    // Changed to match PriceChart initial state
    showGtwap: true,    // Changed to match PriceChart initial state
    floorData: new Map(),
    gtwapData: new Map(),
  });
  const overridesRef = useRef<ChartConfigOverrides>({});   // User-specified overrides
  const indicatorStateRef = useRef<IndicatorState>({});    // Track indicator IDs

  // --------------------------------------------------------------------------
  // Initialization Effects
  // --------------------------------------------------------------------------

  // Update formatter refs when props change
  useEffect(() => {
    formatDateRef.current = formatDate;
  }, [formatDate]);

  useEffect(() => {
    formatBigNumberRef.current = formatBigNumber;
  }, [formatBigNumber]);

  // Register custom indicators on mount
  useEffect(() => {
    // Reset debug flags
    (window as any).__floorCalcLogged = false;
    (window as any).__gtwapCalcLogged = false;
    (window as any).__overlayCalcLogged = false;
    
    ensureIndicatorsRegistered().catch((error) => {
      console.error('[useChartAdapter] Failed to register indicators', error);
    });
  }, []);


  // --------------------------------------------------------------------------
  // Internal Helper Functions
  // --------------------------------------------------------------------------

  /**
   * Recalculates Y-axis range for normal axis type to fit all visible data
   * Including both candles and overlay indicators
   */
  const recalcYAxisRange = useCallback((config?: ChartConfigState) => {
    const chart = chartRef.current;
    if (!chart) return;

    const merged = config ?? mergeConfigs(defaultConfigRef.current, overridesRef.current);
    if (merged.axisType !== 'normal') {
      return;
    }

    const visibleRange = chart.getVisibleRange?.();
    if (!visibleRange) {
      return;
    }

    const dataList = chart.getDataList?.() ?? [];
    let minPrice = Number.POSITIVE_INFINITY;
    let maxPrice = Number.NEGATIVE_INFINITY;

    dataList.forEach((candle: any) => {
      if (!candle) return;
      if (candle.timestamp >= visibleRange.from && candle.timestamp <= visibleRange.to) {
        minPrice = Math.min(minPrice, candle.low ?? candle.close ?? candle.open);
        maxPrice = Math.max(maxPrice, candle.high ?? candle.close ?? candle.open);
        
        // Include floor price if visible
        if (merged.showFloor) {
          const floorValue = merged.floorData.get(candle.timestamp);
          if (floorValue !== undefined) {
            minPrice = Math.min(minPrice, floorValue);
            maxPrice = Math.max(maxPrice, floorValue);
          }
        }
        
        // Include GTWAP price if visible
        if (merged.showGtwap) {
          const gtwapValue = merged.gtwapData.get(candle.timestamp);
          if (gtwapValue !== undefined) {
            minPrice = Math.min(minPrice, gtwapValue);
            maxPrice = Math.max(maxPrice, gtwapValue);
          }
        }
      }
    });

    if (!Number.isFinite(minPrice) || !Number.isFinite(maxPrice)) {
      return;
    }

    if (minPrice === maxPrice) {
      const offset = Math.abs(minPrice) * 0.01 || 1;
      minPrice -= offset;
      maxPrice += offset;
    }

    const padding = Math.max((maxPrice - minPrice) * 0.1, 0.01);
    const lower = minPrice - padding;
    const upper = maxPrice + padding;

    const panes = (chart as any).getPanes?.() ?? (chart as any)._chartPane?._panes ?? [];
    const candlePane = panes.find(
      (pane: any) => pane?.id === 'candle_pane' || pane?._id === 'candle_pane'
    );
    const axis = candlePane?.getAxisComponent?.() ?? candlePane?._axis ?? candlePane?.axis;

    try {
      if (axis?.setExtremum) {
        axis.setExtremum(lower, upper);
        return;
      }
      if (axis?.setRange) {
        axis.setRange({ from: lower, to: upper });
        return;
      }
    } catch (error) {
      console.warn('[useChartAdapter] Failed to set axis extremum via API', error);
    }

    try {
      chart?.setPaneOptions?.({
        id: 'candle_pane',
        axis: {
          range: { from: lower, to: upper },
        },
      } as any);
    } catch (error) {
      console.warn('[useChartAdapter] Failed to set pane options for range', error);
    }
  }, []);


  /**
   * Core function that applies the merged configuration to the chart
   * This handles axis type, overlays, visibility settings, and styles
   * This should be pure and idempotent - same config always produces same result
   */
  const applyConfiguration = useCallback(async () => {
    const chart = chartRef.current;
    if (!chart) return;

    const merged = mergeConfigs(defaultConfigRef.current, overridesRef.current);
    const axisName = AXIS_NAME[merged.axisType];

    try {
      chart.setPaneOptions?.({
        id: 'candle_pane',
        axis: { name: axisName },
      } as any);
    } catch (error) {
      console.warn('[useChartAdapter] setPaneOptions axis failed', error);
    }

    const { LineType } = await import('klinecharts');
    const baseStyles = baseStylesRef.current ?? cloneStyles(chart.getStyles?.() ?? {});
    const chartData = merged.data.length > 0 ? merged.data : (chart.getDataList() as KLineData[]);
    const configForStyles: ChartConfigState = {
      axisType: merged.axisType,
      lastPriceVisible: merged.lastPriceVisible,
      crosshairVisible: merged.crosshairVisible,
      data: chartData,
      showFloor: merged.showFloor,
      showGtwap: merged.showGtwap,
      floorData: merged.floorData,
      gtwapData: merged.gtwapData,
    };

    const stylesToApply = buildStyles(baseStyles, configForStyles, LineType);
    chart.setStyles(stylesToApply as any);

    // Update global data and visibility flags for the combined indicator
    (window as any).__floorPriceData = merged.floorData;
    (window as any).__gtwapPriceData = merged.gtwapData;
    (window as any).__showFloorIndicator = merged.showFloor;
    (window as any).__showGtwapIndicator = merged.showGtwap;
    
    console.log('[Chart] Applying configuration:', {
      floorDataSize: merged.floorData.size,
      gtwapDataSize: merged.gtwapData.size,
      showFloor: merged.showFloor,
      showGtwap: merged.showGtwap
    });

    // Get current data before any modifications
    const currentData = chart.getDataList();
    const hasData = currentData && currentData.length > 0;
    
    // Check if we need the overlay indicator
    const needsOverlay = (merged.showFloor && merged.floorData.size > 0) || 
                        (merged.showGtwap && merged.gtwapData.size > 0);
    const hasOverlay = !!indicatorStateRef.current.overlayIndicatorId;
    
    // Remove old individual indicators if they exist (cleanup from old approach)
    if (indicatorStateRef.current.floorIndicatorId) {
      try {
        chart.removeIndicator({ id: indicatorStateRef.current.floorIndicatorId });
        indicatorStateRef.current.floorIndicatorId = undefined;
      } catch (error) {
        // Ignore errors from cleanup
      }
    }
    
    if (indicatorStateRef.current.gtwapIndicatorId) {
      try {
        chart.removeIndicator({ id: indicatorStateRef.current.gtwapIndicatorId });
        indicatorStateRef.current.gtwapIndicatorId = undefined;
      } catch (error) {
        // Ignore errors from cleanup
      }
    }
    
    // Manage the combined overlay indicator
    if (!needsOverlay && hasOverlay) {
      // Remove overlay if not needed
      try {
        chart.removeIndicator({ id: indicatorStateRef.current.overlayIndicatorId });
        indicatorStateRef.current.overlayIndicatorId = undefined;
        console.log('[Chart] Removed overlay indicator');
      } catch (error) {
        console.warn('[Chart] Failed to remove overlay indicator:', error);
      }
    } else if (needsOverlay && !hasOverlay) {
      // Create overlay if needed
      try {
        const id = chart.createIndicator({
          name: 'FEELS_OVERLAY',
          id: 'overlay_indicator',
        }, false, { id: 'candle_pane' });
        
        if (id) {
          indicatorStateRef.current.overlayIndicatorId = id;
          console.log('[Chart] Created overlay indicator:', id);
        }
      } catch (error) {
        console.error('[Chart] Failed to create overlay indicator:', error);
      }
    }
    
    // Always refresh data if we have any to ensure indicator updates
    if (hasData && (needsOverlay || hasOverlay)) {
      console.log('[Chart] Refreshing data after configuration');
      chart.applyNewData(currentData as any, false);
    }

    if (merged.axisType === 'normal') {
      recalcYAxisRange(merged);
    }
  }, [recalcYAxisRange]);


  /**
   * Initializes the KLineChart instance with base configuration
   * Sets up all initial styles, formatters, and chart behavior
   */
  const initChart = useCallback(async () => {
    if (!container || typeof window === 'undefined') {
      return;
    }

    // Prevent multiple initializations
    if (chartRef.current) {
      console.warn('[useChartAdapter] Chart already initialized, skipping');
      return;
    }

    // Ensure indicators are registered before creating the chart
    await ensureIndicatorsRegistered();

    const { init, DomPosition, LineType, CandleType, TooltipShowRule, TooltipShowType, dispose } =
      await import('klinecharts');

    const chart = init(container, {
      locale: 'en-US',
      timezone,
    });

    chartRef.current = chart;
    if (!chart) return;

    // Expose chart instance on container for debugging
    (container as any).__chart__ = chart;

    chart.setCustomApi({
      formatDate: formatDateRef.current,
      formatBigNumber: formatBigNumberRef.current,
    });
    chart.setZoomEnabled(true);
    chart.setScrollEnabled(true);
    chart.setBarSpace(8);
    chart.setOffsetRightDistance(80);

    const plotPane = chart.getDom('candle_pane', DomPosition.Main);
    if (plotPane) {
      plotPane.style.backgroundColor = PLOT_BACKGROUND;
    }

    chart.setStyles({
      grid: {
        horizontal: { show: true, size: 1, color: CHART_COLORS.grid, style: LineType.Solid },
        vertical: { show: true, size: 1, color: CHART_COLORS.grid, style: LineType.Solid },
      },
      candle: {
        type: CandleType.CandleSolid,
        bar: {
          upColor: CHART_COLORS.up,
          downColor: CHART_COLORS.down,
          noChangeColor: CHART_COLORS.neutral,
          upBorderColor: CHART_COLORS.up,
          downBorderColor: CHART_COLORS.down,
          noChangeBorderColor: CHART_COLORS.neutral,
          upWickColor: CHART_COLORS.up,
          downWickColor: CHART_COLORS.down,
          noChangeWickColor: CHART_COLORS.neutral,
        },
        priceMark: createPriceMarkConfig(LineType),
        tooltip: {
          ...TOOLTIP_CONFIG,
          showRule: TooltipShowRule.Always,
          showType: TooltipShowType.Standard,
        },
      },
      xAxis: AXIS_CONFIG,
      yAxis: AXIS_CONFIG,
      crosshair: createCrosshairConfig(true, LineType),
    } as any);

    baseStylesRef.current = cloneStyles(chart.getStyles?.() ?? {});

    setIsReady(true);

    return dispose;
  }, [container]); // Only re-init when container changes

  useEffect(() => {
    let disposeFn: ((chart: any) => void) | undefined;
    initChart()
      .then((dispose) => {
        disposeFn = dispose;
      })
      .catch((error) => console.error('[useChartAdapter] Failed to init chart', error));

    return () => {
      const chart = chartRef.current;
      if (chart && disposeFn) {
        console.log('[useChartAdapter] Disposing chart');
        disposeFn(chart);
        chartRef.current = null;
        baseStylesRef.current = null;
        setIsReady(false);
      }
    };
  }, [initChart]);

  // --------------------------------------------------------------------------
  // Public API Methods
  // --------------------------------------------------------------------------

  /**
   * Applies new price data and indicators to the chart atomically
   * This is the primary entry point for chart updates
   */
  const applyChartData = useCallback(
    (config: {
      data: KLineData[];
      floor?: { visible: boolean; series: Array<{ timestamp: number; value: number }> };
      gtwap?: { visible: boolean; series: Array<{ timestamp: number; value: number }> };
    }) => {
      const chart = chartRef.current;
      if (!chart || config.data.length === 0) {
        return;
      }

      // Update all configuration atomically
      defaultConfigRef.current.data = config.data;
      
      // Always update both floor and gtwap data if provided
      if (config.floor) {
        const floorMap = new Map<number, number>();
        config.floor.series.forEach(point => {
          floorMap.set(point.timestamp, point.value);
        });
        defaultConfigRef.current.floorData = floorMap;
        defaultConfigRef.current.showFloor = config.floor.visible;
      }
      
      if (config.gtwap) {
        const gtwapMap = new Map<number, number>();
        config.gtwap.series.forEach(point => {
          gtwapMap.set(point.timestamp, point.value);
        });
        defaultConfigRef.current.gtwapData = gtwapMap;
        defaultConfigRef.current.showGtwap = config.gtwap.visible;
      }

      // Clear any overrides since we're setting new defaults
      overridesRef.current = {};

      // Apply new data to chart
      chart.applyNewData(config.data as any, false);
      
      // Apply configuration which will handle indicators declaratively
      void applyConfiguration();
    },
    [applyConfiguration]
  );

  /**
   * Resets chart view to show all available data
   */
  const resetVisibleRange = useCallback(() => {
    const chart = chartRef.current;
    if (!chart) return;

    const dataList = chart.getDataList();
    if (!dataList || dataList.length === 0) return;

    // Get the full time range of the data
    const firstTimestamp = dataList[0]?.timestamp;
    const lastTimestamp = dataList[dataList.length - 1]?.timestamp;

    if (firstTimestamp && lastTimestamp) {
      // Reset to show all data with some padding
      const timePadding = (lastTimestamp - firstTimestamp) * 0.02; // 2% padding

      try {
        // Try different methods to set visible range
        if (typeof (chart as any).setVisibleRange === 'function') {
          (chart as any).setVisibleRange({
            from: firstTimestamp - timePadding,
            to: lastTimestamp + timePadding,
          });
        } else if (typeof (chart as any).zoomAtCoordinate === 'function') {
          // Calculate zoom level to show all data
          const currentRange = chart.getVisibleRange?.();
          if (currentRange) {
            const currentSpan = currentRange.to - currentRange.from;
            const targetSpan = lastTimestamp - firstTimestamp + 2 * timePadding;
            const zoomScale = currentSpan / targetSpan;

            // Zoom at the center of the chart
            const chartDom = (chart as any).getDom?.('candle_pane');
            if (chartDom) {
              const centerX = chartDom.offsetWidth / 2;
              (chart as any).zoomAtCoordinate(zoomScale, { x: centerX, y: 0 });
            }
          }
        } else if (typeof (chart as any).setOffsetRightDistance === 'function') {
          // Reset scroll position to the end
          (chart as any).setOffsetRightDistance(80);
        }

        // Ensure Y-axis is recalculated for the full range
        setTimeout(() => recalcYAxisRange(), 100);
      } catch (error) {
        console.warn('[useChartAdapter] Could not reset visible range:', error);
      }
    }
  }, [recalcYAxisRange]);

  /**
   * Updates chart timezone
   */
  const applyTimezone = useCallback((tz: string) => {
    chartRef.current?.setTimezone(tz);
  }, []);

  /**
   * Changes Y-axis scaling type (normal, logarithm, percentage)
   */
  const applyAxisType = useCallback(
    async (type: AxisType) => {
      overridesRef.current.axisType = type;
      await applyConfiguration();

      const chart = chartRef.current;
      if (chart) {
        const current = chart.getDataList?.();
        if (current && current.length > 0) {
          chart.applyNewData(current as any, true);
        }
        chart.resize?.();
      }
    },
    [applyConfiguration]
  );


  /**
   * Toggles visibility of the last price line
   */
  const setLastPriceVisibility = useCallback(
    async (visible: boolean) => {
      overridesRef.current.lastPriceVisible = visible;
      await applyConfiguration();
    },
    [applyConfiguration]
  );

  /**
   * Toggles visibility of the crosshair cursor
   */
  const setCrosshairVisibility = useCallback(
    async (visible: boolean) => {
      overridesRef.current.crosshairVisible = visible;
      await applyConfiguration();
    },
    [applyConfiguration]
  );

  /**
   * Toggles visibility of the floor indicator
   */
  const setFloorVisibility = useCallback(
    async (visible: boolean) => {
      // Update visibility in default config to ensure it persists
      defaultConfigRef.current.showFloor = visible;
      // Clear any override
      if (overridesRef.current.showFloor !== undefined) {
        delete overridesRef.current.showFloor;
      }
      await applyConfiguration();
    },
    [applyConfiguration]
  );

  /**
   * Toggles visibility of the GTWAP indicator
   */
  const setGtwapVisibility = useCallback(
    async (visible: boolean) => {
      // Update visibility in default config to ensure it persists
      defaultConfigRef.current.showGtwap = visible;
      // Clear any override
      if (overridesRef.current.showGtwap !== undefined) {
        delete overridesRef.current.showGtwap;
      }
      await applyConfiguration();
    },
    [applyConfiguration]
  );

  // --------------------------------------------------------------------------
  // Configuration Sync Effects
  // --------------------------------------------------------------------------

  // Apply configuration when chart becomes ready
  useEffect(() => {
    if (!isReady) return;
    void applyConfiguration();
  }, [applyConfiguration, isReady]);

  // Sync timezone changes
  useEffect(() => {
    if (!isReady) return;
    applyTimezone(timezone);
  }, [applyTimezone, isReady, timezone]);

  // Sync axis type changes
  useEffect(() => {
    if (!isReady) return;
    applyAxisType(priceAxisType).catch((error) => {
      console.error('[useChartAdapter] Failed to apply axis type effect', error);
    });
  }, [applyAxisType, isReady, priceAxisType]);

  // --------------------------------------------------------------------------
  // Return Public API
  // --------------------------------------------------------------------------

  return {
    chart: chartRef.current,
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
