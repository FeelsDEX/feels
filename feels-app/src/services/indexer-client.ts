// Feels Protocol Indexer API Client

export interface IndexerConfig {
  baseUrl: string;
  timeout?: number;
}

// Swap quote and simulation types
export interface SwapRoute {
  from_token: string;
  to_token: string;
  market_address: string;
  protocol: string;
  amount_in: string;
  amount_out: string;
}

export interface SwapQuoteResponse {
  amount_in: string;
  amount_out: string;
  min_amount_out: string;
  fee_amount: string;
  price_impact_bps: number;
  execution_price: number;
  route: SwapRoute[];
  market_price: number;
  slippage_warning?: string;
}

export interface SwapSimulationResponse {
  amount_in: string;
  amount_out: string;
  fee_paid: string;
  price_before: number;
  price_after: number;
  price_impact_percent: number;
  start_tick: number;
  end_tick: number;
  ticks_crossed: number;
  end_liquidity: string;
}

// Token balance types
export interface TokenBalanceResponse {
  mint: string;
  symbol?: string;
  balance: string;
  ui_balance: number;
  decimals: number;
}

export interface TokenBalancesResponse {
  wallet: string;
  balances: TokenBalanceResponse[];
  total_count: number;
}

// Transaction building types
export interface BuildSwapTransactionRequest {
  wallet: string;
  market_address: string;
  amount_in: string;
  min_amount_out: string;
  is_token_0_to_1: boolean;
  user_token_in: string;
  user_token_out: string;
  referrer?: string;
  priority_fee_microlamports?: number;
}

export interface BuildSwapTransactionResponse {
  transaction: string;
  compute_units: number;
  priority_fee: number;
  expires_at: number;
  instructions_summary: string[];
  signers: string[];
}

export interface SimulateTransactionRequest {
  transaction: string;
  include_logs?: boolean;
}

export interface SimulateTransactionResponse {
  success: boolean;
  error?: string;
  logs?: string[];
  units_consumed?: number;
  accounts: string[];
}

// Jupiter integration types
export interface EntryExitQuoteResponse {
  input_mint: string;
  output_mint: string;
  in_amount: string;
  out_amount: string;
  min_out_amount: string;
  price_impact_bps: number;
  price: number;
  route: RouteStep[];
  uses_jupiter: boolean;
}

export interface RouteStep {
  input_mint: string;
  output_mint: string;
  amm: string;
  in_amount: string;
  out_amount: string;
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

