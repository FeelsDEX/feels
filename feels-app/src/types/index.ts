// Centralized type definitions for the Feels app

// Re-export token types from constants
export type { Token } from '@/constants/mock-tokens';

// Market types
export interface Market {
  address: string;
  token0: string;
  token1: string;
  token0Symbol: string;
  token1Symbol: string;
  fee: number;
  tickSpacing: number;
  liquidity: string;
  sqrtPriceX64: string;
  currentTick: number;
  volume24h: string;
  tvl: string;
}

// Pool types
export interface Pool {
  id: string;
  address: string;
  token0: TokenInfo;
  token1: TokenInfo;
  fee: string;
  tvl: string;
  volume24h: string;
  apr: string;
}

// Token info for pools/swaps
export interface TokenInfo {
  address: string;
  symbol: string;
  name: string;
  decimals: number;
  logoURI?: string;
  isFeelsToken?: boolean;
}

// Swap types
export interface SwapQuote {
  inputAmount: string;
  outputAmount: string;
  priceImpact: number;
  route: SwapRoute;
  fee: number;
}

export interface SwapRoute {
  type: 'jupiter-direct' | 'jupiter-to-jitosol' | 'full-feels-onboard' | 'feels-to-meme' | 'full-chain';
  steps: SwapStep[];
}

export interface SwapStep {
  from: string;
  to: string;
  protocol: 'jupiter' | 'feels';
  amount: string;
}

// Transaction types
export interface RecentSwap {
  signature: string;
  timestamp: number;
  from: string;
  to: string;
  fromAmount: string;
  toAmount: string;
  fromSymbol: string;
  toSymbol: string;
  trader: string;
  txUrl: string;
}

// Position types
export interface Position {
  id: string;
  owner: string;
  pool: string;
  tickLower: number;
  tickUpper: number;
  liquidity: string;
  token0Locked: string;
  token1Locked: string;
  token0Symbol: string;
  token1Symbol: string;
  usdValue: string;
}

// Protocol stats
export interface ProtocolStats {
  totalValueLocked: string;
  volume24h: string;
  fees24h: string;
  totalPools: number;
  totalPositions: number;
  totalSwaps24h: number;
}

// Search and faceting
export type { 
  TokenSearchResult, 
  FacetConfig, 
  SelectedFacets 
} from './search';