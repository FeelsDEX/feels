'use client';

import { useState, useEffect, useMemo, useRef } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ExternalLink, BarChart3 } from 'lucide-react';
import { init, dispose, registerIndicator } from 'klinecharts';
import type { Chart, KLineData } from 'klinecharts';

interface ExtendedKLineData extends KLineData {
  floor: number;
  gtwap: number;
}

interface PriceChartProps {
  tokenSymbol?: string;
  tokenAddress?: string;
  isFeelsToken: boolean;
  onPriceDataUpdate?: (data: {
    currentPrice: number;
    currentFloor: number;
    currentGtwap: number;
    allPriceData: ExtendedKLineData[];
  }) => void;
}

type TimeRange = 'all' | '1M' | '1W' | '1D' | '1H';

export function PriceChart({ tokenSymbol, tokenAddress, isFeelsToken, onPriceDataUpdate }: PriceChartProps) {
  const [timeRange] = useState<TimeRange>('all');
  const [loading, setLoading] = useState(false);
  const [allPriceData, setAllPriceData] = useState<ExtendedKLineData[]>([]);
  const [filteredPriceData, setFilteredPriceData] = useState<ExtendedKLineData[]>([]);
  const [currentPrice, setCurrentPrice] = useState(0);
  const [currentFloor, setCurrentFloor] = useState(0);
  const [currentGtwap, setCurrentGtwap] = useState(0);
  const [chartReady, setChartReady] = useState(false);
  const chartRef = useRef<HTMLDivElement>(null);
  const chartInstanceRef = useRef<Chart | null>(null);

  // Simple seeded random for consistent data
  const seededRandom = (seed: number): number => {
    const x = Math.sin(seed) * 10000;
    return x - Math.floor(x);
  };

  // Generate all historical data once
  const generateAllHistoricalData = useMemo(() => {
    return (): ExtendedKLineData[] => {
      const now = Date.now();
      const candles: ExtendedKLineData[] = [];
      
      // Simulate token launched 30 days ago
      const launchDate = now - 30 * 24 * 60 * 60 * 1000;
      
      // Generate 15-minute candles for entire history
      const interval = 15 * 60 * 1000; // 15 minutes
      const numPoints = Math.floor((now - launchDate) / interval);
      
      // Use token address as seed for consistent data
      const baseSeed = tokenAddress ? tokenAddress.charCodeAt(0) : 42;
      
      // Generate realistic-looking candlestick data
      let basePrice = 0.1; // Start with low price
      let baseFloor = 0.08; // Initial floor
      const volatility = 0.02; // 2% volatility
      const floorVolatility = 0.001; // Floor moves slower (0.1% per period)
      let gtwapSum = 0;
      let gtwapWeight = 0;
      
      for (let i = 0; i < numPoints; i++) {
        // Calculate the timestamp for this candle
        const candleTime = launchDate + (i * interval);
        
        // Don't generate candles beyond current time
        if (candleTime > now) break;
        
        // Floor movement - ONLY INCREASES (monotonic)
        const floorIncrease = seededRandom(baseSeed + i * 1000) * floorVolatility;
        baseFloor = baseFloor * (1 + floorIncrease);
        
        // Generate OHLC data
        const open = i === 0 ? basePrice : (candles[i - 1]?.close ?? basePrice);
        
        // Intrabar movements
        const intraDayVolatility = volatility * 0.5;
        const move1 = (seededRandom(baseSeed + i * 1000 + 1) - 0.5) * 2 * intraDayVolatility;
        const move2 = (seededRandom(baseSeed + i * 1000 + 2) - 0.5) * 2 * intraDayVolatility;
        const move3 = (seededRandom(baseSeed + i * 1000 + 3) - 0.5) * 2 * intraDayVolatility;
      
      const intrabarPrices = [
        open,
        open * (1 + move1),
        open * (1 + move1 + move2),
        open * (1 + move1 + move2 + move3)
      ];
      
      // Ensure all prices respect the floor
      const validPrices = intrabarPrices.map(p => Math.max(baseFloor, p));
      
      const high = Math.max(...validPrices);
      const low = Math.min(...validPrices);
      const close = validPrices[validPrices.length - 1];
      
      // Update base price for next candle
      basePrice = close ?? basePrice;
      
      // Calculate GTWAP (simplified - using close prices)
      gtwapSum += (close ?? 0) * (i + 1);
      gtwapWeight += (i + 1);
      const gtwap = gtwapSum / gtwapWeight;
      
      // Generate volume (higher when price moves more)
      const priceChange = Math.abs(((close ?? open) - open) / open);
      const baseVolume = 100000;
      const volume = baseVolume * (1 + priceChange * 10) * (0.5 + seededRandom(baseSeed + i * 1000 + 4));
      
      candles.push({
        timestamp: candleTime,
        open,
        high,
        low,
        close: close ?? 0,
        volume: Math.round(volume),
        floor: baseFloor,
        gtwap
      });
    }
    
    return candles;
    };
  }, [tokenAddress]);

  // Filter data based on time range
  const filterDataByTimeRange = (data: ExtendedKLineData[], range: TimeRange): ExtendedKLineData[] => {
    const now = Date.now();
    let cutoffTime: number;
    
    switch (range) {
      case 'all':
        return data; // Return all data
      case '1M':
        cutoffTime = now - 30 * 24 * 60 * 60 * 1000;
        break;
      case '1W':
        cutoffTime = now - 7 * 24 * 60 * 60 * 1000;
        break;
      case '1D':
        cutoffTime = now - 24 * 60 * 60 * 1000;
        break;
      case '1H':
        cutoffTime = now - 60 * 60 * 1000;
        break;
      default:
        return data;
    }
    
    return data.filter(candle => candle.timestamp >= cutoffTime);
  };

  // Register custom indicator once
  useEffect(() => {
    registerIndicator({
      name: 'FloorGTWAP',
      shortName: 'FG',
      precision: 4,
      figures: [
        { 
          key: 'floor', 
          title: 'Floor', 
          type: 'line',
          styles: () => ({
            color: '#5cca39',
            dashedValue: [4, 4]
          })
        },
        { 
          key: 'gtwap', 
          title: 'GTWAP', 
          type: 'line',
          styles: () => ({
            color: '#3b82f6'
          })
        }
      ],
      calc: (kLineDataList: KLineData[]) => {
        return kLineDataList.map((kLineData) => {
          const extendedData = kLineData as ExtendedKLineData;
          return {
            floor: extendedData.floor || 0,
            gtwap: extendedData.gtwap || 0
          };
        });
      }
    });
  }, []);

  // Initialize chart
  useEffect(() => {
    if (!chartRef.current || !isFeelsToken) return;

    // Temporarily suppress console.log to avoid KLineChart welcome message
    const originalLog = console.log;
    console.log = () => {};
    
    // Create chart instance
    const chartConfig: any = {
      locale: 'en-US',
      timezone: 'UTC',
      styles: {
        grid: {
          show: true,
          horizontal: {
            show: true,
            size: 1,
            color: 'rgba(180, 180, 180, 0.3)'
          },
          vertical: {
            show: true,
            size: 1,
            color: 'rgba(180, 180, 180, 0.3)'
          }
        },
        candle: {
          type: 'candle_solid',
          bar: {
            upColor: '#5cca39',
            downColor: '#ef4444',
            borderUpColor: '#5cca39',
            borderDownColor: '#ef4444',
            upBorderColor: '#5cca39',
            downBorderColor: '#ef4444',
            wickUpColor: '#5cca39',
            wickDownColor: '#ef4444'
          }
        },
        xAxis: {
          axisLine: {
            show: true,
            color: 'transparent',
            size: 1
          },
          tickLine: {
            show: true,
            color: 'transparent',
            size: 1
          },
          tickText: {
            show: true,
            color: '#666666',
            size: 12
          },
          tickCount: 6
        },
        yAxis: {
          type: 'normal',
          position: 'right',
          axisLine: {
            show: true,
            color: 'transparent',
            size: 1
          },
          tickLine: {
            show: true,
            color: 'transparent',
            size: 1
          },
          tickText: {
            show: true,
            color: '#666666',
            size: 12
          }
        },
        crosshair: {
          show: true,
          horizontal: {
            show: true,
            line: {
              show: true,
                dashValue: [2, 2],
              size: 1,
              color: '#666'
            },
            text: {
              show: true,
              color: '#fff',
              size: 12,
              backgroundColor: '#333',
              borderSize: 0,
              paddingLeft: 4,
              paddingRight: 4,
              paddingTop: 4,
              paddingBottom: 4,
              borderRadius: 4
            }
          },
          vertical: {
            show: true,
            line: {
              show: true,
              style: 'dashed' as const, 
              dashValue: [2, 2],
              size: 1,
              color: '#666'
            },
            text: {
              show: true,
              color: '#fff',
              size: 12,
              backgroundColor: '#333',
              borderSize: 0,
              paddingLeft: 4,
              paddingRight: 4,
              paddingTop: 4,
              paddingBottom: 4,
              borderRadius: 4
            }
          }
        }
      }
    };
    
    const chart = init(chartRef.current, chartConfig);

    if (chart) {
      chartInstanceRef.current = chart;

      // Create the registered indicator
      chart.createIndicator('FloorGTWAP', true, { id: 'candle_pane' });
    }
    
    // Restore console.log
    console.log = originalLog;

    // Apply background color to the main canvas
    setTimeout(() => {
      if (chartRef.current) {
        // Find the main chart canvas (first canvas element)
        const canvas = chartRef.current.querySelector('div > div:first-child > div:first-child > canvas:first-child') as HTMLCanvasElement;
        if (canvas) {
          canvas.style.backgroundColor = '#f8f8f8';
        }
      }
    }, 100);

    // Cleanup
    return () => {
      const chartElement = chartRef.current;
      if (chartElement) {
        dispose(chartElement);
      }
      chartInstanceRef.current = null;
    };
  }, [isFeelsToken]);

  // Generate all data once when component mounts or token changes
  useEffect(() => {
    if (!isFeelsToken) return;
    
    setLoading(true);
    const data = generateAllHistoricalData();
    setAllPriceData(data);
    
    // Set initial filtered data
    const filtered = filterDataByTimeRange(data, timeRange);
    setFilteredPriceData(filtered);
    if (filtered.length > 0) {
      const lastCandle = filtered[filtered.length - 1];
      setCurrentPrice(lastCandle?.close ?? 0);
      setCurrentFloor(lastCandle?.floor ?? 0);
      setCurrentGtwap(lastCandle?.gtwap ?? 0);
      
      // Notify parent component
      if (onPriceDataUpdate && lastCandle) {
        onPriceDataUpdate({
          currentPrice: lastCandle.close,
          currentFloor: lastCandle.floor,
          currentGtwap: lastCandle.gtwap,
          allPriceData: data
        });
      }
    }
    setLoading(false);
  }, [isFeelsToken, tokenAddress, generateAllHistoricalData, onPriceDataUpdate]);

  // Filter data when time range changes
  useEffect(() => {
    if (!isFeelsToken || allPriceData.length === 0) return;

    const data = filterDataByTimeRange(allPriceData, timeRange);
    setFilteredPriceData(data);
    
    if (data.length > 0) {
      const lastCandle = data[data.length - 1];
      setCurrentPrice(lastCandle?.close ?? 0);
      setCurrentFloor(lastCandle?.floor ?? 0);
      setCurrentGtwap(lastCandle?.gtwap ?? 0);
    }
  }, [timeRange, allPriceData, isFeelsToken]);

  // Apply data to chart when filtered data changes
  useEffect(() => {
    if (!chartInstanceRef.current || !isFeelsToken) return;

    let timeoutId: NodeJS.Timeout | undefined;
    let rangeTimeoutId: NodeJS.Timeout | undefined;
    let readyTimeoutId: NodeJS.Timeout | undefined;

    // Reset ready state when data changes
    setChartReady(false);

    // Only apply data if we have some
    if (filteredPriceData.length > 0) {
      // Small delay to ensure chart is fully initialized
      timeoutId = setTimeout(() => {
        if (chartInstanceRef.current) {
          chartInstanceRef.current.clearData();
          chartInstanceRef.current.applyNewData(filteredPriceData);
        }
      }, 50);
    
      // Set the visible range to show all data for the selected time period
      // Force the chart to display the full range
      rangeTimeoutId = setTimeout(() => {
        if (chartInstanceRef.current) {
          // Calculate bar width to fit all data points in the visible area
          const chartWidth = chartRef.current!.clientWidth - 100; // Account for margins
          const desiredBarWidth = Math.max(2, Math.floor(chartWidth / filteredPriceData.length));
          
          // Set bar space to show all data
          chartInstanceRef.current.setBarSpace(desiredBarWidth);
          
          // Set right offset to 0 to align the latest data with the right edge
          chartInstanceRef.current.setOffsetRightDistance(0);
          
          // Ensure we can see all data by setting the left min visible bar count
          chartInstanceRef.current.setLeftMinVisibleBarCount(filteredPriceData.length);
        }
      }, 100);

      // Mark chart as ready after all operations complete
      readyTimeoutId = setTimeout(() => {
        setChartReady(true);
      }, 200);
    }
      
    // Set appropriate time format based on time range
    let customFormatter;
    switch (timeRange) {
        case '1H':
          customFormatter = {
            xAxis: {
              tickText: {
                custom: (timestamp: number) => {
                  const date = new Date(timestamp);
                  return `${date.getHours().toString().padStart(2, '0')}:${date.getMinutes().toString().padStart(2, '0')}`;
                }
              }
            }
          };
          break;
        case '1D':
          customFormatter = {
            xAxis: {
              tickText: {
                custom: (timestamp: number) => {
                  const date = new Date(timestamp);
                  return `${date.getHours().toString().padStart(2, '0')}:${date.getMinutes().toString().padStart(2, '0')}`;
                }
              }
            }
          };
          break;
        case '1W':
          customFormatter = {
            xAxis: {
              tickText: {
                custom: (timestamp: number) => {
                  const date = new Date(timestamp);
                  return `${date.getMonth() + 1}/${date.getDate()} ${date.getHours().toString().padStart(2, '0')}:00`;
                }
              }
            }
          };
          break;
        case '1M':
          customFormatter = {
            xAxis: {
              tickText: {
                custom: (timestamp: number) => {
                  const date = new Date(timestamp);
                  return `${date.getMonth() + 1}/${date.getDate()}`;
                }
              }
            }
          };
          break;
        case 'all':
          customFormatter = {
            xAxis: {
              tickText: {
                custom: (timestamp: number) => {
                  const date = new Date(timestamp);
                  return `${date.getMonth() + 1}/${date.getDate()}`;
                }
              }
            }
          };
          break;
      }
    
    // Apply custom formatter with y-axis precision
    if (chartInstanceRef.current && customFormatter) {
      const styles = {
        ...customFormatter,
        yAxis: {
          tickText: {
            show: true,
            color: '#666666',
            size: 12,
            custom: (value: number) => value.toFixed(4)
          }
        }
      };
      
      chartInstanceRef.current.setStyles(styles as any);
    }
    
    return () => {
      if (timeoutId) clearTimeout(timeoutId);
      if (rangeTimeoutId) clearTimeout(rangeTimeoutId);
      if (readyTimeoutId) clearTimeout(readyTimeoutId);
    };
  }, [filteredPriceData, isFeelsToken, timeRange]);


  // Handle window resize
  useEffect(() => {
    const handleResize = () => {
      if (chartInstanceRef.current && chartRef.current) {
        chartInstanceRef.current.resize();
      }
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  // Calculate price change percentage
  const calculatePriceChange = () => {
    if (filteredPriceData.length < 2) return 0;
    const firstPrice = filteredPriceData[0]?.close ?? 0;
    const lastPrice = filteredPriceData[filteredPriceData.length - 1]?.close ?? 0;
    return ((lastPrice - firstPrice) / firstPrice) * 100;
  };

  const priceChange = calculatePriceChange();

  if (!isFeelsToken) {
    return (
      <Card id="price-chart-empty-state" className="h-full">
        <CardHeader>
          <CardTitle className="text-lg">Price Chart</CardTitle>
        </CardHeader>
        <CardContent>
          <div id="empty-state-message" className="flex items-center justify-center h-[400px] text-muted-foreground">
            <p className="text-center">
              Select a token launched on Feels<br />
              to view price history
            </p>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card id="price-chart-card" className="h-full">
      <CardHeader id="chart-header-section" className="pl-8 pb-4">
        <div className="flex justify-between items-end">
          <div>
            <div className="flex items-center gap-4">
              <CardTitle className="text-lg">{tokenSymbol || 'Token'} / FeelsSOL</CardTitle>
              <div className="flex items-center gap-3 text-xs">
                {tokenAddress && (
                  <a 
                    href={`https://solscan.io/token/${tokenAddress}?cluster=devnet`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-primary hover:underline flex items-center gap-1"
                  >
                    Explorer
                    <ExternalLink className="h-3 w-3" />
                  </a>
                )}
              </div>
            </div>
            <div id="price-display" className="flex items-center gap-2 mt-1">
              <span id="current-price" className="text-2xl font-bold">{currentPrice.toFixed(4)}</span>
              <span id="price-change-percentage" className={`text-sm ${priceChange >= 0 ? 'text-primary' : 'text-red-500'}`}>
                {priceChange >= 0 ? '+' : ''}{priceChange.toFixed(2)}%
              </span>
            </div>
          </div>
          <div id="chart-indicators" className="flex items-center gap-4 text-xs text-muted-foreground mr-4 mb-1">
            <div className="flex items-center gap-1">
              <BarChart3 className="h-3 w-3" />
              <span>OHLC</span>
            </div>
            <div className="flex items-center gap-1">
              <div className="w-3 h-0.5 bg-primary"></div>
              <span>Floor: {currentFloor.toFixed(4)}</span>
            </div>
            <div className="flex items-center gap-1">
              <div className="w-3 h-0.5 bg-blue-500"></div>
              <span>GTWAP: {currentGtwap.toFixed(4)}</span>
            </div>
          </div>
        </div>
      </CardHeader>
      <CardContent className="pl-8 pr-2 pt-0">
        {/* Always render the chart container */}
        <div id="chart-canvas-container" className="w-full relative">
          <div 
            ref={chartRef} 
            id="kline-chart"
            className="klinechart-container"
            style={{ 
              width: '100%', 
              height: '400px',
              backgroundColor: 'white',
              opacity: chartReady ? 1 : 0,
              transition: 'opacity 0.05s ease-in-out'
            }} 
          />
          {(loading || !chartReady) && (
            <div id="chart-loading-spinner" className="absolute inset-0 flex items-center justify-center bg-background">
              <div className="animate-spin h-8 w-8 rounded-full border-2 border-muted border-t-primary"></div>
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}