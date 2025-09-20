// Feels Protocol Indexer API Client

export interface IndexerConfig {
  baseUrl: string;
  timeout?: number;
}

export interface IndexedMarket {
  address: string;
  token_0: string;
  token_1: string;
  sqrt_price: string; // u128 as string
  liquidity: string; // u128 as string
  current_tick: number;
  tick_spacing: number;
  fee_bps: number;
  is_paused: boolean;
  phase: 'PriceDiscovery' | 'SteadyState';
  last_updated_slot: number;
  last_updated_timestamp: number;
}

export interface IndexedSwap {
  signature: string;
  slot: number;
  timestamp: number;
  market: string;
  user: string;
  token_in: string;
  token_out: string;
  amount_in: number;
  amount_out: number;
  fee_amount: number;
  price_before: number;
  price_after: number;
  liquidity_before: string;
  liquidity_after: string;
}

export interface IndexedFloor {
  market: string;
  current_floor_tick: number;
  current_floor_price: number;
  jitosol_reserves: string;
  circulating_supply: string;
  last_ratchet_slot: number;
  floor_buffer: number;
  history: FloorUpdate[];
}

export interface FloorUpdate {
  slot: number;
  timestamp: number;
  old_floor_tick: number;
  new_floor_tick: number;
  trigger: 'Ratchet' | 'Manual';
}

export interface ProtocolStats {
  total_markets: number;
  total_volume_24h: number;
  total_fees_24h: number;
  total_liquidity: string;
  active_positions: number;
  last_updated: number;
}

export interface OHLCVCandle {
  timestamp: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

export interface MarketStats {
  volume_24h: number;
  fees_24h: number;
  swaps_24h: number;
  price_change_24h: number;
  liquidity_change_24h: number;
}

export class FeelsIndexerClient {
  private config: IndexerConfig;

  constructor(config: IndexerConfig) {
    this.config = {
      timeout: 10000,
      ...config,
    };
  }

  private async request<T>(endpoint: string): Promise<T> {
    const url = `${this.config.baseUrl}${endpoint}`;
    
    try {
      const response = await fetch(url, {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
        signal: AbortSignal.timeout(this.config.timeout!),
      });

      if (!response.ok) {
        // Special case: Return empty array for 404 on list endpoints
        if (response.status === 404 && endpoint === '/markets') {
          return [] as T;
        }
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      return await response.json();
    } catch (error) {
      if (error instanceof Error) {
        throw new Error(`Indexer API request failed: ${error.message}`);
      }
      throw error;
    }
  }

  // Protocol-level endpoints
  async getProtocolStats(): Promise<ProtocolStats> {
    return this.request<ProtocolStats>('/protocol/stats');
  }

  // Market endpoints
  async getMarkets(): Promise<IndexedMarket[]> {
    return this.request<IndexedMarket[]>('/markets');
  }

  async getMarket(address: string): Promise<IndexedMarket> {
    return this.request<IndexedMarket>(`/markets/${address}`);
  }

  async getMarketStats(address: string): Promise<MarketStats> {
    return this.request<MarketStats>(`/markets/${address}/stats`);
  }

  // Swap endpoints
  async getMarketSwaps(
    address: string, 
    options?: {
      limit?: number;
      offset?: number;
      since?: number;
    }
  ): Promise<IndexedSwap[]> {
    const params = new URLSearchParams();
    if (options?.limit) params.set('limit', options.limit.toString());
    if (options?.offset) params.set('offset', options.offset.toString());
    if (options?.since) params.set('since', options.since.toString());
    
    const query = params.toString() ? `?${params.toString()}` : '';
    return this.request<IndexedSwap[]>(`/markets/${address}/swaps${query}`);
  }

  // Floor liquidity endpoints
  async getMarketFloor(address: string): Promise<IndexedFloor> {
    return this.request<IndexedFloor>(`/markets/${address}/floor`);
  }

  // OHLCV data
  async getMarketOHLCV(
    address: string,
    options?: {
      interval?: '1m' | '5m' | '15m' | '1h' | '4h' | '1d';
      limit?: number;
      since?: number;
    }
  ): Promise<OHLCVCandle[]> {
    const params = new URLSearchParams();
    if (options?.interval) params.set('interval', options.interval);
    if (options?.limit) params.set('limit', options.limit.toString());
    if (options?.since) params.set('since', options.since.toString());
    
    const query = params.toString() ? `?${params.toString()}` : '';
    return this.request<OHLCVCandle[]>(`/markets/${address}/ohlcv${query}`);
  }

  // Health check
  async getHealth(): Promise<{ status: string; timestamp: number }> {
    return this.request<{ status: string; timestamp: number }>('/health');
  }
}

// Default client configuration
// Note: The indexer is optional - if not running, features that depend on it will gracefully degrade
export const createIndexerClient = (baseUrl?: string): FeelsIndexerClient => {
  return new FeelsIndexerClient({
    baseUrl: baseUrl || process.env.NEXT_PUBLIC_INDEXER_URL || 'http://localhost:8080',
  });
};
