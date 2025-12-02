// Multi-token chart component showing GTWAP and Floor lines for all tokens on splash page
'use client';

import React, { useCallback, useEffect, useRef, useState, useMemo } from 'react';
import Image from 'next/image';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { TokenSearchResult, TRENDING_TOKEN_ADDRESSES } from '@/utils/token-search';

interface MultiTokenChartProps {
  tokens: TokenSearchResult[];
  timeRange?: string;
}

// Token colors - each token gets a unique color for both GTWAP and Floor
const TOKEN_COLORS = [
  '#3B82F6', // blue
  '#10B981', // green
  '#F59E0B', // amber
  '#EF4444', // red
  '#8B5CF6', // purple
  '#EC4899', // pink
  '#14B8A6', // teal
  '#F97316', // orange
];

export function MultiTokenChart({ tokens, timeRange: _timeRange = '1D' }: MultiTokenChartProps) {
  const [chartContainer, setChartContainer] = useState<HTMLDivElement | null>(null);
  const chartRef = useRef<any>(null);
  const [isReady, setIsReady] = useState(false);
  const registeredRef = useRef(false);
  const initializingRef = useRef(false);
  
  // Token visibility state - all tokens visible by default
  const [tokenVisibility, setTokenVisibility] = useState<Record<string, boolean>>({});

  // Filter to only show trending tokens (max 6)
  const trendingTokens = useMemo(() => {
    return tokens
      .filter(token => TRENDING_TOKEN_ADDRESSES.includes(token.address))
      .slice(0, 6);
  }, [tokens]);

  // Initialize token visibility when trending tokens change
  useEffect(() => {
    const initialVisibility: Record<string, boolean> = {};
    trendingTokens.forEach(token => {
      initialVisibility[token.address] = true; // All tokens visible by default
    });
    setTokenVisibility(initialVisibility);
  }, [trendingTokens]);

  // Store trending tokens in ref for chart initialization
  const trendingTokensRef = useRef(trendingTokens);
  useEffect(() => {
    trendingTokensRef.current = trendingTokens;
  }, [trendingTokens]);

  // Initialize chart with line-only view (no candlesticks)
  const initChart = useCallback(async () => {
    if (!chartContainer) return;
    if (chartRef.current) return; // Prevent re-initialization
    if (initializingRef.current) return; // Prevent concurrent initialization

    initializingRef.current = true;

    try {
      const { init, registerIndicator, dispose } = await import('klinecharts');

      // Clean up any existing chart instance first
      const existingCharts = chartContainer.querySelectorAll('canvas');
      if (existingCharts.length > 0) {
        dispose(chartContainer);
      }

      // Generate data BEFORE creating the chart so indicators have data to display
      const now = Date.now();
      const dataPoints = 96;
      const interval = 15 * 60 * 1000; // 15 minutes

      // Generate GTWAP and Floor data for each token
      const currentTrendingTokens = trendingTokensRef.current;
      console.log('Generating data for tokens:', currentTrendingTokens.map(t => t.symbol));
      
      // All tokens start at the same base price for comparison
      const uniformStartPrice = 1.0;
      
      // Define different volatility profiles and performance for each token
      const tokenProfiles = [
        { volatility: 0.08, performance: 1.5 },  // High volatility, good performance
        { volatility: 0.12, performance: 2.2 },  // Very high volatility, excellent performance  
        { volatility: 0.04, performance: 0.8 },  // Low volatility, poor performance
        { volatility: 0.06, performance: 1.1 },  // Medium volatility, slight gain
        { volatility: 0.10, performance: 1.8 },  // High volatility, strong performance
        { volatility: 0.03, performance: 0.9 },  // Very low volatility, slight loss
      ];
      
      currentTrendingTokens.forEach((token, tokenIndex) => {
        const profile = tokenProfiles[tokenIndex % tokenProfiles.length] ?? tokenProfiles[0]!;
        const gtwapData = new Map();
        const floorData = new Map();

        let currentGtwapPrice = uniformStartPrice;
        let currentFloorPrice = uniformStartPrice * 0.85; // Floor starts 15% below GTWAP
        let accumulatedVolatility = 0; // Track cumulative volatility for floor increases

        for (let i = 0; i < dataPoints; i++) {
          const timestamp = now - (dataPoints - i) * interval;
          const progress = i / (dataPoints - 1); // 0 to 1
          
          // Track volatility magnitude for floor calculation (before GTWAP calculation)
          const randomChange = (Math.random() - 0.5) * 2 * profile.volatility;
          accumulatedVolatility += Math.abs(randomChange);
          
          // Floor: Monotonically increasing based on accumulated fees (volatility proxy)
          // More gradual increase - reduced slope
          const baseFloorIncrease = progress * 0.08; // Base 8% increase over time period (reduced from 30%)
          const volatilityBonus = accumulatedVolatility * 0.15; // Volatility adds to floor growth (reduced from 0.5)
          const newFloorPrice = uniformStartPrice * 0.85 * (1 + baseFloorIncrease + volatilityBonus);
          
          // Ensure floor only increases (monotonic)
          currentFloorPrice = Math.max(currentFloorPrice, newFloorPrice);
          
          // GTWAP: Random walk with drift toward final performance
          const driftToTarget = (uniformStartPrice * profile.performance - currentGtwapPrice) * 0.02;
          const proposedGtwapPrice = currentGtwapPrice + randomChange + driftToTarget;
          
          // GTWAP cannot go below floor price - floor acts as support
          currentGtwapPrice = Math.max(currentFloorPrice, proposedGtwapPrice);
          
          gtwapData.set(timestamp, currentGtwapPrice);
          floorData.set(timestamp, currentFloorPrice);
        }

        // Store data in organized window globals like the working chart
        if (!(window as any).__multiTokenGtwapData) {
          (window as any).__multiTokenGtwapData = {};
        }
        if (!(window as any).__multiTokenFloorData) {
          (window as any).__multiTokenFloorData = {};
        }
        
        (window as any).__multiTokenGtwapData[token.address] = gtwapData;
        (window as any).__multiTokenFloorData[token.address] = floorData;
        
        console.log(`Stored data for ${token.symbol}, gtwap points: ${gtwapData.size}, floor points: ${floorData.size}, final GTWAP: ${currentGtwapPrice.toFixed(4)}, final floor: ${currentFloorPrice.toFixed(4)}`);
      });

      // Store token info in window for indicator access
      (window as any).__multiTokenInfo = currentTrendingTokens.map((token, index) => ({
        address: token.address,
        symbol: token.symbol.substring(0, 8),
        color: TOKEN_COLORS[index % TOKEN_COLORS.length],
        index,
      }));

      // Store initial visibility state
      (window as any).__multiTokenVisibility = tokenVisibility;


      // Register indicator for multi-token overlay if not already registered
      if (!registeredRef.current) {
        // Build figures dynamically for all tokens
        const figures: any[] = [];
        
        currentTrendingTokens.forEach((token, index) => {
          const color = TOKEN_COLORS[index % TOKEN_COLORS.length];
          const shortSymbol = token.symbol.substring(0, 8);
          
          // GTWAP figure (solid line)
          figures.push({
            key: `gtwap_${index}`,
            title: `${shortSymbol} GTWAP: `,
            type: 'line',
            styles: (_data: any) => ({
              color: color,
              size: 1,
              lineWidth: 1,
              solid: true,
            }),
          });
          
          // Floor figure (dashed line)  
          figures.push({
            key: `floor_${index}`,
            title: `${shortSymbol} Floor: `,
            type: 'line',
            styles: (_data: any) => ({
              color: color,
              size: 1,
              lineWidth: 1,
              style: 'dashed',
              dashedValue: [4, 4],
            }),
          });
        });

        // Register the indicator
        registerIndicator({
          name: 'MULTI_TOKEN_OVERLAY',
          shortName: '',
          calcParams: [],
          figures: figures,
          precision: 4,
          shouldOhlc: false,
          shouldFormatBigNumber: true,
          calc: (dataList: any[]) => {
            const gtwapDataMap = (window as any).__multiTokenGtwapData || {};
            const floorDataMap = (window as any).__multiTokenFloorData || {};
            const tokenInfo = (window as any).__multiTokenInfo || [];
            const tokenVisibility = (window as any).__multiTokenVisibility || {};
            
            console.log('Multi-token calc called:', {
              dataListLength: dataList.length,
              tokenInfoLength: tokenInfo.length,
              hasGtwapData: Object.keys(gtwapDataMap).length > 0,
              hasFloorData: Object.keys(floorDataMap).length > 0,
              visibility: tokenVisibility
            });
            
            return dataList.map(kline => {
              const result: any = {};
              
              tokenInfo.forEach((token: any, idx: number) => {
                // Check if token is visible (default to visible)
                const isVisible = tokenVisibility[token.address] !== false;
                if (!isVisible) return;
                
                const gtwapData = gtwapDataMap[token.address];
                const floorData = floorDataMap[token.address];
                
                if (gtwapData) {
                  const value = gtwapData.get(kline.timestamp);
                  if (value !== undefined) {
                    result[`gtwap_${idx}`] = value;
                  }
                }
                if (floorData) {
                  const value = floorData.get(kline.timestamp);
                  if (value !== undefined) {
                    result[`floor_${idx}`] = value;
                  }
                }
              });
              
              if (Object.keys(result).length > 0) {
                console.log('Sample calc result:', result);
              }
              
              return result;
            });
          },
        });
        registeredRef.current = true;
      }

      // Initialize chart with custom Y-axis range control
      const chartInstance = init(chartContainer, {
        layout: [
          {
            // @ts-expect-error - klinecharts type definitions issue
            type: 'candle',
            options: {
              id: 'candle_pane',
              axis: {
                createRange: ({ defaultRange }: any) => {
                  // Get our stored data range for custom calculation
                  const storedMinMax = (window as any).__customYAxisRange;
                  
                  if (storedMinMax) {
                    const { min, max } = storedMinMax;
                    const range = max - min;
                    
                    console.log(`Custom Y-axis range applied: ${min.toFixed(4)} to ${max.toFixed(4)}`);
                    
                    return {
                      from: min,
                      to: max,
                      range: range,
                      realFrom: min,
                      realTo: max,
                      realRange: range,
                      displayFrom: min,
                      displayTo: max,
                      displayRange: range
                    };
                  }
                  
                  // Fallback to default range
                  return defaultRange;
                }
              }
            }
          }
        ]
      });

      if (!chartInstance) {
        console.error('Failed to initialize multi-token chart');
        return;
      }

      chartRef.current = chartInstance;

      // Configure chart styles to hide candlesticks
      chartInstance.setStyles({
        candle: {
          // @ts-expect-error - klinecharts type definitions issue
          type: 'candle_solid',
          bar: {
            upColor: 'transparent',
            downColor: 'transparent',
            noChangeColor: 'transparent',
            upBorderColor: 'transparent',
            downBorderColor: 'transparent',
            noChangeBorderColor: 'transparent',
            upWickColor: 'transparent',
            downWickColor: 'transparent',
            noChangeWickColor: 'transparent',
          },
          area: {
            lineSize: 0,
            lineColor: 'transparent',
            // @ts-expect-error - klinecharts type definitions issue
            fillColor: 'transparent',
          },
          priceMark: {
            show: false,
          },
          tooltip: {
            // @ts-expect-error - klinecharts type definitions issue
            showRule: 'none',
          },
        },
        xAxis: {
          show: true,
          axisLine: {
            show: true,
            color: '#E5E7EB',
          },
          tickLine: {
            show: true,
            length: 4,
            color: '#E5E7EB',
          },
          tickText: {
            show: true,
            color: '#6B7280',
            size: 11,
          },
        },
        yAxis: {
          show: true,
          // @ts-expect-error - klinecharts type definitions issue
          type: 'normal',
          position: 'right',
          inside: false,
          axisLine: {
            show: true,
            color: '#E5E7EB',
          },
          tickLine: {
            show: true,
            length: 4,
            color: '#E5E7EB',
          },
          tickText: {
            show: true,
            color: '#6B7280',
            size: 11,
          },
        },
        grid: {
          show: true,
          horizontal: {
            show: true,
            size: 1,
            color: '#F3F4F6',
            // @ts-expect-error - klinecharts type definitions issue
            style: 'solid',
          },
          vertical: {
            show: false,
          },
        },
        crosshair: {
          show: true,
          horizontal: {
            show: true,
            line: {
              show: true,
              // @ts-expect-error - klinecharts type definitions issue
              style: 'dashed',
              dashedValue: [4, 2],
              size: 1,
              color: '#9CA3AF',
            },
            text: {
              show: true,
              color: '#FFFFFF',
              size: 11,
              backgroundColor: '#1F2937',
              borderColor: '#1F2937',
              borderSize: 1,
              borderRadius: 2,
              paddingLeft: 4,
              paddingRight: 4,
              paddingTop: 2,
              paddingBottom: 2,
            },
          },
          vertical: {
            show: true,
            line: {
              show: true,
              // @ts-expect-error - klinecharts type definitions issue
              style: 'dashed',
              dashedValue: [4, 2],
              size: 1,
              color: '#9CA3AF',
            },
            text: {
              show: true,
              color: '#FFFFFF',
              size: 11,
              backgroundColor: '#1F2937',
              borderColor: '#1F2937',
              borderSize: 1,
              borderRadius: 2,
              paddingLeft: 4,
              paddingRight: 4,
              paddingTop: 2,
              paddingBottom: 2,
            },
          },
        },
        indicator: {
          lastValueMark: {
            show: false,
          },
          tooltip: {
            // @ts-expect-error - klinecharts type definitions issue
            showRule: 'always',
            // @ts-expect-error - klinecharts type definitions issue
            showType: 'standard',
            text: {
              size: 11,
              color: '#1F2937',
              weight: 'normal',
              marginLeft: 8,
              marginTop: 8,
              marginRight: 8,
              marginBottom: 0,
            },
          },
        },
      });

      // Generate kline data
      const klineData: Array<{
        timestamp: number;
        open: number;
        high: number;
        low: number;
        close: number;
        volume: number;
      }> = [];
      for (let i = 0; i < dataPoints; i++) {
        const timestamp = now - (dataPoints - i) * interval;
        klineData.push({
          timestamp,
          open: 0,
          high: 0,
          low: 0,
          close: 0,
          volume: 0,
        });
      }

      // Calculate data range for better Y-axis fitting
      let minPrice = Infinity;
      let maxPrice = -Infinity;
      
      currentTrendingTokens.forEach((token) => {
        const gtwapData = (window as any).__multiTokenGtwapData[token.address];
        const floorData = (window as any).__multiTokenFloorData[token.address];
        
        if (gtwapData) {
          for (const value of gtwapData.values()) {
            if (value < minPrice) minPrice = value;
            if (value > maxPrice) maxPrice = value;
          }
        }
        if (floorData) {
          for (const value of floorData.values()) {
            if (value < minPrice) minPrice = value;
            if (value > maxPrice) maxPrice = value;
          }
        }
      });
      
      // Add minimal padding (5% on each side)
      const range = maxPrice - minPrice;
      const padding = range * 0.05;
      const adjustedMin = minPrice - padding; // Allow Y-axis to start above zero
      const adjustedMax = maxPrice + padding;
      
      console.log(`Data range: ${minPrice.toFixed(4)} - ${maxPrice.toFixed(4)}, adjusted: ${adjustedMin.toFixed(4)} - ${adjustedMax.toFixed(4)}`);

      // Store the custom range for the createRange callback
      (window as any).__customYAxisRange = {
        min: adjustedMin,
        max: adjustedMax
      };

      // Apply data to chart
      chartInstance.applyNewData(klineData);
      
      // Force the chart to recalculate range with our custom values
      setTimeout(() => {
        if (chartInstance) {
          try {
            // Trigger range recalculation by reapplying data
            chartInstance.applyNewData(klineData);
            console.log(`Triggered range recalculation with custom range: ${adjustedMin.toFixed(4)} to ${adjustedMax.toFixed(4)}`);
          } catch (e) {
            console.warn('Error triggering range recalculation:', e);
          }
        }
      }, 200);

      // Create the multi-token overlay indicator
      const indicatorId = chartInstance.createIndicator({
        name: 'MULTI_TOKEN_OVERLAY',
        id: 'multi_token_overlay',
        visible: true,
      }, false, { id: 'candle_pane' });
      
      console.log('Created multi-token indicator with ID:', indicatorId);
      
      // Debug: Check if indicator was created successfully
      const indicators = chartInstance.getIndicators();
      console.log('All indicators after creation:', indicators);
      
      // Debug: Check the specific indicator
      const createdIndicator = indicators.find(ind => ind.name === 'MULTI_TOKEN_OVERLAY');
      console.log('Created indicator details:', createdIndicator);

      // Force update by re-applying data to trigger indicator calculation
      chartInstance.applyNewData(klineData);

      setIsReady(true);
      initializingRef.current = false;
      
      // Additional delay to ensure everything is properly initialized
      setTimeout(() => {
        if (chartInstance) {
          chartInstance.applyNewData(klineData);
          console.log('Final chart update applied');
        }
      }, 200);
      
      // Force a resize and fit data to visible range
      setTimeout(() => {
        if (chartInstance) {
          try {
            chartInstance.resize();
            // Fit the chart to show all data optimally
            chartInstance.scrollToRealTime();
            chartInstance.zoomAtDataIndex(0, dataPoints - 1);
            console.log('Chart resized and fitted to data range');
          } catch (e) {
            console.warn('Error resizing chart:', e);
          }
        }
      }, 100);
    } catch (error) {
      console.error('Multi-token chart initialization error:', error);
      setIsReady(false);
      initializingRef.current = false;
    }
  }, [chartContainer]);

  // Initialize chart on mount - wait for tokenVisibility to be initialized
  useEffect(() => {
    if (!chartContainer) return;
    if (Object.keys(tokenVisibility).length === 0) return; // Wait for visibility state to be initialized
    if (chartRef.current) return; // Don't reinitialize if chart already exists
    
    // Wait for next frame to ensure container is fully rendered with dimensions
    const timeoutId = setTimeout(() => {
      initChart();
    }, 100);

    // Cleanup on unmount
    return () => {
      clearTimeout(timeoutId);
      if (chartRef.current && chartContainer) {
        try {
          const { dispose } = require('klinecharts');
          dispose(chartContainer);
        } catch (error) {
          console.warn('Error disposing chart:', error);
        }
        chartRef.current = null;
        initializingRef.current = false;
        setIsReady(false);
      }
    };
  }, [chartContainer, tokenVisibility]); // Trigger when container or visibility is ready, but only init once

  // Update chart when token visibility changes
  useEffect(() => {
    if (!isReady || !chartRef.current) return;
    
    // Update visibility state in window globals
    (window as any).__multiTokenVisibility = tokenVisibility;
    
    // Recalculate Y-axis range for visible tokens only
    let minPrice = Infinity;
    let maxPrice = -Infinity;
    
    // Only consider visible tokens for range calculation
    trendingTokensRef.current.forEach((token) => {
      const isVisible = tokenVisibility[token.address] !== false;
      if (!isVisible) return;
      
      const gtwapData = (window as any).__multiTokenGtwapData?.[token.address];
      const floorData = (window as any).__multiTokenFloorData?.[token.address];
      
      if (gtwapData) {
        for (const value of gtwapData.values()) {
          if (value < minPrice) minPrice = value;
          if (value > maxPrice) maxPrice = value;
        }
      }
      if (floorData) {
        for (const value of floorData.values()) {
          if (value < minPrice) minPrice = value;
          if (value > maxPrice) maxPrice = value;
        }
      }
    });
    
    // Apply new range if we have valid data
    if (minPrice !== Infinity && maxPrice !== -Infinity) {
      const range = maxPrice - minPrice;
      const padding = range * 0.05;
      const adjustedMin = minPrice - padding;
      const adjustedMax = maxPrice + padding;
      
      // Update the stored range
      (window as any).__customYAxisRange = {
        min: adjustedMin,
        max: adjustedMax
      };
      
      console.log(`Updated Y-axis range for visible tokens: ${adjustedMin.toFixed(4)} - ${adjustedMax.toFixed(4)}`);
    }
    
    // Generate kline data once
    const klineData: Array<{
      timestamp: number;
      open: number;
      high: number;
      low: number;
      close: number;
      volume: number;
    }> = [];
    const now = Date.now();
    const dataPoints = 96;
    const interval = 15 * 60 * 1000;
    
    for (let i = 0; i < dataPoints; i++) {
      const timestamp = now - (dataPoints - i) * interval;
      klineData.push({
        timestamp,
        open: 0,
        high: 0,
        low: 0,
        close: 0,
        volume: 0,
      });
    }
    
    // Apply data once with updated Y-axis range
    try {
      chartRef.current.applyNewData(klineData);
      chartRef.current.scrollToRealTime();
    } catch (e) {
      console.warn('Error updating chart visibility:', e);
    }
  }, [tokenVisibility, isReady]);

  // Handle chart resize when container size changes
  useEffect(() => {
    if (!chartRef.current || !chartContainer) return;

    const resizeObserver = new ResizeObserver(() => {
      if (chartRef.current) {
        try {
          chartRef.current.resize();
        } catch (error) {
          console.warn('Error resizing chart:', error);
        }
      }
    });

    resizeObserver.observe(chartContainer);

    return () => {
      resizeObserver.disconnect();
    };
  }, [chartContainer]);

  return (
    <Card className="w-full h-full flex flex-col">
      <CardHeader className="pb-3 flex-shrink-0">
        {/* Token Toggle Legend - Interactive token visibility controls */}
        <div className="w-full px-4">
          <div className="flex justify-between items-center">
            {trendingTokens.map((token, index) => {
              const color = TOKEN_COLORS[index % TOKEN_COLORS.length];
              const isVisible = tokenVisibility[token.address] ?? true;
              
              return (
                <button
                  key={token.address}
                  onClick={() => setTokenVisibility(prev => ({
                    ...prev,
                    [token.address]: !isVisible
                  }))}
                  className={`flex-1 flex items-center justify-center gap-2 cursor-pointer transition-opacity ${
                    isVisible ? '' : 'opacity-40'
                  }`}
                >
                  {/* Price and price change on the left */}
                  <div className="flex flex-col items-end">
                    <p className={`text-sm font-semibold whitespace-nowrap transition-opacity ${
                      isVisible ? '' : 'opacity-40'
                    }`}>
                      ${token.price.toFixed(4)}
                    </p>
                    <p className={`text-xs font-medium whitespace-nowrap transition-opacity ${
                      isVisible ? (token.priceChange24h >= 0 ? 'text-primary' : 'text-danger-500') : 'opacity-40'
                    }`}>
                      {token.priceChange24h >= 0 ? '+' : ''}{token.priceChange24h.toFixed(2)}%
                    </p>
                  </div>
                  
                  {/* Token image */}
                  {token.imageUrl && (
                    <Image
                      src={token.imageUrl}
                      alt={token.symbol}
                      width={24}
                      height={24}
                      className="rounded-sm"
                    />
                  )}
                  
                  {/* Radio button indicator */}
                  <div 
                    className="relative h-3 w-3 rounded-full border-2 flex items-center justify-center flex-shrink-0"
                    style={{ borderColor: color }}
                  >
                    {isVisible && (
                      <div 
                        className="h-1.5 w-1.5 rounded-full"
                        style={{ backgroundColor: color }}
                      />
                    )}
                  </div>
                  
                  {/* Token symbol */}
                  <p 
                    className={`text-xs whitespace-nowrap transition-colors ${
                      isVisible ? 'font-medium' : 'text-muted-foreground'
                    }`}
                    style={{ color: isVisible ? color : undefined }}
                  >
                    {token.symbol}
                  </p>
                </button>
              );
            })}
          </div>
        </div>
      </CardHeader>
      <CardContent className="pl-8 pr-2 pt-2 pb-4 flex-1 flex flex-col min-h-0">
        {/* Chart container */}
        <div className="w-full relative overflow-visible flex-1 min-h-[500px]">
          <div
            ref={setChartContainer}
            className="klinechart-container absolute inset-0"
            style={{
              backgroundColor: '#ffffff',
              opacity: isReady ? 1 : 0,
              transition: 'opacity 0.2s ease-in-out',
            }}
          />
          {!isReady && (
            <div className="absolute inset-0 flex items-center justify-center bg-background">
              <div className="animate-spin h-8 w-8 rounded-full border-2 border-muted border-t-primary"></div>
            </div>
          )}
        </div>

        {/* Footer legend with visual line examples */}
        <div className="pr-12 pt-4 flex-shrink-0">
          <div className="flex items-center justify-center gap-8 text-sm text-foreground">
            <div className="flex items-center gap-2">
              <svg width="32" height="3" className="flex-shrink-0">
                <line x1="0" y1="1.5" x2="32" y2="1.5" stroke="#000000" strokeWidth="1" />
              </svg>
              <span className="font-medium">GTWAP</span>
            </div>
            <div className="flex items-center gap-2">
              <svg width="32" height="3" className="flex-shrink-0">
                <line x1="0" y1="1.5" x2="32" y2="1.5" stroke="#000000" strokeWidth="1" strokeDasharray="4 4" />
              </svg>
              <span className="font-medium">Floor Price</span>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

