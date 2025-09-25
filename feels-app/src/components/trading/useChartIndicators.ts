// Custom indicator implementation for floor and GTWAP lines that properly affect Y-axis range
import { useCallback, useRef } from 'react';
import type { Chart as KLineChart } from 'klinecharts';

interface UseChartIndicatorsResult {
  createFloorIndicator: (points: { timestamp: number; value: number }[]) => void;
  removeFloorIndicator: () => void;
  createGTWAPIndicator: (points: { timestamp: number; value: number }[]) => void;
  removeGTWAPIndicator: () => void;
}

// Store the data globally so indicators can access it
const floorDataMap = new Map<number, number>();
const gtwapDataMap = new Map<number, number>();
let showFloor = false;
let showGtwap = false;

// Register custom indicators on first load
let indicatorsRegistered = false;

async function ensureIndicatorsRegistered() {
  if (indicatorsRegistered || typeof window === 'undefined') {
    return;
  }

  const { registerIndicator } = await import('klinecharts');

  // Register a combined indicator that can show both Floor and GTWAP
  registerIndicator({
    name: 'FLOOR_GTWAP',
    shortName: 'Floor/GTWAP',
    calcParams: [],
    figures: [
      {
        key: 'floor',
        title: 'Floor Price',
        type: 'line',
        styles: () => ({
          color: '#3B82F6',
          size: 2
        })
      },
      {
        key: 'gtwap',
        title: 'GTWAP',
        type: 'line',
        styles: () => ({
          color: '#5cca39',
          size: 2
        })
      }
    ],
    calc: (kLineDataList: any[]) => {
      return kLineDataList.map(kLineData => {
        const floorValue = showFloor ? floorDataMap.get(kLineData.timestamp) : null;
        const gtwapValue = showGtwap ? gtwapDataMap.get(kLineData.timestamp) : null;
        return {
          floor: floorValue ?? null,
          gtwap: gtwapValue ?? null
        };
      });
    }
  });

  indicatorsRegistered = true;
}

export function useChartIndicators(chartRef: React.RefObject<KLineChart | null>): UseChartIndicatorsResult {
  const combinedIndicatorId = useRef<string | null>(null);
  const isInitialized = useRef(false);

  const updateCombinedIndicator = useCallback(async () => {
    const chart = chartRef.current;
    if (!chart) return;

    await ensureIndicatorsRegistered();

    // If indicator doesn't exist yet and we need to show something, create it
    if (!combinedIndicatorId.current && (showFloor || showGtwap)) {
      const id = chart.createIndicator({
        name: 'FLOOR_GTWAP',
        id: 'floor_gtwap_indicator'
      }, false, { id: 'candle_pane' });

      if (id) {
        combinedIndicatorId.current = id;
        isInitialized.current = true;
        console.log(`[useChartIndicators] Combined indicator created - Floor: ${showFloor}, GTWAP: ${showGtwap}`);
      }
    }
    
    // If indicator exists but both are hidden, remove it
    else if (combinedIndicatorId.current && !showFloor && !showGtwap) {
      try {
        chart.removeIndicator({ id: combinedIndicatorId.current });
        combinedIndicatorId.current = null;
        isInitialized.current = false;
        console.log('[useChartIndicators] Combined indicator removed - both lines hidden');
      } catch (e) {
        // Ignore errors
      }
    }
    
    // If indicator exists and at least one line should be shown, force recreation
    else if (combinedIndicatorId.current && (showFloor || showGtwap)) {
      console.log(`[useChartIndicators] Recreating indicator - Floor: ${showFloor}, GTWAP: ${showGtwap}`);
      
      // Remove existing indicator
      try {
        chart.removeIndicator({ id: combinedIndicatorId.current });
      } catch (e) {
        // Ignore errors
      }
      
      // Recreate it with updated data
      const id = chart.createIndicator({
        name: 'FLOOR_GTWAP',
        id: 'floor_gtwap_indicator'
      }, false, { id: 'candle_pane' });
      
      if (id) {
        combinedIndicatorId.current = id;
      }
    }
  }, [chartRef]);

  const createFloorIndicator = useCallback(async (points: { timestamp: number; value: number }[]) => {
    if (points.length === 0) return;

    // Check if we're actually changing anything
    const wasShowing = showFloor;
    const hadSameData = floorDataMap.size === points.length && 
      points.every(p => floorDataMap.get(p.timestamp) === p.value);

    if (wasShowing && hadSameData) {
      console.log('[useChartIndicators] Floor data unchanged, skipping update');
      return;
    }

    // Update the global data map
    floorDataMap.clear();
    points.forEach(point => {
      floorDataMap.set(point.timestamp, point.value);
    });

    showFloor = true;
    console.log('[useChartIndicators] Floor data updated with', points.length, 'points');
    
    // Update the combined indicator
    await updateCombinedIndicator();
  }, [updateCombinedIndicator]);

  const removeFloorIndicator = useCallback(async () => {
    if (!showFloor) {
      console.log('[useChartIndicators] Floor already hidden, skipping update');
      return;
    }

    // Don't clear the data map, just set the flag
    showFloor = false;
    console.log('[useChartIndicators] Floor visibility set to false');
    
    // Update the combined indicator
    await updateCombinedIndicator();
  }, [updateCombinedIndicator]);

  const createGTWAPIndicator = useCallback(async (points: { timestamp: number; value: number }[]) => {
    if (points.length === 0) return;

    // Check if we're actually changing anything
    const wasShowing = showGtwap;
    const hadSameData = gtwapDataMap.size === points.length && 
      points.every(p => gtwapDataMap.get(p.timestamp) === p.value);

    if (wasShowing && hadSameData) {
      console.log('[useChartIndicators] GTWAP data unchanged, skipping update');
      return;
    }

    // Update the global data map
    gtwapDataMap.clear();
    points.forEach(point => {
      gtwapDataMap.set(point.timestamp, point.value);
    });

    showGtwap = true;
    console.log('[useChartIndicators] GTWAP data updated with', points.length, 'points');
    
    // Update the combined indicator
    await updateCombinedIndicator();
  }, [updateCombinedIndicator]);

  const removeGTWAPIndicator = useCallback(async () => {
    if (!showGtwap) {
      console.log('[useChartIndicators] GTWAP already hidden, skipping update');
      return;
    }

    // Don't clear the data map, just set the flag
    showGtwap = false;
    console.log('[useChartIndicators] GTWAP visibility set to false');
    
    // Update the combined indicator
    await updateCombinedIndicator();
  }, [updateCombinedIndicator]);

  return {
    createFloorIndicator,
    removeFloorIndicator,
    createGTWAPIndicator,
    removeGTWAPIndicator
  };
}