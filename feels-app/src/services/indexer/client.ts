// HTTP client for Feels Protocol Indexer API
import type {
  IndexerConfig,
  ProtocolStats,
  IndexedMarket,
  MarketStats,
  IndexedSwap,
  IndexedFloor,
  OHLCVCandle,
  SwapQuoteResponse,
  SwapSimulationResponse,
  TokenBalanceResponse,
  TokenBalancesResponse,
  BuildSwapTransactionRequest,
  BuildSwapTransactionResponse,
  SimulateTransactionRequest,
  SimulateTransactionResponse,
  EntryExitQuoteResponse,
} from './types';

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
        // Special case: Return empty array for 404 on list endpoints without throwing
        if (response.status === 404 && endpoint === '/markets') {
          return [] as T;
        }
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      return await response.json();
    } catch (error) {
      // Suppress console output for expected 404s on markets endpoint
      if (error instanceof Error && 
          error.message.includes('Failed to fetch') && 
          endpoint === '/markets') {
        // Return empty array for expected markets endpoint failures
        return [] as T;
      }
      
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
    try {
      const response = await this.request<{ markets: IndexedMarket[], total: number, limit: number, offset: number }>('/markets');
      return response.markets || [];
    } catch (error) {
      // Silently return empty array for markets endpoint when unavailable
      return [];
    }
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

// Factory function for creating client instances
export const createIndexerClient = (baseUrl?: string): FeelsIndexerClient => {
  return new FeelsIndexerClient({
    // Use Next.js proxy to avoid CORS issues in development
    baseUrl: baseUrl || '/api/indexer',
  });
};

