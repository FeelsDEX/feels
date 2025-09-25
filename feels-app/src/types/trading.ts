// Trading related types shared across the app

// Candle data used by the simulated chart
export interface KLineData {
  timestamp: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  turnover: number;
  [key: string]: number | undefined;
}

