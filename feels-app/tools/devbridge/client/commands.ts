'use client';

import { useRouter } from 'next/navigation';

type Handler = (args?: any) => Promise<any> | any;

// Feature flags store (example)
const featureFlags = new Map<string, boolean>();

// Built-in command handlers
export function setupBuiltinCommands(
  router: ReturnType<typeof useRouter>,
  registerCommand: (name: string, handler: Handler) => void
) {
  // Ping command for testing
  registerCommand('ping', async () => {
    return { pong: true, timestamp: Date.now() };
  });

  // Toggle feature flag
  registerCommand('toggleFlag', ({ name }: { name: string }) => {
    if (!name) {
      throw new Error('Flag name required');
    }
    const current = featureFlags.get(name) || false;
    featureFlags.set(name, !current);
    return { flag: name, enabled: !current };
  });

  // Get all feature flags
  registerCommand('getFlags', () => {
    const flags: Record<string, boolean> = {};
    featureFlags.forEach((value, key) => {
      flags[key] = value;
    });
    return flags;
  });

  // Navigate to route
  registerCommand('navigate', ({ path }: { path: string }) => {
    if (!path) {
      throw new Error('Path required');
    }
    router.push(path);
    return { navigated: path };
  });

  // Refresh current route
  registerCommand('refresh', () => {
    router.refresh();
    return { refreshed: true };
  });

  // Get current pathname
  registerCommand('getPath', () => {
    return { path: window.location.pathname };
  });

  // Get app info
  registerCommand('appInfo', () => {
    return {
      name: 'Feels App',
      version: process.env['NEXT_PUBLIC_APP_VERSION'] || '1.0.0',
      env: process.env.NODE_ENV,
      timestamp: Date.now()
    };
  });

  // Clear local storage
  registerCommand('clearStorage', () => {
    localStorage.clear();
    sessionStorage.clear();
    return { cleared: true };
  });

  // Get storage info
  registerCommand('storageInfo', () => {
    return {
      localStorage: {
        keys: Object.keys(localStorage),
        size: Object.keys(localStorage).length
      },
      sessionStorage: {
        keys: Object.keys(sessionStorage),
        size: Object.keys(sessionStorage).length
      }
    };
  });

  // Trigger a test event
  registerCommand('testEvent', ({ message }: { message?: string }) => {
    window.dispatchEvent(new CustomEvent('devbridge:test', {
      detail: { message: message || 'Test event triggered' }
    }));
    return { eventTriggered: true, message };
  });

  // Get window dimensions
  registerCommand('windowInfo', () => {
    return {
      innerWidth: window.innerWidth,
      innerHeight: window.innerHeight,
      outerWidth: window.outerWidth,
      outerHeight: window.outerHeight,
      devicePixelRatio: window.devicePixelRatio,
      screenWidth: window.screen.width,
      screenHeight: window.screen.height
    };
  });

  // Simulate wallet connection (for testing)
  registerCommand('simulateWallet', ({ connected }: { connected: boolean }) => {
    window.dispatchEvent(new CustomEvent('devbridge:wallet', {
      detail: { connected }
    }));
    return { walletSimulation: connected };
  });

  // Get performance metrics
  registerCommand('perfMetrics', () => {
    const nav = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming;
    return {
      domContentLoaded: nav?.domContentLoadedEventEnd - nav?.domContentLoadedEventStart,
      loadComplete: nav?.loadEventEnd - nav?.loadEventStart,
      responseTime: nav?.responseEnd - nav?.fetchStart,
      renderTime: nav?.domComplete - nav?.domInteractive
    };
  });

  // Console log levels control
  registerCommand('setLogLevel', ({ level }: { level: 'all' | 'warn' | 'error' | 'none' }) => {
    // This would integrate with your logging system
    return { logLevel: level };
  });

  // Debug chart y-axis type
  registerCommand('setChartAxisType', ({ type }: { type: string }) => {
    if (typeof window !== 'undefined' && (window as any).__debugPriceChart) {
      (window as any).__debugPriceChart.setPriceAxisType(type);
      return { success: true, type };
    }
    return { error: 'Chart debug not available' };
  });

  // Get chart state
  registerCommand('getChartState', () => {
    if (typeof window !== 'undefined' && (window as any).__debugPriceChart) {
      return (window as any).__debugPriceChart.getState();
    }
    return { error: 'Chart debug not available' };
  });

  // Debug chart instance and available methods
  registerCommand('debugChart', async () => {
    // Find the chart container
    const chartContainer = document.querySelector('#kline-chart') as HTMLElement;
    if (!chartContainer) {
      return { error: 'Chart container not found' };
    }

    // Try to get chart instance from the element data
    const chartInstance = (chartContainer as any).__chart__ || (chartContainer as any).chart;
    
    if (!chartInstance) {
      // Try global klinecharts registry if available
      if ((window as any).klinecharts?.instances) {
        const instances = (window as any).klinecharts.instances;
        for (const [elem, chart] of instances) {
          if (elem === chartContainer) {
            const methods = Object.getOwnPropertyNames(Object.getPrototypeOf(chart))
              .filter(name => typeof chart[name] === 'function')
              .sort();
            return {
              found: true,
              instanceLocation: 'klinecharts.instances',
              methods,
              hasAdjustVisibleRange: methods.includes('adjustVisibleRange'),
              hasResetDataVisibleRange: methods.includes('resetDataVisibleRange'),
              hasZoomAtCoordinate: methods.includes('zoomAtCoordinate'),
              hasSetVisibleRange: methods.includes('setVisibleRange'),
              hasGetVisibleRange: methods.includes('getVisibleRange')
            };
          }
        }
      }
      return { error: 'Chart instance not found in any known location' };
    }

    const methods = Object.getOwnPropertyNames(Object.getPrototypeOf(chartInstance))
      .filter(name => typeof chartInstance[name] === 'function')
      .sort();

    return {
      found: true,
      instanceLocation: 'element property',
      methods,
      hasAdjustVisibleRange: methods.includes('adjustVisibleRange'),
      hasResetDataVisibleRange: methods.includes('resetDataVisibleRange'),
      hasZoomAtCoordinate: methods.includes('zoomAtCoordinate'),
      hasSetVisibleRange: methods.includes('setVisibleRange'),
      hasGetVisibleRange: methods.includes('getVisibleRange')
    };
  });

  // Force chart zoom recalculation
  registerCommand('recalcChartZoom', async () => {
    const chartContainer = document.querySelector('#kline-chart') as HTMLElement;
    if (!chartContainer) {
      return { error: 'Chart container not found' };
    }
    
    // Dispatch a custom event that can be listened to by the chart component
    window.dispatchEvent(new CustomEvent('devbridge:recalcChartZoom'));
    return { dispatched: true };
  });

  // Test overlay toggle functionality
  registerCommand('testOverlayToggle', async () => {
    const floorButton = document.querySelector('button[aria-label="Toggle floor price line"]') as HTMLButtonElement;
    const gtwapButton = document.querySelector('button[aria-label="Toggle GTWAP price line"]') as HTMLButtonElement;
    
    if (!floorButton || !gtwapButton) {
      return { error: 'Floor or GTWAP buttons not found' };
    }

    // Get initial state
    const initialFloorActive = floorButton.getAttribute('data-state') === 'checked';
    const initialGtwapActive = gtwapButton.getAttribute('data-state') === 'checked';

    // Toggle floor off if on, wait, then toggle back
    if (initialFloorActive) {
      console.log('[testOverlayToggle] Toggling floor OFF');
      floorButton.click();
      await new Promise(resolve => setTimeout(resolve, 500));
      console.log('[testOverlayToggle] Toggling floor ON');
      floorButton.click();
      await new Promise(resolve => setTimeout(resolve, 500));
    }

    // Toggle GTWAP off if on, wait, then toggle back  
    if (initialGtwapActive) {
      console.log('[testOverlayToggle] Toggling GTWAP OFF');
      gtwapButton.click();
      await new Promise(resolve => setTimeout(resolve, 500));
      console.log('[testOverlayToggle] Toggling GTWAP ON');
      gtwapButton.click();
      await new Promise(resolve => setTimeout(resolve, 500));
    }

    return {
      tested: true,
      initialFloorActive,
      initialGtwapActive,
      message: 'Check console logs and visual display to verify Y-axis recalculation'
    };
  });

  // Debug USD toggle and chart data
  registerCommand('debugUsdToggle', async () => {
    // Find USD button
    const usdButtons = Array.from(document.querySelectorAll('button')).filter(b => b.textContent?.includes('USD'));
    if (usdButtons.length === 0) {
      return { error: 'USD button not found' };
    }

    const usdButton = usdButtons[0];
    const chartContainer = document.querySelector('#kline-chart') as HTMLElement;
    const chartInstance = (chartContainer as any).__chart__;

    // Get initial state
    const initialData = chartInstance?.getDataList ? chartInstance.getDataList() : null;
    const initialDataCount = initialData ? initialData.length : 0;
    const initialSampleData = initialData ? initialData.slice(0, 3) : [];
    console.log('[debugUsdToggle] Initial data count:', initialDataCount, 'Sample:', initialSampleData);

    // Click USD toggle
    console.log('[debugUsdToggle] Clicking USD toggle');
    if (usdButton) {
      usdButton.click();
    } else {
      console.log('[debugUsdToggle] USD button not found, skipping click');
    }

    // Wait for re-render
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Get data after toggle
    const afterData = chartInstance?.getDataList ? chartInstance.getDataList() : null;
    const afterDataCount = afterData ? afterData.length : 0;
    const afterSampleData = afterData ? afterData.slice(0, 3) : [];
    console.log('[debugUsdToggle] After toggle data count:', afterDataCount, 'Sample:', afterSampleData);

    return {
      buttonFound: true,
      initialDataCount,
      afterDataCount,
      dataDisappeared: afterDataCount === 0 && initialDataCount > 0,
      initialSampleData,
      afterSampleData,
      chartInstanceAvailable: !!chartInstance
    };
  });

  // Debug logarithmic axis
  registerCommand('debugLogAxis', async () => {
    const buttons = Array.from(document.querySelectorAll('button'));
    const axisButtons = buttons.filter((button) => {
      const text = button.textContent ?? '';
      return text.includes('Linear') || text.includes('Logarithmic') || text.includes('Percentage');
    });

    if (axisButtons.length === 0) {
      return { error: 'Axis dropdown button not found' };
    }

    const axisButton = axisButtons[0] as HTMLButtonElement;
    if (!axisButton) {
      return { error: 'Axis dropdown button not found after filter' };
    }

    const chartContainer = document.querySelector('#kline-chart') as HTMLElement | null;
    if (!chartContainer) {
      return { error: 'Chart container not found' };
    }

    const chartInstance = (chartContainer as any).__chart__;
    if (!chartInstance) {
      return { error: 'Chart instance not found' };
    }

    const getPaneOptions = chartInstance.getPaneOptions?.bind(chartInstance);
    const initialPaneOptions = getPaneOptions ? getPaneOptions('candle_pane') : null;
    const initialYAxisTicks = Array.from(document.querySelectorAll('.k-line-chart-y-axis-text')).map((el) =>
      el.textContent
    );

    const currentAxisType = axisButton.textContent?.trim();

    axisButton.click();
    await new Promise((resolve) => setTimeout(resolve, 200));

    const logOption = Array.from(document.querySelectorAll('[role="menuitem"]')).find((item) =>
      item.textContent?.includes('Logarithmic')
    ) as HTMLElement | undefined;

    if (!logOption) {
      return { error: 'Logarithmic option not found in dropdown' };
    }

    logOption.click();
    await new Promise((resolve) => setTimeout(resolve, 1500));

    const afterPaneOptions = getPaneOptions ? getPaneOptions('candle_pane') : null;
    const afterYAxisTicks = Array.from(document.querySelectorAll('.k-line-chart-y-axis-text')).map((el) =>
      el.textContent
    );

    const parseTickValues = (ticks: (string | null)[]) => {
      const values: number[] = [];
      for (const tick of ticks) {
        if (!tick) continue;
        const cleaned = tick.replace(/[$,%]/g, '');
        const numeric = Number.parseFloat(cleaned);
        if (Number.isFinite(numeric)) {
          values.push(numeric);
        }
      }
      return values;
    };

    const afterTickValues = parseTickValues(afterYAxisTicks);
    let isLogarithmic = false;

    if (afterTickValues.length >= 3) {
      const ratios: number[] = [];
      for (let i = 1; i < afterTickValues.length; i++) {
        const prev = afterTickValues[i - 1];
        const current = afterTickValues[i];
        if (prev !== undefined && prev !== 0 && current !== undefined) {
          ratios.push(current / prev);
        }
      }

      if (ratios.length > 0) {
        const avgRatio = ratios.reduce((sum, value) => sum + value, 0) / ratios.length;
        const variance = ratios.reduce((sum, value) => sum + Math.pow(value - avgRatio, 2), 0) / ratios.length;
        isLogarithmic = Number.isFinite(avgRatio) && variance < 0.1 && avgRatio > 1.5;
      }
    }

    const axisConfig = {
      styles: chartInstance.getStyles?.(),
      paneOptions: afterPaneOptions,
      axisType: afterPaneOptions?.axisOptions?.type || afterPaneOptions?.yAxis?.type || 'unknown',
    };

    return {
      dropdownFound: true,
      optionClicked: true,
      currentAxisType,
      initialYAxisTicks,
      afterYAxisTicks,
      afterTickValues,
      isLogarithmic,
      axisConfig,
      axisTypeChanged: initialPaneOptions?.yAxis?.type !== afterPaneOptions?.yAxis?.type,
      message: isLogarithmic ? 'Axis appears to be logarithmic' : 'Axis appears to be linear',
    };
  });

  // Get comprehensive chart debug info
  registerCommand('getChartDebugInfo', () => {
    const chartContainer = document.querySelector('#kline-chart') as HTMLElement;
    if (!chartContainer) {
      return { error: 'Chart container not found' };
    }

    const chartInstance = (chartContainer as any).__chart__;
    if (!chartInstance) {
      return { error: 'Chart instance not found' };
    }

    const debugInfo = {
      dataCount: chartInstance.getDataList ? chartInstance.getDataList().length : 0,
      paneOptions: chartInstance.getPaneOptions ? chartInstance.getPaneOptions('candle_pane') : null,
      visibleRange: chartInstance.getVisibleRange ? chartInstance.getVisibleRange() : null,
      styles: chartInstance.getStyles ? chartInstance.getStyles() : null,
      customApi: chartInstance.getCustomApi ? chartInstance.getCustomApi() : null,
      timezone: chartInstance.getTimezone ? chartInstance.getTimezone() : null
    };

    console.log('[getChartDebugInfo] Full debug info:', debugInfo);

    return debugInfo;
  });
}

// Export feature flags for app use
export function getFeatureFlag(name: string): boolean {
  return featureFlags.get(name) || false;
}

export function getAllFeatureFlags(): Record<string, boolean> {
  const flags: Record<string, boolean> = {};
  featureFlags.forEach((value, key) => {
    flags[key] = value;
  });
  return flags;
}