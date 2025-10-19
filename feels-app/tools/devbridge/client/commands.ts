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

  // Get current data source
  registerCommand('getDataSource', () => {
    // Try to access the data source context from the window
    const dataSource = (window as any).__dataSourceContext;
    return {
      dataSource: dataSource?.dataSource || 'unknown',
      isIndexerAvailable: dataSource?.isIndexerAvailable || false,
      indexerUrl: process.env['NEXT_PUBLIC_INDEXER_URL'],
      useIndexer: process.env['NEXT_PUBLIC_USE_INDEXER'],
      timestamp: Date.now()
    };
  });

  // Switch data source
  registerCommand('setDataSource', ({ source }: { source: 'test' | 'indexer' }) => {
    const dataSourceContext = (window as any).__dataSourceContext;
    if (!dataSourceContext) {
      return { error: 'Data source context not available' };
    }
    
    if (source !== 'test' && source !== 'indexer') {
      return { error: 'Invalid data source. Must be "test" or "indexer"' };
    }
    
    dataSourceContext.setDataSource(source);
    return {
      success: true,
      newDataSource: source,
      isIndexerAvailable: dataSourceContext.isIndexerAvailable,
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
  
  // Debug syntax highlighting
  registerCommand('debugSyntaxHighlight', () => {
    const codeBlocks = document.querySelectorAll('[data-rehype-pretty-code-figure] pre code, [data-rehype-pretty-code-fragment] pre code');
    const results: any[] = [];
    
    codeBlocks.forEach((block, i) => {
      const pre = block.parentElement;
      const figure = pre?.parentElement;
      const spans = block.querySelectorAll('span[style]');
      const computedStyle = window.getComputedStyle(block);
      
      // Get first few spans with inline styles
      const spanStyles: any[] = [];
      Array.from(spans).slice(0, 5).forEach(span => {
        const style = span.getAttribute('style');
        const computedColor = window.getComputedStyle(span).color;
        spanStyles.push({
          style,
          computedColor,
          textContent: span.textContent
        });
      });
      
      results.push({
        index: i,
        figureAttrs: figure?.attributes ? Array.from(figure.attributes).map(a => `${a.name}="${a.value}"`) : [],
        preAttrs: pre?.attributes ? Array.from(pre.attributes).map(a => `${a.name}="${a.value}"`) : [],
        codeAttrs: Array.from(block.attributes).map(a => `${a.name}="${a.value}"`),
        computedCodeColor: computedStyle.color,
        totalSpans: spans.length,
        sampleSpans: spanStyles,
        inheritedStyles: {
          fontFamily: computedStyle.fontFamily,
          fontSize: computedStyle.fontSize,
          lineHeight: computedStyle.lineHeight
        }
      });
    });
    
    console.log('[debugSyntaxHighlight] Results:', results);
    return {
      codeBlockCount: codeBlocks.length,
      results
    };
  });

  // Debug Mermaid diagrams
  registerCommand('debugMermaid', () => {
    const mermaidContainers = document.querySelectorAll('.mermaid-container, .mermaid');
    const results: any[] = [];
    
    mermaidContainers.forEach((container, i) => {
      const mermaidDiv = container.classList.contains('mermaid') ? container : container.querySelector('.mermaid');
      if (!mermaidDiv) return;
      
      const svg = mermaidDiv.querySelector('svg');
      if (!svg) return;
      
      // Get all text elements in the SVG
      const textElements = svg.querySelectorAll('text');
      const textAnalysis: any[] = [];
      
      textElements.forEach((text, idx) => {
        const computedStyle = window.getComputedStyle(text);
        const parentElement = text.parentElement;
        const bbox = text.getBBox ? text.getBBox() : null;
        
        textAnalysis.push({
          index: idx,
          textContent: text.textContent?.trim(),
          textAnchor: text.getAttribute('text-anchor') || computedStyle.textAnchor,
          dominantBaseline: text.getAttribute('dominant-baseline') || computedStyle.dominantBaseline,
          x: text.getAttribute('x'),
          y: text.getAttribute('y'),
          transform: text.getAttribute('transform'),
          parentTag: parentElement?.tagName,
          parentClass: parentElement?.getAttribute('class'),
          computedTextAnchor: computedStyle.textAnchor,
          computedTextAlign: computedStyle.textAlign,
          bbox: bbox ? { width: bbox.width, height: bbox.height, x: bbox.x, y: bbox.y } : null,
          style: text.getAttribute('style'),
          className: text.getAttribute('class')
        });
      });
      
      // Get node information
      const nodes = svg.querySelectorAll('.node, g.node, [class*="node"]');
      const nodeAnalysis: any[] = [];
      
      nodes.forEach((node, idx) => {
        const nodeText = node.querySelectorAll('text');
        const rect = node.querySelector('rect, circle, ellipse, polygon, path');
        const transform = node.getAttribute('transform');
        
        nodeAnalysis.push({
          index: idx,
          tag: node.tagName,
          className: node.getAttribute('class'),
          transform,
          textCount: nodeText.length,
          hasShape: !!rect,
          shapeType: rect?.tagName,
          shapeAttrs: rect ? {
            x: rect.getAttribute('x'),
            y: rect.getAttribute('y'),
            width: rect.getAttribute('width'),
            height: rect.getAttribute('height'),
            cx: rect.getAttribute('cx'),
            cy: rect.getAttribute('cy'),
            r: rect.getAttribute('r')
          } : null
        });
      });
      
      results.push({
        containerIndex: i,
        containerClass: container.className,
        mermaidClass: mermaidDiv.className,
        svgDimensions: {
          width: svg.getAttribute('width'),
          height: svg.getAttribute('height'),
          viewBox: svg.getAttribute('viewBox')
        },
        textElementCount: textElements.length,
        nodeCount: nodes.length,
        textAnalysis: textAnalysis.slice(0, 10), // First 10 text elements
        nodeAnalysis: nodeAnalysis.slice(0, 5), // First 5 nodes
        svgStyle: svg.getAttribute('style'),
        mermaidStyle: mermaidDiv.getAttribute('style')
      });
    });
    
    console.log('[debugMermaid] Mermaid analysis:', results);
    return {
      mermaidCount: mermaidContainers.length,
      results
    };
  });

  // Fix Mermaid text centering
  registerCommand('fixMermaidCentering', () => {
    const mermaidContainers = document.querySelectorAll('.mermaid-container, .mermaid');
    let fixedCount = 0;
    
    mermaidContainers.forEach((container) => {
      const mermaidDiv = container.classList.contains('mermaid') ? container : container.querySelector('.mermaid');
      if (!mermaidDiv) return;
      
      const svg = mermaidDiv.querySelector('svg');
      if (!svg) return;
      
      // Fix all text elements in nodes
      const textElements = svg.querySelectorAll('text');
      textElements.forEach((text) => {
        // Set text-anchor to middle for horizontal centering
        text.setAttribute('text-anchor', 'middle');
        text.style.textAnchor = 'middle';
        
        // Set dominant-baseline for vertical centering
        text.setAttribute('dominant-baseline', 'central');
        text.style.dominantBaseline = 'central';
        
        fixedCount++;
      });
    });
    
    return {
      mermaidContainers: mermaidContainers.length,
      textElementsFixed: fixedCount,
      message: `Fixed ${fixedCount} text elements across ${mermaidContainers.length} Mermaid diagrams`
    };
  });

  // Inspect edge labels specifically
  registerCommand('inspectEdgeLabels', () => {
    const mermaidContainers = document.querySelectorAll('.mermaid-container, .mermaid');
    const results: any[] = [];
    
    mermaidContainers.forEach((container, i) => {
      const mermaidDiv = container.classList.contains('mermaid') ? container : container.querySelector('.mermaid');
      if (!mermaidDiv) return;
      
      const svg = mermaidDiv.querySelector('svg');
      if (!svg) return;
      
      // Find all edge labels
      const edgeLabels = svg.querySelectorAll('[class*="edgeLabel"], .edgeLabel, g[class*="edgeLabel"]');
      const edgeLabelData: any[] = [];
      
      edgeLabels.forEach((label, idx) => {
        const computedStyle = window.getComputedStyle(label);
        const rect = label.querySelector('rect');
        const text = label.querySelector('text');
        const foreignObject = label.querySelector('foreignObject');
        
        edgeLabelData.push({
          index: idx,
          tagName: label.tagName,
          className: label.getAttribute('class'),
          id: label.getAttribute('id'),
          computedBackground: computedStyle.backgroundColor,
          computedFill: computedStyle.fill,
          hasRect: !!rect,
          rectFill: rect ? rect.getAttribute('fill') : null,
          rectStyle: rect ? rect.getAttribute('style') : null,
          rectComputedFill: rect ? window.getComputedStyle(rect).fill : null,
          hasText: !!text,
          textContent: text ? text.textContent : null,
          hasForeignObject: !!foreignObject,
          transform: label.getAttribute('transform'),
          style: label.getAttribute('style'),
          innerHTML: label.innerHTML.substring(0, 200)
        });
      });
      
      results.push({
        containerIndex: i,
        edgeLabelCount: edgeLabels.length,
        edgeLabelData: edgeLabelData
      });
    });
    
    console.log('[inspectEdgeLabels] Edge label analysis:', results);
    return {
      mermaidContainers: mermaidContainers.length,
      results
    };
  });

  // Fix edge label backgrounds directly
  registerCommand('fixEdgeLabels', () => {
    const mermaidContainers = document.querySelectorAll('.mermaid-container, .mermaid');
    let fixedCount = 0;
    
    mermaidContainers.forEach((container) => {
      const mermaidDiv = container.classList.contains('mermaid') ? container : container.querySelector('.mermaid');
      if (!mermaidDiv) return;
      
      const svg = mermaidDiv.querySelector('svg');
      if (!svg) return;
      
      // Find all edge labels
      const edgeLabels = svg.querySelectorAll('[class*="edgeLabel"], .edgeLabel, g[class*="edgeLabel"]');
      
      edgeLabels.forEach((label) => {
        // Remove background from the label itself
        if (label instanceof HTMLElement || label instanceof SVGElement) {
          (label as any).style.backgroundColor = 'transparent';
          (label as any).style.background = 'transparent';
        }
        
        // Find and hide background rectangles
        const rect = label.querySelector('rect');
        if (rect && rect instanceof SVGElement) {
          rect.style.fill = 'transparent';
          rect.style.stroke = 'none';
          rect.style.display = 'none';
          fixedCount++;
        }
        
        // Adjust text position slightly down
        const text = label.querySelector('text');
        if (text && text instanceof SVGElement) {
          text.style.transform = 'translateY(2px)';
        }
      });
    });
    
    return {
      mermaidContainers: mermaidContainers.length,
      edgeLabelsFixed: fixedCount,
      message: `Fixed ${fixedCount} edge label backgrounds`
    };
  });

  // Inspect page content for any code blocks or mermaid-related elements
  registerCommand('inspectPage', () => {
    const allPre = document.querySelectorAll('pre');
    const allCode = document.querySelectorAll('code');
    const allMermaid = document.querySelectorAll('[class*="mermaid"], [id*="mermaid"]');
    const allSvg = document.querySelectorAll('svg');
    
    const preAnalysis: any[] = [];
    allPre.forEach((pre, i) => {
      const code = pre.querySelector('code');
      const classes = pre.className;
      const dataLang = pre.getAttribute('data-language');
      const content = (code?.textContent || pre.textContent || '').substring(0, 100);
      
      preAnalysis.push({
        index: i,
        classes,
        dataLanguage: dataLang,
        hasCode: !!code,
        codeClasses: code?.className,
        contentPreview: content,
        isMermaid: content.includes('graph') || content.includes('flowchart') || classes.includes('mermaid') || dataLang === 'mermaid'
      });
    });
    
    const codeAnalysis: any[] = [];
    allCode.forEach((code, i) => {
      if (i < 10) { // First 10 only
        const parent = code.parentElement;
        const classes = code.className;
        const content = (code.textContent || '').substring(0, 100);
        
        codeAnalysis.push({
          index: i,
          classes,
          parentTag: parent?.tagName,
          parentClasses: parent?.className,
          contentPreview: content,
          isMermaid: content.includes('graph') || content.includes('flowchart') || classes.includes('mermaid')
        });
      }
    });
    
    return {
      preCount: allPre.length,
      codeCount: allCode.length,
      mermaidElements: allMermaid.length,
      svgCount: allSvg.length,
      preAnalysis,
      codeAnalysis: codeAnalysis,
      currentUrl: window.location.href,
      title: document.title
    };
  });

  // Debug vanity address mining
  registerCommand('debugVanityMiner', () => {
    try {
      // Check if VanityAddressContext is available
      const vanityStatus = (window as any).__vanityMinerStatus;
      
      // Check if worker is running
      const workerStatus = {
        workerExists: !!(window as any).__vanityMinerWorker,
        isRunning: vanityStatus?.isRunning,
        attempts: vanityStatus?.attempts,
        elapsedMs: vanityStatus?.elapsedMs,
        hasKeypair: !!vanityStatus?.keypair,
        error: vanityStatus?.error
      };

      // Check localStorage
      const storedKeypair = localStorage.getItem('vanityKeypair');
      const hasStoredKeypair = !!storedKeypair;
      
      // Check WASM support
      const wasmSupported = typeof WebAssembly !== 'undefined';
      const sharedArrayBufferSupported = typeof SharedArrayBuffer !== 'undefined';
      
      return {
        workerStatus,
        hasStoredKeypair,
        wasmSupported,
        sharedArrayBufferSupported,
        browserInfo: {
          userAgent: navigator.userAgent,
          hardwareConcurrency: navigator.hardwareConcurrency
        }
      };
    } catch (error) {
      return {
        error: 'Failed to get vanity miner status',
        details: error instanceof Error ? error.message : String(error)
      };
    }
  });

  // Get vanity miner performance metrics
  registerCommand('getVanityPerformance', () => {
    const status = (window as any).__vanityMinerStatus;
    if (!status || !status.attempts || !status.elapsedMs) {
      return { error: 'No mining in progress or no data available' };
    }
    
    const attemptsPerSecond = (status.attempts / (status.elapsedMs / 1000)).toFixed(2);
    const estimatedTimeForMatch = (1 / (1 / Math.pow(58, 4))) / parseFloat(attemptsPerSecond);
    
    return {
      attempts: status.attempts,
      elapsedSeconds: (status.elapsedMs / 1000).toFixed(2),
      attemptsPerSecond,
      estimatedSecondsToFind: estimatedTimeForMatch.toFixed(0),
      estimatedMinutesToFind: (estimatedTimeForMatch / 60).toFixed(2)
    };
  });

  // Start/stop vanity miner
  registerCommand('controlVanityMiner', ({ action }: { action: 'start' | 'stop' | 'reset' }) => {
    const minerControl = (window as any).__vanityMinerControl;
    if (!minerControl) {
      return { error: 'Vanity miner control not available' };
    }
    
    switch (action) {
      case 'start':
        minerControl.startMining();
        return { message: 'Mining started' };
      case 'stop':
        minerControl.stopMining();
        return { message: 'Mining stopped' };
      case 'reset':
        minerControl.resetAndMine();
        return { message: 'Mining reset and restarted' };
      default:
        return { error: 'Invalid action' };
    }
  });
  
  // Clear vanity localStorage
  registerCommand('clearVanityStorage', () => {
    const before = localStorage.getItem('vanityKeypair');
    localStorage.removeItem('vanityKeypair');
    return { 
      cleared: true, 
      hadStoredKeypair: !!before,
      message: 'Vanity keypair cleared from localStorage'
    };
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