import { useState, useEffect, useCallback } from 'react';
import { useIndexerClient } from './useIndexer';
import { SwapQuoteResponse } from '@/services/indexer-client';
import { useConnection, useWallet } from '@solana/wallet-adapter-react';
import { PublicKey } from '@solana/web3.js';

export interface SwapQuoteParams {
  fromToken?: string;
  toToken?: string;
  amount?: string;
  slippageBps?: number;
}

export interface UseSwapQuoteResult {
  quote: SwapQuoteResponse | null;
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

export function useSwapQuote(params: SwapQuoteParams): UseSwapQuoteResult {
  const [quote, setQuote] = useState<SwapQuoteResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  const { client, isConnected } = useIndexerClient();

  const fetchQuote = useCallback(async () => {
    if (!params.fromToken || !params.toToken || !params.amount || params.amount === '0') {
      setQuote(null);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      // If indexer is not connected, generate a mock quote
      if (!isConnected || !client) {
        // Mock quote for development
        const mockQuote: SwapQuoteResponse = {
          amount_in: params.amount,
          amount_out: (parseFloat(params.amount) * 0.95).toFixed(6), // 5% slippage mock
          min_amount_out: (parseFloat(params.amount) * 0.94).toFixed(6),
          fee_amount: (parseFloat(params.amount) * 0.003).toFixed(6), // 0.3% fee
          price_impact_bps: 50, // 0.5%
          execution_price: 1.05,
          route: [{
            from_token: params.fromToken,
            to_token: params.toToken,
            market_address: 'mock-market',
            protocol: 'Feels',
            amount_in: params.amount,
            amount_out: (parseFloat(params.amount) * 0.95).toFixed(6),
          }],
          market_price: 1.0,
        };
        setQuote(mockQuote);
        return;
      }

      const response = await client.getSwapQuote({
        amount_in: params.amount,
        token_in: params.fromToken,
        token_out: params.toToken,
        slippage_bps: params.slippageBps,
      });

      setQuote(response);
    } catch (err) {
      console.error('Failed to fetch swap quote:', err);
      setError('Failed to fetch quote');
      
      // Provide fallback mock quote on error
      const mockQuote: SwapQuoteResponse = {
        amount_in: params.amount,
        amount_out: (parseFloat(params.amount) * 0.95).toFixed(6),
        min_amount_out: (parseFloat(params.amount) * 0.94).toFixed(6),
        fee_amount: (parseFloat(params.amount) * 0.003).toFixed(6),
        price_impact_bps: 50,
        execution_price: 1.05,
        route: [{
          from_token: params.fromToken,
          to_token: params.toToken,
          market_address: 'mock-market',
          protocol: 'Feels',
          amount_in: params.amount,
          amount_out: (parseFloat(params.amount) * 0.95).toFixed(6),
        }],
        market_price: 1.0,
      };
      setQuote(mockQuote);
    } finally {
      setLoading(false);
    }
  }, [params.fromToken, params.toToken, params.amount, params.slippageBps, client, isConnected]);

  useEffect(() => {
    fetchQuote();
  }, [fetchQuote]);

  return {
    quote,
    loading,
    error,
    refresh: fetchQuote,
  };
}

// Hook for fetching token balances
export function useTokenBalances(wallet?: string) {
  const [balances, setBalances] = useState<Record<string, number>>({});
  const [loading, setLoading] = useState(false);
  const { publicKey } = useWallet();
  const { client, isConnected } = useIndexerClient();
  
  const walletAddress = wallet || publicKey?.toBase58();

  const fetchBalances = useCallback(async () => {
    if (!walletAddress) {
      setBalances({});
      return;
    }

    setLoading(true);

    try {
      if (!isConnected || !client) {
        // Mock balances for development
        setBalances({
          'So11111111111111111111111111111111111111112': 10.0, // SOL
          'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v': 100.0, // USDC
          'FEELsso1VoSkqwJQsYq3h3mBGsVZcKbXgssbKdZrmMad': 50.0, // FeelsSOL
        });
        return;
      }

      const response = await client.getWalletBalances(walletAddress);
      
      const balanceMap: Record<string, number> = {};
      response.balances.forEach(b => {
        balanceMap[b.mint] = b.ui_balance;
      });
      
      setBalances(balanceMap);
    } catch (err) {
      console.error('Failed to fetch balances:', err);
      // Use mock balances as fallback
      setBalances({
        'So11111111111111111111111111111111111111112': 10.0,
        'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v': 100.0,
        'FEELsso1VoSkqwJQsYq3h3mBGsVZcKbXgssbKdZrmMad': 50.0,
      });
    } finally {
      setLoading(false);
    }
  }, [walletAddress, client, isConnected]);

  useEffect(() => {
    fetchBalances();
    
    // Refresh balances every 10 seconds
    const interval = setInterval(fetchBalances, 10000);
    return () => clearInterval(interval);
  }, [fetchBalances]);

  return {
    balances,
    loading,
    refresh: fetchBalances,
  };
}