// Generates unified historical price data with appropriate granularity for different time horizons
import { KLineData } from '@/types/trading';

export interface HistoricalDataPoint {
  timestamp: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  turnover: number;
}

export interface UnifiedHistoricalData {
  minuteData: HistoricalDataPoint[]; // Last 24 hours, 1-minute candles
  hourlyData: HistoricalDataPoint[]; // Last 7 days, 1-hour candles
  dailyData: HistoricalDataPoint[]; // Last 90 days, daily candles
  weeklyData: HistoricalDataPoint[]; // Last 2 years, weekly candles
  floorPrices: { timestamp: number; value: number }[];
  gtwapPrices: { timestamp: number; value: number }[];
  currentPrice: number;
  floorPrice: number;
  gtwapPrice: number;
}

interface GeneratorParams {
  tokenAddress: string;
  // basePrice?: number;
  volatility?: number;
  trend?: number;
}

// Brownian motion with drift
function generateBrownianMotion(
  current: number,
  volatility: number,
  drift: number,
  timeStep: number
): number {
  const randomShock = (Math.random() - 0.5) * 2; // -1 to 1
  const brownianComponent = volatility * randomShock * Math.sqrt(timeStep);
  const driftComponent = drift * timeStep;

  return current * Math.exp(driftComponent + brownianComponent);
}

// Generate realistic OHLC from price movements
function generateOHLC(
  openPrice: number,
  closePrice: number,
  volatility: number,
  timeSpan: number
): { high: number; low: number } {
  const priceRange = Math.abs(closePrice - openPrice);
  const additionalRange = Math.max(priceRange, openPrice * volatility * Math.sqrt(timeSpan));

  // Ensure high/low extend beyond open/close
  const high = Math.max(openPrice, closePrice) + Math.random() * 0.5 * additionalRange;
  const low = Math.min(openPrice, closePrice) - Math.random() * 0.5 * additionalRange;

  return { high, low };
}

// Volume generation with price-volume correlation
function generateVolume(
  priceChange: number,
  baseVolume: number,
  timeOfDay?: number // 0-24 for hourly patterns
): number {
  // Higher volume on larger price moves
  const priceImpact = 1 + Math.abs(priceChange) * 10;

  // Intraday volume pattern (if applicable)
  let timeMultiplier = 1;
  if (timeOfDay !== undefined) {
    // Peak volumes at market open (9-10am) and close (3-4pm)
    if ((timeOfDay >= 9 && timeOfDay <= 10) || (timeOfDay >= 15 && timeOfDay <= 16)) {
      timeMultiplier = 1.5 + Math.random() * 0.5;
    } else if (timeOfDay >= 11 && timeOfDay <= 14) {
      // Lunch time lull
      timeMultiplier = 0.7 + Math.random() * 0.3;
    }
  }

  // Random variation
  const randomFactor = 0.5 + Math.random() * 1.5;

  return baseVolume * priceImpact * timeMultiplier * randomFactor;
}

// Generate monotonically increasing floor price with steady growth
function generateFloorPrice(
  previousFloor: number,
  daysSinceStart: number,
  // initialFloorPrice: number,
  seed: number
): number {
  // Start with a reasonable initial floor price (10% of initial price)
  if (previousFloor === 0 && daysSinceStart >= 0) {
    return 0.1; // Start at 0.1 (10% of initial price which is 1)
  }
  
  // Base daily increase rate - more aggressive for visible growth
  const baseRate = 0.001; // 0.1% base daily increase (5x the original)

  // Add subtle variations to make it less linear but still monotonic
  const weekNumber = Math.floor(daysSinceStart / 7);
  const monthNumber = Math.floor(daysSinceStart / 30);

  // Create gentle variations (reduced amplitude for steadier growth)
  const weeklyVariation = Math.sin(weekNumber * 0.5 + seed * 0.1) * 0.2 + 1; // ±20% variation
  const monthlyVariation = Math.cos(monthNumber * 0.3 + seed * 0.2) * 0.15 + 1; // ±15% variation
  const dailyNoise = Math.sin(daysSinceStart * 0.1 + seed * 0.05) * 0.05 + 1; // ±5% variation

  // Combine variations for subtle but always positive growth
  const effectiveRate = baseRate * weeklyVariation * monthlyVariation * dailyNoise;

  // Ensure minimum growth to maintain steady monotonic increase
  const minRate = baseRate * 0.5; // At least 50% of base rate (more aggressive minimum)
  const finalRate = Math.max(effectiveRate, minRate);

  // Calculate new floor price (always increasing)
  const newFloor = previousFloor * (1 + finalRate);

  return newFloor;
}

