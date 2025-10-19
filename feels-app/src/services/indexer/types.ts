// Type definitions for Feels Protocol Indexer API

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

// Market data types
export interface IndexedMarket {
  address: string;
  token_0: string;
  token_1: string;
  sqrt_price: string;
  liquidity: string;
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