  private async request<T>(
    endpoint: string, 
    options?: { 
      method?: string; 
      body?: string; 
      params?: Record<string, any> 
    }
  ): Promise<T> {
    let url = `${this.config.baseUrl}${endpoint}`;
    
    // Add query parameters for GET requests
    if (options?.params && (!options.method || options.method === 'GET')) {
      const params = new URLSearchParams();
      Object.entries(options.params).forEach(([key, value]) => {
        if (value !== undefined) {
          params.set(key, value.toString());
        }
      });
      const query = params.toString();
      if (query) {
        url += `?${query}`;
      }
    }
    
    try {
      const response = await fetch(url, {
        method: options?.method || 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
        body: options?.body,
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
    const response = await this.request<{ markets: IndexedMarket[], total: number, limit: number, offset: number }>('/markets');
    return response.markets;
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

  // Swap quote and simulation
  async getSwapQuote(params: {
    amount_in: string;
    token_in: string;
    token_out: string;
    slippage_bps?: number;
  }): Promise<SwapQuoteResponse> {
    return this.request<SwapQuoteResponse>('/swap/quote', { params });
  }

  async simulateSwap(params: {
    market_address: string;
    amount_in: string;
    is_token_0_to_1: boolean;
  }): Promise<SwapSimulationResponse> {
    return this.request<SwapSimulationResponse>('/swap/simulate', {
      method: 'POST',
      body: JSON.stringify(params),
    });
  }

  // Token balances
  async getTokenBalance(mint: string, wallet: string): Promise<TokenBalanceResponse> {
    return this.request<TokenBalanceResponse>(`/tokens/${mint}/balance/${wallet}`);
  }

  async getWalletBalances(wallet: string): Promise<TokenBalancesResponse> {
    return this.request<TokenBalancesResponse>(`/wallets/${wallet}/balances`);
  }

  // Transaction building
  async buildSwapTransaction(
    params: BuildSwapTransactionRequest
  ): Promise<BuildSwapTransactionResponse> {
    return this.request<BuildSwapTransactionResponse>('/swap/build', {
      method: 'POST',
      body: JSON.stringify(params),
    });
  }

  async simulateTransaction(
    params: SimulateTransactionRequest
  ): Promise<SimulateTransactionResponse> {
    return this.request<SimulateTransactionResponse>('/tx/simulate', {
      method: 'POST',
      body: JSON.stringify(params),
    });
  }

  // Jupiter integration for entry/exit
  async getEntryQuote(params: {
    input_mint: string;
    amount: string;
    slippage_bps?: number;
  }): Promise<EntryExitQuoteResponse> {
    return this.request<EntryExitQuoteResponse>('/entry/quote', { params });
  }

  async getExitQuote(params: {
    output_mint: string;
    amount: string;
    slippage_bps?: number;
  }): Promise<EntryExitQuoteResponse> {
    return this.request<EntryExitQuoteResponse>('/exit/quote', { params });
  }
}

// WebSocket subscription types
export type SubscriptionType = 
  | { type: 'all_markets' }
  | { type: 'market'; address: string }
  | { type: 'swaps'; market?: string }
  | { type: 'positions'; user?: string }
  | { type: 'floor_updates'; market: string }
  | { type: 'price_updates'; market: string };

// WebSocket update events
export type UpdateEvent = 
  | {
      type: 'market_update';
      market: string;
      sqrt_price: string;
      liquidity: string;
      current_tick: number;
      timestamp: number;
    }
  | {
      type: 'swap_event';
      market: string;
      user: string;
      amount_in: string;
      amount_out: string;
      token_in: string;
      token_out: string;
      price: number;
      timestamp: number;
    }
  | {
      type: 'position_update';
      position: string;
      market: string;
      owner: string;
      liquidity: string;
      tick_lower: number;
      tick_upper: number;
      timestamp: number;
    }
  | {
      type: 'floor_update';
      market: string;
      new_floor_tick: number;
      new_floor_price: number;
      timestamp: number;
    }
  | {
      type: 'price_update';
      market: string;
      price: number;
      price_change_24h: number;
      timestamp: number;
    }
  | {
      type: 'subscribed';
      id: string;
      subscriptions: SubscriptionType[];
    }
  | {
      type: 'error';
      code: string;
      message: string;
    };

// WebSocket client for real-time updates
export class FeelsWebSocketClient {
  private ws?: WebSocket;
  private url: string;
  private reconnectTimeout?: NodeJS.Timeout;
  private subscriptions: Map<string, SubscriptionType[]> = new Map();
  private eventHandlers: Map<string, (event: UpdateEvent) => void> = new Map();
  
  constructor(url: string) {
    this.url = url.replace('http:', 'ws:').replace('https:', 'wss:');
  }
  
  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      // Only use WebSocket in browser environment
      if (typeof window === 'undefined') {
        reject(new Error('WebSocket is not available in server environment'));
        return;
      }
      
      try {
        this.ws = new WebSocket(`${this.url}/ws`);
        
        this.ws.onopen = () => {
          console.log('WebSocket connected');
          // Resubscribe to all previous subscriptions
          for (const [id, subs] of this.subscriptions) {
            this.ws?.send(JSON.stringify({
              type: 'subscribe',
              id,
              subscriptions: subs,
            }));
          }
          resolve();
        };
        
        this.ws.onmessage = (event) => {
          try {
            const update = JSON.parse(event.data) as UpdateEvent;
            // Dispatch to all registered handlers
            for (const handler of this.eventHandlers.values()) {
              handler(update);
            }
          } catch (e) {
            console.error('Failed to parse WebSocket message:', e);
          }
        };
        
        this.ws.onerror = (error) => {
          console.error('WebSocket error:', error);
          reject(error);
        };
        
        this.ws.onclose = () => {
          console.log('WebSocket disconnected');
          // Attempt reconnection after 5 seconds
          this.reconnectTimeout = setTimeout(() => {
            this.connect().catch(console.error);
          }, 5000);
        };
      } catch (error) {
        reject(error);
      }
    });
  }
  
  disconnect(): void {
    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
    }
    if (this.ws) {
      this.ws.close();
      this.ws = undefined;
    }
  }
  
  subscribe(id: string, subscriptions: SubscriptionType[]): void {
    this.subscriptions.set(id, subscriptions);
    // Only check WebSocket state in browser environment
    if (typeof window !== 'undefined' && this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({
        type: 'subscribe',
        id,
        subscriptions,
      }));
    }
  }
  
  unsubscribe(id: string): void {
    const subscriptions = this.subscriptions.get(id);
    // Only check WebSocket state in browser environment
    if (typeof window !== 'undefined' && subscriptions && this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({
        type: 'unsubscribe',
        id,
        subscriptions,
      }));
    }
    this.subscriptions.delete(id);
  }
  
  onUpdate(id: string, handler: (event: UpdateEvent) => void): void {
    this.eventHandlers.set(id, handler);
  }
  
  removeHandler(id: string): void {
    this.eventHandlers.delete(id);
  }
}

// Default client configuration
// Note: The indexer is optional - if not running, features that depend on it will gracefully degrade
export const createIndexerClient = (baseUrl?: string): FeelsIndexerClient => {
  return new FeelsIndexerClient({
    baseUrl: baseUrl || process.env['NEXT_PUBLIC_INDEXER_URL'] || 'http://localhost:8080',
  });
};

// Create WebSocket client
export const createWebSocketClient = (baseUrl?: string): FeelsWebSocketClient => {
  const url = baseUrl || process.env['NEXT_PUBLIC_INDEXER_URL'] || 'http://localhost:8080';
  return new FeelsWebSocketClient(url);
};