// Calculate GTWAP (Geometric Time-Weighted Average Price)
function calculateGTWAP(candles: HistoricalDataPoint[]): number {
  if (candles.length === 0) return 0;

  let totalTimeWeight = 0;
  let weightedLogPriceSum = 0;

  for (let i = 0; i < candles.length; i++) {
    const candle = candles[i];
    if (!candle) continue;
    const timeWeight = i === candles.length - 1 ? 1 : 1; // Equal weighting for simplicity
    const logPrice = Math.log(candle.close);

    weightedLogPriceSum += logPrice * timeWeight;
    totalTimeWeight += timeWeight;
  }

  const avgLogPrice = weightedLogPriceSum / totalTimeWeight;
  return Math.exp(avgLogPrice);
}

export function generateUnifiedHistoricalData({
  tokenAddress,
  // basePrice,
  volatility = 0.02, // 2% base volatility
  trend = 0.0001, // 0.01% daily trend
}: GeneratorParams): UnifiedHistoricalData {
  const now = Date.now();

  // Deterministic randomness based on token address
  const seed = tokenAddress.split('').reduce((sum, char) => sum + char.charCodeAt(0), 0);
  const priceScenario = seed % 5;

  // Volume scenarios (price always starts at 1)
  const volumeScenarios = [
    { volumeBase: 1e6 }, // Micro cap
    { volumeBase: 5e6 }, // Small cap
    { volumeBase: 10e6 }, // Mid cap
    { volumeBase: 50e6 }, // Large cap
    { volumeBase: 100e6 }, // Mega cap
  ];

  const scenario = volumeScenarios[priceScenario] || volumeScenarios[0]!;
  const initialPrice = 1; // Always start at price 1

  // Generate data forwards from 2 years ago
  // const allData: HistoricalDataPoint[] = [];
  let currentPrice = initialPrice; // Start at 1

  // Start from 2 years ago and work forward
  const twoYearsAgo = now - 365 * 2 * 24 * 60 * 60 * 1000;
  // let timestamp = twoYearsAgo;

  // Weekly data for 2 years (104 weeks)
  const weeklyData: HistoricalDataPoint[] = [];
  for (let i = 0; i < 104; i++) {
    const weekStart = twoYearsAgo + i * 7 * 24 * 60 * 60 * 1000;
    if (weekStart >= now) break;

    const openPrice = currentPrice;
    const weeklyDrift = trend * 7;
    const weeklyVolatility = volatility * Math.sqrt(7);

    currentPrice = generateBrownianMotion(currentPrice, weeklyVolatility, weeklyDrift, 1);
    const { high, low } = generateOHLC(openPrice, currentPrice, weeklyVolatility, 7);

    const priceChange = (currentPrice - openPrice) / openPrice;
    const volume = generateVolume(priceChange, scenario.volumeBase * 7);

    weeklyData.push({
      timestamp: weekStart,
      open: openPrice,
      high,
      low,
      close: currentPrice,
      volume,
      turnover: volume * currentPrice,
    });
  }

  // Daily data for last 90 days
  const ninetyDaysAgo = now - 90 * 24 * 60 * 60 * 1000;
  const dailyData: HistoricalDataPoint[] = [];

  // Find starting price from weekly data
  const weeklyAtNinetyDays = weeklyData.find((w) => w.timestamp >= ninetyDaysAgo);
  if (weeklyAtNinetyDays) {
    currentPrice = weeklyAtNinetyDays.close;
  }

  for (let i = 0; i < 90; i++) {
    const dayStart = ninetyDaysAgo + i * 24 * 60 * 60 * 1000;
    if (dayStart >= now) break;

    const openPrice = currentPrice;
    const dailyVolatility = volatility;

    currentPrice = generateBrownianMotion(currentPrice, dailyVolatility, trend, 1);
    const { high, low } = generateOHLC(openPrice, currentPrice, dailyVolatility, 1);

    const priceChange = (currentPrice - openPrice) / openPrice;
    const volume = generateVolume(priceChange, scenario.volumeBase);

    dailyData.push({
      timestamp: dayStart,
      open: openPrice,
      high,
      low,
      close: currentPrice,
      volume,
      turnover: volume * currentPrice,
    });
  }

  // Hourly data for last 7 days
  const sevenDaysAgo = now - 7 * 24 * 60 * 60 * 1000;
  const hourlyData: HistoricalDataPoint[] = [];

  // Find starting price from daily data
  const dailyAtSevenDays = dailyData.find((d) => d.timestamp >= sevenDaysAgo);
  if (dailyAtSevenDays) {
    currentPrice = dailyAtSevenDays.close;
  }

  for (let i = 0; i < 7 * 24; i++) {
    const hourStart = sevenDaysAgo + i * 60 * 60 * 1000;
    if (hourStart >= now) break;

    const openPrice = currentPrice;
    const hourlyVolatility = volatility / Math.sqrt(24);
    const hourlyDrift = trend / 24;
    const hourOfDay = new Date(hourStart).getHours();

    currentPrice = generateBrownianMotion(currentPrice, hourlyVolatility, hourlyDrift, 1);
    const { high, low } = generateOHLC(openPrice, currentPrice, hourlyVolatility, 1 / 24);

    const priceChange = (currentPrice - openPrice) / openPrice;
    const volume = generateVolume(priceChange, scenario.volumeBase / 24, hourOfDay);

    hourlyData.push({
      timestamp: hourStart,
      open: openPrice,
      high,
      low,
      close: currentPrice,
      volume,
      turnover: volume * currentPrice,
    });
  }

  // Minute data for last 24 hours
  const oneDayAgo = now - 24 * 60 * 60 * 1000;
  const minuteData: HistoricalDataPoint[] = [];

  // Find starting price from hourly data
  const hourlyAtOneDay = hourlyData.find((h) => h.timestamp >= oneDayAgo);
  if (hourlyAtOneDay) {
    currentPrice = hourlyAtOneDay.close;
  }

  for (let i = 0; i < 24 * 60; i++) {
    const minuteStart = oneDayAgo + i * 60 * 1000;
    if (minuteStart >= now) break;

    const openPrice = currentPrice;
    const minuteVolatility = volatility / Math.sqrt(24 * 60);
    const minuteDrift = trend / (24 * 60);

    currentPrice = generateBrownianMotion(currentPrice, minuteVolatility, minuteDrift, 1);
    const { high, low } = generateOHLC(openPrice, currentPrice, minuteVolatility, 1 / (24 * 60));

    const priceChange = (currentPrice - openPrice) / openPrice;
    const volume = generateVolume(priceChange, scenario.volumeBase / (24 * 60));

    minuteData.push({
      timestamp: minuteStart,
      open: openPrice,
      high,
      low,
      close: currentPrice,
      volume,
      turnover: volume * currentPrice,
    });
  }

  // Generate floor and GTWAP series
  const floorPrices: { timestamp: number; value: number }[] = [];
  const gtwapPrices: { timestamp: number; value: number }[] = [];

  // Combine all data for floor/GTWAP calculation
  const allCandles = [...weeklyData, ...dailyData, ...hourlyData, ...minuteData].sort(
    (a, b) => a.timestamp - b.timestamp
  );

  let runningGTWAP = initialPrice;
  // const initialFloorPrice = 0.1; // Start floor at 0.1 (10% of initial price)
  let previousFloor = 0; // Start at 0 to trigger initial value in generateFloorPrice

  allCandles.forEach((candle, index) => {
    const daysSinceStart = (candle.timestamp - twoYearsAgo) / (24 * 60 * 60 * 1000);

    // Generate monotonically increasing floor price
    const floorPrice = generateFloorPrice(previousFloor, daysSinceStart, seed);
    previousFloor = floorPrice; // Update for next iteration

    floorPrices.push({
      timestamp: candle.timestamp,
      value: floorPrice,
    });

    // Calculate running GTWAP
    const recentCandles = allCandles.slice(Math.max(0, index - 20), index + 1);
    runningGTWAP = calculateGTWAP(recentCandles);
    
    // Ensure GTWAP is always >= floor price
    const adjustedGTWAP = Math.max(runningGTWAP, floorPrice);

    gtwapPrices.push({
      timestamp: candle.timestamp,
      value: adjustedGTWAP,
    });
  });

  const finalPrice = currentPrice;
  const finalFloor = floorPrices[floorPrices.length - 1]?.value || 0.1;
  const finalGTWAP = Math.max(
    gtwapPrices[gtwapPrices.length - 1]?.value || initialPrice,
    finalFloor
  );

  return {
    minuteData,
    hourlyData,
    dailyData,
    weeklyData,
    floorPrices,
    gtwapPrices,
    currentPrice: finalPrice,
    floorPrice: finalFloor,
    gtwapPrice: finalGTWAP,
  };
}

