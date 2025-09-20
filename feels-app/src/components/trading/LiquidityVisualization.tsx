'use client';

import { useState, useEffect, useMemo } from 'react';
import { Connection } from '@solana/web3.js';
import { Program, Idl } from '@coral-xyz/anchor';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Waves } from 'lucide-react';

interface LiquidityVisualizationProps {
  connection: Connection;
  program: Program<Idl> | null;
  selectedPool?: string;
}

interface TickData {
  tick: number;
  price: number;
  liquidity: number;
  logPrice: number;
  isFloorTick?: boolean;
  tickLower?: number;  // Lower bound of tick range
  tickUpper?: number;  // Upper bound of tick range
  rangeSize?: number;  // Size of the liquidity position range
}

interface PoolMetrics {
  floorTickIndex: number;
  gtwapPrice: number;
  virtualJitDepth: number;
}

export function LiquidityVisualization({ connection, program, selectedPool = 'SOL/USDC' }: LiquidityVisualizationProps) {
  const [tickData, setTickData] = useState<TickData[]>([]);
  const [poolMetrics, setPoolMetrics] = useState<PoolMetrics | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  
  // Visibility state for chart elements
  const [showRegularTicks, setShowRegularTicks] = useState(true);
  const [showFloorTick, setShowFloorTick] = useState(true);
  const [showGtwapLine, setShowGtwapLine] = useState(true);
  const [showCumulativeLiquidity, setShowCumulativeLiquidity] = useState(true);

  // Generate realistic market maker liquidity distribution
  const generateMockData = useMemo(() => {
    const data: TickData[] = [];
    
    // Pool-specific parameters reflecting real market maker behavior
    const poolConfig = {
      'SOL/USDC': { 
        basePrice: 100, 
        maxLiquidity: 1200000, 
        isStable: false, 
        floorOffset: -25, 
        gtwapOffset: 8, 
        jitDepth: 450000,
        tightSpread: 2, // ticks within tight spread
        mediumSpread: 8 // ticks within medium spread
      },
      'SOL/JitoSOL': { 
        basePrice: 1.05, 
        maxLiquidity: 800000, 
        isStable: true, 
        floorOffset: -15, 
        gtwapOffset: 3, 
        jitDepth: 320000,
        tightSpread: 1,
        mediumSpread: 4
      },
      'FeelsSOL/SOL': { 
        basePrice: 0.98, 
        maxLiquidity: 600000, 
        isStable: false, 
        floorOffset: -20, 
        gtwapOffset: 5, 
        jitDepth: 280000,
        tightSpread: 2,
        mediumSpread: 6
      },
      'USDC/USDT': { 
        basePrice: 1.0001, 
        maxLiquidity: 400000, 
        isStable: true, 
        floorOffset: -5, 
        gtwapOffset: 1, 
        jitDepth: 150000,
        tightSpread: 1,
        mediumSpread: 3
      },
      'JitoSOL/mSOL': { 
        basePrice: 1.02, 
        maxLiquidity: 500000, 
        isStable: true, 
        floorOffset: -10, 
        gtwapOffset: 2, 
        jitDepth: 180000,
        tightSpread: 1,
        mediumSpread: 3
      },
      'WOJAK/FeelsSOL': { 
        basePrice: 0.0423, 
        maxLiquidity: 420000, 
        isStable: false, 
        floorOffset: -30, 
        gtwapOffset: 10, 
        jitDepth: 69000,
        tightSpread: 3,
        mediumSpread: 10
      },
      'COOMER/FeelsSOL': { 
        basePrice: 0.0234, 
        maxLiquidity: 380000, 
        isStable: false, 
        floorOffset: -28, 
        gtwapOffset: 9, 
        jitDepth: 85000,
        tightSpread: 3,
        mediumSpread: 9
      },
      'CHAD/FeelsSOL': { 
        basePrice: 0.0345, 
        maxLiquidity: 560000, 
        isStable: false, 
        floorOffset: -35, 
        gtwapOffset: 12, 
        jitDepth: 120000,
        tightSpread: 4,
        mediumSpread: 12
      },
    };
    
    const config = poolConfig[selectedPool as keyof typeof poolConfig] || poolConfig['SOL/USDC'];
    const currentTick = Math.log(config.basePrice);
    const tickSpacing = 0.02;
    
    const floorTickIndex = config.floorOffset;
    const gtwapTickIndex = config.gtwapOffset;
    
    // Generate realistic market maker liquidity distribution
    for (let i = -100; i <= 100; i += 1) {
      const tick = currentTick + (i * tickSpacing);
      const price = Math.exp(tick);
      const isFloorTick = i === floorTickIndex;
      const distanceFromCurrent = Math.abs(i);
      const distanceFromFloor = Math.abs(i - floorTickIndex);
      const distanceFromGtwap = Math.abs(i - gtwapTickIndex);
      
      let liquidity = 0;
      
      // 1. TIGHT SPREAD AROUND CURRENT PRICE (80% of volume happens here)
      if (distanceFromCurrent <= config.tightSpread) {
        liquidity = config.maxLiquidity * (0.8 - distanceFromCurrent * 0.2);
      }
      
      // 2. MEDIUM SPREAD LIQUIDITY (capture larger moves)
      else if (distanceFromCurrent <= config.mediumSpread) {
        liquidity = config.maxLiquidity * (0.4 - (distanceFromCurrent - config.tightSpread) * 0.08);
      }
      
      // 3. STRATEGIC PLACEMENT ABOVE FLOOR TICK (arbitrage opportunity)
      else if (i > floorTickIndex && distanceFromFloor <= 5) {
        liquidity = config.maxLiquidity * (0.6 - distanceFromFloor * 0.1);
      }
      
      // 4. AVOID DIRECT COMPETITION WITH GTWAP (but place nearby)
      else if (distanceFromGtwap >= 2 && distanceFromGtwap <= 4) {
        liquidity = config.maxLiquidity * 0.3;
      }
      
      // 5. SPARSE LIQUIDITY AT EXTREMES (limited due to IL risk)
      else if (distanceFromCurrent > 20 && distanceFromCurrent <= 40) {
        liquidity = config.maxLiquidity * 0.1 * Math.random();
      }
      
      // 6. INTENTIONAL GAPS (market makers avoid unprofitable ranges)
      else if (distanceFromCurrent > config.mediumSpread && distanceFromCurrent < 20) {
        // Sparse liquidity with gaps
        if (Math.random() > 0.7) {
          liquidity = config.maxLiquidity * 0.15 * Math.random();
        }
      }
      
      // 7. FLOOR TICK SPECIAL HANDLING
      if (isFloorTick) {
        liquidity = config.maxLiquidity * 1.8; // Protocol-owned, very deep
      }
      
      // 8. ASYMMETRIC DISTRIBUTION (more liquidity on buy side near support)
      if (i < 0 && i > floorTickIndex) {
        liquidity *= 1.3; // More buy-side liquidity near support
      }
      
      // 9. STABLE PAIR ADJUSTMENTS (tighter clustering, less volatility)
      if (config.isStable) {
        if (distanceFromCurrent > 5) {
          liquidity *= 0.5; // Much less liquidity far from current price
        }
      }
      
      // 10. ADD REALISTIC NOISE
      liquidity *= (0.8 + Math.random() * 0.4); // 20% variance
      
      // Only add tick if it has meaningful liquidity
      if (liquidity > config.maxLiquidity * 0.05) {
        data.push({
          tick: i,
          price,
          liquidity: Math.round(liquidity),
          logPrice: tick,
          isFloorTick
        });
      }
    }
    
    // Set pool metrics
    const gtwapPrice = Math.exp(currentTick + (gtwapTickIndex * tickSpacing));
    setPoolMetrics({
      floorTickIndex,
      gtwapPrice,
      virtualJitDepth: config.jitDepth
    });
    
    return data.sort((a, b) => a.logPrice - b.logPrice);
  }, [selectedPool]);

  useEffect(() => {
    async function fetchTickData() {
      try {
        setLoading(true);
        setError(null);
        
        // In a real implementation, this would fetch actual tick data from the program
        // For now, use mock data immediately
        setTickData(generateMockData);
        setLoading(false);
        
      } catch (err) {
        console.error('Failed to fetch tick data:', err);
        setError(err instanceof Error ? err.message : 'Failed to fetch tick data');
        setLoading(false);
      }
    }

    fetchTickData();
  }, [connection, program, generateMockData]);

  // Calculate cumulative liquidity (swappable liquidity at each price point)
  const cumulativeLiquidityData = useMemo(() => {
    if (tickData.length === 0) return [];
    
    // First, assign realistic range sizes to each tick position
    // In concentrated liquidity, each position spans a range of ticks
    const positionsWithRanges = tickData.map(d => {
      let rangeSize = 1; // Default single tick
      
      const distanceFromCurrent = Math.abs(d.tick);
      
      // Market makers use different range sizes based on strategy
      if (d.isFloorTick) {
        rangeSize = 1; // Floor tick is a single point
      } else if (distanceFromCurrent <= 3) {
        rangeSize = Math.random() < 0.7 ? 2 : 4; // Tight ranges near current price
      } else if (distanceFromCurrent <= 10) {
        rangeSize = Math.random() < 0.5 ? 5 : 8; // Medium ranges
      } else if (distanceFromCurrent <= 20) {
        rangeSize = Math.random() < 0.3 ? 10 : 15; // Wide ranges further out
      } else {
        rangeSize = Math.random() < 0.2 ? 20 : 30; // Very wide ranges at extremes
      }
      
      const halfRange = Math.floor(rangeSize / 2);
      
      return {
        ...d,
        tickLower: d.tick - halfRange,
        tickUpper: d.tick + halfRange,
        rangeSize
      };
    });
    
    // Get price range for calculation
    const priceExtent = [
      Math.min(...tickData.map(d => d.logPrice)),
      Math.max(...tickData.map(d => d.logPrice))
    ];
    
    // Create high-resolution price grid
    const numPoints = 300;
    const depthData = [];
    
    for (let i = 0; i <= numPoints; i++) {
      const logPrice = priceExtent[0] + (i / numPoints) * (priceExtent[1] - priceExtent[0]);
      const currentTick = (logPrice - Math.log(100)) / 0.02; // Convert to tick space
      
      // Calculate swappable liquidity at this price
      // Sum all positions that include this price in their range
      let swappableLiquidity = 0;
      
      positionsWithRanges.forEach(position => {
        if (currentTick >= position.tickLower && currentTick <= position.tickUpper) {
          swappableLiquidity += position.liquidity;
        }
      });
      
      depthData.push({
        tick: currentTick,
        price: Math.exp(logPrice),
        liquidity: 0, // Not used for cumulative
        logPrice,
        cumulativeLiquidity: swappableLiquidity,
        isFloorTick: false
      });
    }
    
    return depthData;
  }, [tickData]);

  // Calculate chart dimensions and scaling
  const chartDimensions = {
    width: 1000,
    height: 600,
    padding: { top: 60, right: 60, bottom: 60, left: 80 }
  };

  const chartArea = {
    width: chartDimensions.width - chartDimensions.padding.left - chartDimensions.padding.right,
    height: chartDimensions.height - chartDimensions.padding.top - chartDimensions.padding.bottom
  };

  // Add vertical padding to prevent bars from extending beyond axes
  const verticalPadding = 20; // pixels of padding on top and bottom

  // Calculate scales
  const liquidityExtent = tickData.length > 0 ? [
    0,
    Math.max(...tickData.map(d => d.liquidity))
  ] : [0, 1];

  const cumulativeExtent = cumulativeLiquidityData.length > 0 ? [
    0,
    Math.max(...cumulativeLiquidityData.map(d => d.cumulativeLiquidity)) || 1
  ] : [0, 1];

  const priceExtent = tickData.length > 0 ? [
    Math.min(...tickData.map(d => d.logPrice)),
    Math.max(...tickData.map(d => d.logPrice))
  ] : [0, 1];

  const xScale = (liquidity: number) => 
    (liquidity / liquidityExtent[1]) * chartArea.width;

  const xScaleCumulative = (cumulative: number) => {
    if (cumulativeExtent[1] === 0) return 0;
    return (cumulative / cumulativeExtent[1]) * chartArea.width;
  };

  const yScale = (logPrice: number) => {
    const normalizedPosition = (logPrice - priceExtent[0]) / (priceExtent[1] - priceExtent[0]);
    return chartArea.height - verticalPadding - (normalizedPosition * (chartArea.height - 2 * verticalPadding));
  };

  // Calculate bar width based on tick spacing
  const getBarWidth = () => {
    if (tickData.length <= 1) return 1;
    
    // Calculate the average spacing between consecutive ticks in chart coordinates
    const logPriceSpacing = tickData.length > 1 ? 
      (tickData[1].logPrice - tickData[0].logPrice) : 0.02;
    
    // Convert to chart coordinates
    const barWidth = (logPriceSpacing / (priceExtent[1] - priceExtent[0])) * chartArea.height;
    
    // Ensure minimum width for visibility but cap maximum
    return Math.max(1, Math.min(barWidth * 0.8, 20));
  };

  // Generate SVG path for cumulative liquidity line
  const generateCumulativePath = () => {
    if (cumulativeLiquidityData.length === 0) return '';
    
    const points = cumulativeLiquidityData.map(d => 
      `${xScaleCumulative(d.cumulativeLiquidity)},${yScale(d.logPrice)}`
    );
    return `M ${points.join(' L ')}`;
  };

  // Generate tick marks and labels for individual liquidity
  const generateXTicks = () => {
    const numTicks = 5;
    const ticks = [];
    for (let i = 0; i <= numTicks; i++) {
      const value = (liquidityExtent[1] / numTicks) * i;
      const x = xScale(value);
      ticks.push({
        x,
        value,
        label: value >= 1000000 ? `${(value / 1000000).toFixed(1)}M` : 
               value >= 1000 ? `${(value / 1000).toFixed(0)}K` : 
               value.toFixed(0)
      });
    }
    return ticks;
  };

  // Generate tick marks and labels for cumulative liquidity (top axis)
  const generateXTicksCumulative = () => {
    const numTicks = 5;
    const ticks = [];
    for (let i = 0; i <= numTicks; i++) {
      const value = (cumulativeExtent[1] / numTicks) * i;
      const x = xScaleCumulative(value);
      ticks.push({
        x,
        value,
        label: value >= 1000000 ? `${(value / 1000000).toFixed(1)}M` : 
               value >= 1000 ? `${(value / 1000).toFixed(0)}K` : 
               value.toFixed(0)
      });
    }
    return ticks;
  };

  const generateYTicks = () => {
    const numTicks = 6;
    const ticks = [];
    for (let i = 0; i <= numTicks; i++) {
      const logPrice = priceExtent[0] + ((priceExtent[1] - priceExtent[0]) / numTicks) * i;
      const price = Math.exp(logPrice);
      const y = yScale(logPrice);
      ticks.push({
        y,
        logPrice,
        price,
        label: price >= 1000 ? `${(price / 1000).toFixed(1)}K` :
               price >= 1 ? price.toFixed(2) :
               price.toFixed(4)
      });
    }
    return ticks;
  };

  if (loading) {
    return (
      <div id="liquidity-loading-container" className="card">
        <div className="p-8">
          <div className="flex items-center justify-center">
            <div className="flex flex-col items-center space-y-4">
              <div id="liquidity-loading-spinner" className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary"></div>
              <p id="liquidity-loading-text" className="text-muted-foreground">Loading tick liquidity data...</p>
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div id="liquidity-error-container" className="card">
        <div className="p-6">
          <h2 id="liquidity-error-title" className="text-xl font-medium mb-4 flex items-center gap-2">
            <span className="text-xl">Warning</span>
            Data Error
          </h2>
          <p id="liquidity-error-message" className="text-muted-foreground">{error}</p>
        </div>
      </div>
    );
  }

  return (
    <Card id="liquidity-visualization-container" style={{ isolation: 'isolate' }}>
      <CardHeader>
        <div id="liquidity-header" className="flex justify-between items-center">
          <CardTitle className="flex items-center gap-2">
            <Waves className="h-5 w-5" />
            Tick Liquidity
          </CardTitle>
          <div id="liquidity-metrics" className="flex items-center space-x-4 text-sm text-muted-foreground">
            <span id="total-ticks-metric">Total Ticks: {tickData.length}</span>
            <span>•</span>
            <span id="max-liquidity-metric">Max Liquidity: {liquidityExtent[1].toLocaleString()}</span>
            <span>•</span>
            <span id="max-swappable-metric">Max Swappable: {cumulativeExtent[1].toLocaleString()}</span>
          </div>
        </div>
      </CardHeader>
      <CardContent>

        {/* Interactive Legend */}
        <div id="liquidity-legend-container" className="flex justify-center mb-4">
          <div id="liquidity-legend" className="flex items-center space-x-6 text-sm">
            <div id="regular-ticks-legend-item" className="flex items-center space-x-2">
              <input
                type="checkbox"
                id="regular-ticks-checkbox"
                checked={showRegularTicks}
                onChange={(e) => setShowRegularTicks(e.target.checked)}
                className="w-4 h-4 rounded-full appearance-none border-2 border-gray-400 checked:bg-gray-700 checked:border-gray-700 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-offset-2"
                style={{ 
                  backgroundImage: showRegularTicks ? 'radial-gradient(circle, #374151 30%, transparent 30%)' : 'none',
                  backgroundPosition: 'center',
                  backgroundRepeat: 'no-repeat',
                  backgroundColor: showRegularTicks ? 'white' : 'transparent'
                }}
              />
              <div id="regular-ticks-color-indicator" className="w-4 h-3 bg-primary opacity-80 rounded-sm"></div>
              <label htmlFor="regular-ticks-checkbox" className="cursor-pointer">Regular Ticks</label>
            </div>
            <div id="floor-tick-legend-item" className="flex items-center space-x-2">
              <input
                type="checkbox"
                id="floor-tick-checkbox"
                checked={showFloorTick}
                onChange={(e) => setShowFloorTick(e.target.checked)}
                className="w-4 h-4 rounded-full appearance-none border-2 border-gray-400 checked:bg-gray-700 checked:border-gray-700 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-offset-2"
                style={{ 
                  backgroundImage: showFloorTick ? 'radial-gradient(circle, #374151 30%, transparent 30%)' : 'none',
                  backgroundPosition: 'center',
                  backgroundRepeat: 'no-repeat',
                  backgroundColor: showFloorTick ? 'white' : 'transparent'
                }}
              />
              <div id="floor-tick-color-indicator" className="w-4 h-3 rounded-sm opacity-80" style={{ backgroundColor: 'hsl(15, 100%, 60%)' }}></div>
              <label htmlFor="floor-tick-checkbox" className="cursor-pointer">Pool-Owned Floor Tick</label>
            </div>
            <div id="gtwap-legend-item" className="flex items-center space-x-2">
              <input
                type="checkbox"
                id="gtwap-checkbox"
                checked={showGtwapLine}
                onChange={(e) => setShowGtwapLine(e.target.checked)}
                className="w-4 h-4 rounded-full appearance-none border-2 border-gray-400 checked:bg-gray-700 checked:border-gray-700 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-offset-2"
                style={{ 
                  backgroundImage: showGtwapLine ? 'radial-gradient(circle, #374151 30%, transparent 30%)' : 'none',
                  backgroundPosition: 'center',
                  backgroundRepeat: 'no-repeat',
                  backgroundColor: showGtwapLine ? 'white' : 'transparent'
                }}
              />
              <div id="gtwap-color-indicator" className="w-4 h-1 rounded-sm" style={{ backgroundColor: 'hsl(280, 100%, 70%)', borderStyle: 'dashed', borderWidth: '1px' }}></div>
              <label htmlFor="gtwap-checkbox" className="cursor-pointer">GTWAP Virtual JIT</label>
            </div>
            <div id="cumulative-liquidity-legend-item" className="flex items-center space-x-2">
              <input
                type="checkbox"
                id="cumulative-liquidity-checkbox"
                checked={showCumulativeLiquidity}
                onChange={(e) => setShowCumulativeLiquidity(e.target.checked)}
                className="w-4 h-4 rounded-full appearance-none border-2 border-gray-400 checked:bg-gray-700 checked:border-gray-700 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-offset-2"
                style={{ 
                  backgroundImage: showCumulativeLiquidity ? 'radial-gradient(circle, #374151 30%, transparent 30%)' : 'none',
                  backgroundPosition: 'center',
                  backgroundRepeat: 'no-repeat',
                  backgroundColor: showCumulativeLiquidity ? 'white' : 'transparent'
                }}
              />
              <div id="cumulative-liquidity-color-indicator" className="w-4 h-1 rounded-sm" style={{ backgroundColor: 'hsl(220, 100%, 60%)' }}></div>
              <label htmlFor="cumulative-liquidity-checkbox" className="cursor-pointer">Cumulative Liquidity</label>
            </div>
          </div>
        </div>

        <div id="liquidity-chart-wrapper" className="w-full flex justify-center">
          <svg 
            id="liquidity-chart-svg"
            width={chartDimensions.width} 
            height={chartDimensions.height}
            className="bg-background"
          >
            <defs>
              <linearGradient id="liquidity-gradient" x1="0%" y1="0%" x2="100%" y2="0%">
                <stop offset="0%" style={{ stopColor: 'hsl(var(--primary))', stopOpacity: 0.1 }} />
                <stop offset="100%" style={{ stopColor: 'hsl(var(--primary))', stopOpacity: 0.3 }} />
              </linearGradient>
            </defs>

            {/* Chart area */}
            <g id="liquidity-chart-area" transform={`translate(${chartDimensions.padding.left}, ${chartDimensions.padding.top})`}>
              
              {/* Grid lines */}
              <g id="liquidity-grid-lines">
                {generateXTicks().map((tick, i) => (
                  <line
                    key={`x-grid-${i}`}
                    id={`x-grid-line-${i}`}
                    x1={tick.x}
                    y1={0}
                    x2={tick.x}
                    y2={chartArea.height}
                    stroke="hsl(var(--border))"
                    strokeDasharray="2,2"
                    opacity={0.3}
                  />
                ))}
                
                {generateYTicks().map((tick, i) => {
                  // Only draw grid lines within the padded area
                  if (tick.y >= verticalPadding && tick.y <= chartArea.height - verticalPadding) {
                    return (
                      <line
                        key={`y-grid-${i}`}
                        id={`y-grid-line-${i}`}
                        x1={0}
                        y1={tick.y}
                        x2={chartArea.width}
                        y2={tick.y}
                        stroke="hsl(var(--border))"
                        strokeDasharray="2,2"
                        opacity={0.3}
                      />
                    );
                  }
                  return null;
                })}
              </g>

              {/* Liquidity bars */}
              <g id="liquidity-bars">
                {tickData.map((d, i) => {
                  const barWidth = getBarWidth();
                  const barHeight = xScale(d.liquidity);
                  const x = 0;
                  const centerY = yScale(d.logPrice);
                  
                  // Ensure bars don't extend beyond the padded area
                  const halfBarWidth = barWidth / 2;
                  const topEdge = Math.max(verticalPadding, centerY - halfBarWidth);
                  const bottomEdge = Math.min(chartArea.height - verticalPadding, centerY + halfBarWidth);
                  const clampedHeight = bottomEdge - topEdge;
                  
                  // Only render if the bar is visible within bounds
                  if (clampedHeight <= 0) return null;
                  
                  // Different color for floor tick
                  const barColor = d.isFloorTick ? 'hsl(15, 100%, 60%)' : 'hsl(var(--primary))';
                  
                  // Show/hide based on toggle state
                  const shouldShow = d.isFloorTick ? showFloorTick : showRegularTicks;
                  
                  if (!shouldShow) return null;
                  
                  return (
                    <rect
                      key={i}
                      id={`liquidity-bar-${i}${d.isFloorTick ? '-floor' : ''}`}
                      x={x}
                      y={topEdge}
                      width={barHeight}
                      height={clampedHeight}
                      fill={barColor}
                      opacity={0.8}
                      stroke={barColor}
                      strokeWidth={0.5}
                    >
                      <title>
                        {d.isFloorTick ? 'FLOOR TICK - ' : ''}Tick: {d.tick}, Price: ${d.price.toFixed(4)}, Liquidity: {d.liquidity.toLocaleString()}
                      </title>
                    </rect>
                  );
                })}
              </g>

              {/* Floor Tick Label */}
              {showFloorTick && tickData.find(d => d.isFloorTick) && (
                <text
                  id="floor-tick-label"
                  x={xScale(tickData.find(d => d.isFloorTick)!.liquidity) + 10}
                  y={yScale(tickData.find(d => d.isFloorTick)!.logPrice) + 4}
                  textAnchor="start"
                  fontSize="12"
                  fill="hsl(15, 100%, 60%)"
                  fontWeight="500"
                >
                  Floor: {tickData.find(d => d.isFloorTick)!.liquidity.toLocaleString()}
                </text>
              )}

              {/* Cumulative Liquidity Line */}
              {showCumulativeLiquidity && cumulativeLiquidityData.length > 0 && (
                <path
                  id="cumulative-liquidity-path"
                  d={generateCumulativePath()}
                  fill="none"
                  stroke="hsl(220, 100%, 60%)"
                  strokeWidth={2}
                  opacity={0.8}
                />
              )}

              {/* GTWAP Virtual JIT Liquidity Line */}
              {showGtwapLine && poolMetrics && (
                <g id="gtwap-line-group">
                  <line
                    id="gtwap-line"
                    x1={0}
                    y1={yScale(Math.log(poolMetrics.gtwapPrice))}
                    x2={chartArea.width}
                    y2={yScale(Math.log(poolMetrics.gtwapPrice))}
                    stroke="hsl(280, 100%, 70%)"
                    strokeWidth={2}
                    strokeDasharray="8,4"
                  />
                  <text
                    id="gtwap-label"
                    x={chartArea.width - 10}
                    y={yScale(Math.log(poolMetrics.gtwapPrice)) - 8}
                    textAnchor="end"
                    fontSize="12"
                    fill="hsl(280, 100%, 70%)"
                    fontWeight="500"
                  >
                    GTWAP: {poolMetrics.virtualJitDepth.toLocaleString()}
                  </text>
                </g>
              )}

              {/* Axes */}
              <g id="chart-axes">
                <line id="x-axis" x1={0} y1={chartArea.height - verticalPadding} x2={chartArea.width} y2={chartArea.height - verticalPadding} stroke="hsl(var(--foreground))" strokeWidth={1} />
                <line id="y-axis" x1={0} y1={verticalPadding} x2={0} y2={chartArea.height - verticalPadding} stroke="hsl(var(--foreground))" strokeWidth={1} />
                {/* Top axis for cumulative */}
                {showCumulativeLiquidity && (
                  <line id="x-axis-cumulative" x1={0} y1={verticalPadding} x2={chartArea.width} y2={verticalPadding} stroke="hsl(220, 100%, 60%)" strokeWidth={1} />
                )}
              </g>
            </g>

            {/* X-axis labels */}
            <g id="x-axis-labels">
              {generateXTicks().map((tick, i) => (
                <g key={`x-label-${i}`} id={`x-axis-label-${i}`}>
                  <line
                    id={`x-axis-tick-${i}`}
                    x1={chartDimensions.padding.left + tick.x}
                    y1={chartDimensions.padding.top + chartArea.height - verticalPadding}
                    x2={chartDimensions.padding.left + tick.x}
                    y2={chartDimensions.padding.top + chartArea.height - verticalPadding + 5}
                    stroke="hsl(var(--foreground))"
                  />
                  <text
                    id={`x-axis-text-${i}`}
                    x={chartDimensions.padding.left + tick.x}
                    y={chartDimensions.padding.top + chartArea.height - verticalPadding + 20}
                    textAnchor="middle"
                    fontSize="12"
                    fill="hsl(var(--muted-foreground))"
                  >
                    {tick.label}
                  </text>
                </g>
              ))}
            </g>

            {/* Y-axis labels */}
            <g id="y-axis-labels">
              {generateYTicks().map((tick, i) => (
                <g key={`y-label-${i}`} id={`y-axis-label-${i}`}>
                  <line
                    id={`y-axis-tick-${i}`}
                    x1={chartDimensions.padding.left - 5}
                    y1={chartDimensions.padding.top + tick.y}
                    x2={chartDimensions.padding.left}
                    y2={chartDimensions.padding.top + tick.y}
                    stroke="hsl(var(--foreground))"
                  />
                  <text
                    id={`y-axis-text-${i}`}
                    x={chartDimensions.padding.left - 10}
                    y={chartDimensions.padding.top + tick.y + 4}
                    textAnchor="end"
                    fontSize="12"
                    fill="hsl(var(--muted-foreground))"
                  >
                    ${tick.label}
                  </text>
                </g>
              ))}
            </g>

            {/* Top X-axis labels for cumulative liquidity */}
            {showCumulativeLiquidity && (
              <g id="cumulative-x-axis-labels">
                {generateXTicksCumulative().map((tick, i) => (
                  <g key={`x-cum-label-${i}`} id={`cumulative-x-axis-label-${i}`}>
                    <line
                      id={`cumulative-x-axis-tick-${i}`}
                      x1={chartDimensions.padding.left + tick.x}
                      y1={chartDimensions.padding.top + verticalPadding - 5}
                      x2={chartDimensions.padding.left + tick.x}
                      y2={chartDimensions.padding.top + verticalPadding}
                      stroke="hsl(220, 100%, 60%)"
                    />
                    <text
                      id={`cumulative-x-axis-text-${i}`}
                      x={chartDimensions.padding.left + tick.x}
                      y={chartDimensions.padding.top - 8}
                      textAnchor="middle"
                      fontSize="12"
                      fill="hsl(220, 100%, 60%)"
                    >
                      {tick.label}
                    </text>
                  </g>
                ))}
              </g>
            )}

            {/* Axis titles */}
            <text
              id="x-axis-title"
              x={chartDimensions.width / 2}
              y={chartDimensions.height - 10}
              textAnchor="middle"
              fontSize="14"
              fill="hsl(var(--foreground))"
              fontWeight="500"
            >
              Individual Liquidity
            </text>

            {showCumulativeLiquidity && (
              <text
                id="cumulative-x-axis-title"
                x={chartDimensions.width / 2}
                y={25}
                textAnchor="middle"
                fontSize="14"
                fill="hsl(220, 100%, 60%)"
                fontWeight="500"
              >
                Cumulative Liquidity
              </text>
            )}
            
            <text
              id="y-axis-title"
              x={20}
              y={chartDimensions.height / 2}
              textAnchor="middle"
              fontSize="14"
              fill="hsl(var(--foreground))"
              fontWeight="500"
              transform={`rotate(-90 20 ${chartDimensions.height / 2})`}
            >
              log(Price)
            </text>

          </svg>
        </div>
      </CardContent>
    </Card>
  );
}