// Get appropriate data for a specific time range
export function getDataForTimeRange(
  data: UnifiedHistoricalData,
  timeRange: string
): {
  klineData: KLineData[];
  floorSeries: { timestamp: number; value: number }[];
  gtwapSeries: { timestamp: number; value: number }[];
} {
  const now = Date.now();
  let klineData: KLineData[] = [];
  let startTime: number;

  switch (timeRange) {
    case '1m':
      startTime = now - 60 * 60 * 1000; // Last hour
      klineData = data.minuteData.filter((d) => d.timestamp >= startTime) as KLineData[];
      break;
    case '5m':
      startTime = now - 5 * 60 * 60 * 1000; // Last 5 hours
      // Aggregate minute data into 5-minute candles
      klineData = aggregateCandles(
        data.minuteData.filter((d) => d.timestamp >= startTime),
        5 * 60 * 1000
      ) as KLineData[];
      break;
    case '15m':
      startTime = now - 15 * 60 * 60 * 1000; // Last 15 hours
      // Aggregate minute data into 15-minute candles
      klineData = aggregateCandles(
        data.minuteData.filter((d) => d.timestamp >= startTime),
        15 * 60 * 1000
      ) as KLineData[];
      break;
    case '1h':
      startTime = now - 72 * 60 * 60 * 1000; // Last 3 days
      klineData = data.hourlyData.filter((d) => d.timestamp >= startTime) as KLineData[];
      break;
    case '6h':
      startTime = now - 7 * 24 * 60 * 60 * 1000; // Last week
      // Aggregate hourly data into 6-hour candles
      klineData = aggregateCandles(
        data.hourlyData.filter((d) => d.timestamp >= startTime),
        6 * 60 * 60 * 1000
      ) as KLineData[];
      break;
    case '1D':
      startTime = now - 30 * 24 * 60 * 60 * 1000; // Last 30 days
      klineData = data.dailyData.filter((d) => d.timestamp >= startTime) as KLineData[];
      break;
    case '1W':
      startTime = now - 52 * 7 * 24 * 60 * 60 * 1000; // Last year
      klineData = data.weeklyData.filter((d) => d.timestamp >= startTime) as KLineData[];
      break;
    case '1M':
      startTime = now - 24 * 30 * 24 * 60 * 60 * 1000; // Last 2 years
      // Aggregate weekly data into monthly candles
      klineData = aggregateCandles(data.weeklyData, 30 * 24 * 60 * 60 * 1000) as KLineData[];
      break;
    case 'all':
      // Use all weekly data
      klineData = data.weeklyData as KLineData[];
      startTime = data.weeklyData[0]?.timestamp || now;
      break;
    default:
      // Default to daily view
      startTime = now - 30 * 24 * 60 * 60 * 1000;
      klineData = data.dailyData.filter((d) => d.timestamp >= startTime) as KLineData[];
  }

  // Filter floor and GTWAP series to match time range
  const floorSeries = data.floorPrices.filter((p) => p.timestamp >= startTime);
  const gtwapSeries = data.gtwapPrices.filter((p) => p.timestamp >= startTime);

  return {
    klineData,
    floorSeries,
    gtwapSeries,
  };
}

// Aggregate candles into larger time periods
function aggregateCandles(candles: HistoricalDataPoint[], intervalMs: number): KLineData[] {
  if (candles.length === 0) return [];

  const aggregated: KLineData[] = [];
  let currentBucket: HistoricalDataPoint[] = [];
  let bucketStart = Math.floor(candles[0]!.timestamp / intervalMs) * intervalMs;

  candles.forEach((candle) => {
    const candleBucket = Math.floor(candle.timestamp / intervalMs) * intervalMs;

    if (candleBucket !== bucketStart && currentBucket.length > 0) {
      // Process current bucket
      const open = currentBucket[0]!.open;
      const close = currentBucket[currentBucket.length - 1]!.close;
      const high = Math.max(...currentBucket.map((c) => c.high));
      const low = Math.min(...currentBucket.map((c) => c.low));
      const volume = currentBucket.reduce((sum, c) => sum + c.volume, 0);
      const turnover = currentBucket.reduce((sum, c) => sum + c.turnover, 0);

      aggregated.push({
        timestamp: bucketStart,
        open,
        high,
        low,
        close,
        volume,
        turnover,
      });

      // Start new bucket
      currentBucket = [];
      bucketStart = candleBucket;
    }

    currentBucket.push(candle);
  });

  // Process last bucket
  if (currentBucket.length > 0) {
    const open = currentBucket[0]!.open;
    const close = currentBucket[currentBucket.length - 1]!.close;
    const high = Math.max(...currentBucket.map((c) => c.high));
    const low = Math.min(...currentBucket.map((c) => c.low));
    const volume = currentBucket.reduce((sum, c) => sum + c.volume, 0);
    const turnover = currentBucket.reduce((sum, c) => sum + c.turnover, 0);

    aggregated.push({
      timestamp: bucketStart,
      open,
      high,
      low,
      close,
      volume,
      turnover,
    });
  }

  return aggregated;
}
