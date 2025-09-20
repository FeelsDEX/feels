/**
 * Jupiter Swap API Client for Feels Protocol
 * 
 * Implements the Jupiter Swap API to enable users to swap any token
 * (especially SOL) to JitoSOL before depositing into Feels Protocol.
 * 
 * Based on Jupiter Swap API documentation:
 * https://dev.jup.ag/docs/swap-api/
 */

import { Connection, PublicKey, Transaction, VersionedTransaction } from '@solana/web3.js';
import solanaLogoImage from '@/assets/images/solana_logo.svg';

// Jupiter API base URL
const JUPITER_API_BASE = 'https://quote-api.jup.ag/v6';

// Common token addresses on Solana
export const TOKENS = {
  SOL: 'So11111111111111111111111111111111111111112', // Wrapped SOL
  JITOSOL: 'J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn', // JitoSOL
  USDC: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', // USDC
  USDT: 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB', // USDT
} as const;

export interface JupiterQuoteResponse {
  inputMint: string;
  inAmount: string;
  outputMint: string;
  outAmount: string;
  otherAmountThreshold: string;
  swapMode: string;
  slippageBps: number;
  platformFee?: {
    amount: string;
    feeBps: number;
  };
  priceImpactPct: string;
  routePlan: Array<{
    swapInfo: {
      ammKey: string;
      label: string;
      inputMint: string;
      outputMint: string;
      inAmount: string;
      outAmount: string;
      feeAmount: string;
      feeMint: string;
    };
    percent: number;
  }>;
  contextSlot: number;
  timeTaken: number;
}

export interface JupiterSwapRequest {
  quoteResponse: JupiterQuoteResponse;
  userPublicKey: string;
  wrapAndUnwrapSol?: boolean;
  useSharedAccounts?: boolean;
  feeAccount?: string;
  trackingAccount?: string;
  computeUnitPriceMicroLamports?: number;
  prioritizationFeeLamports?: number;
  asLegacyTransaction?: boolean;
  useTokenLedger?: boolean;
  destinationTokenAccount?: string;
}

export interface JupiterSwapResponse {
  swapTransaction: string;
  lastValidBlockHeight: number;
  prioritizationFeeLamports: number;
  computeUnitLimit: number;
  prioritizationType: {
    computeBudget: {
      microLamports: number;
      estimatedFeeInSOL: number;
    };
  };
  dynamicSlippageReport?: {
    otherAmountThreshold: string;
    slippageBps: number;
  };
}

export interface TokenInfo {
  address: string;
  symbol: string;
  name: string;
  decimals: number;
  logoURI?: string;
  tags?: string[];
  isFeelsToken?: boolean;
}

export class JupiterClient {
  private baseUrl: string;

  constructor(baseUrl: string = JUPITER_API_BASE) {
    this.baseUrl = baseUrl;
  }

  /**
   * Get a quote for swapping tokens
   * 
   * @param inputMint - Input token mint address
   * @param outputMint - Output token mint address  
   * @param amount - Amount to swap (in token's smallest unit)
   * @param slippageBps - Slippage tolerance in basis points (default: 50 = 0.5%)
   * @param swapMode - 'ExactIn' or 'ExactOut' (default: 'ExactIn')
   * @param onlyDirectRoutes - Only use direct routes (default: false)
   * @param asLegacyTransaction - Return legacy transaction format (default: false)
   */
  async getQuote(
    inputMint: string,
    outputMint: string,
    amount: string,
    slippageBps: number = 50,
    swapMode: 'ExactIn' | 'ExactOut' = 'ExactIn',
    onlyDirectRoutes: boolean = false,
    asLegacyTransaction: boolean = false
  ): Promise<JupiterQuoteResponse> {
    const params = new URLSearchParams({
      inputMint,
      outputMint,
      amount,
      slippageBps: slippageBps.toString(),
      swapMode,
      onlyDirectRoutes: onlyDirectRoutes.toString(),
      asLegacyTransaction: asLegacyTransaction.toString(),
    });

    const response = await fetch(`${this.baseUrl}/quote?${params}`);
    
    if (!response.ok) {
      const errorText = await response.text();
      
      // Try to parse JSON error for cleaner messages
      try {
        const errorData = JSON.parse(errorText);
        if (errorData.error === "Input and output mints are not allowed to be equal") {
          throw new Error("From and To tokens must be different");
        }
        if (errorData.errorCode === "CIRCULAR_ARBITRAGE_IS_DISABLED") {
          throw new Error("From and To tokens must be different");
        }
        // Handle other known error patterns
        if (errorData.error) {
          throw new Error(errorData.error);
        }
        throw new Error(`Jupiter quote failed: ${response.status}`);
      } catch (parseError) {
        // Check if parseError is the intentional error we threw above
        if (parseError instanceof Error && parseError.message.includes("From and To tokens must be different")) {
          throw parseError;
        }
        // If JSON parsing fails, fall back to a cleaner generic error
        throw new Error("Failed to get swap quote. Please try again.");
      }
    }

    return response.json();
  }

  /**
   * Build a swap transaction from a quote
   * 
   * @param swapRequest - Swap request parameters including quote and user info
   */
  async getSwapTransaction(swapRequest: JupiterSwapRequest): Promise<JupiterSwapResponse> {
    const response = await fetch(`${this.baseUrl}/swap`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(swapRequest),
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Jupiter swap transaction failed: ${response.status} ${errorText}`);
    }

    return response.json();
  }

  /**
   * Get a quote and build transaction in one call (convenience method)
   * 
   * @param inputMint - Input token mint address
   * @param outputMint - Output token mint address
   * @param amount - Amount to swap
   * @param userPublicKey - User's wallet public key
   * @param slippageBps - Slippage tolerance in basis points
   * @param priorityFeeLamports - Priority fee for faster execution
   */
  async getQuoteAndSwap(
    inputMint: string,
    outputMint: string,
    amount: string,
    userPublicKey: string,
    slippageBps: number = 50,
    priorityFeeLamports: number = 0
  ): Promise<{ quote: JupiterQuoteResponse; swap: JupiterSwapResponse }> {
    // Get quote first
    const quote = await this.getQuote(inputMint, outputMint, amount, slippageBps);

    // Build swap transaction
    const swapRequest: JupiterSwapRequest = {
      quoteResponse: quote,
      userPublicKey,
      wrapAndUnwrapSol: true, // Automatically handle SOL wrapping
      useSharedAccounts: true, // Use shared accounts for better efficiency
      prioritizationFeeLamports: priorityFeeLamports,
      asLegacyTransaction: false, // Use versioned transactions for better efficiency
    };

    const swap = await this.getSwapTransaction(swapRequest);

    return { quote, swap };
  }

  /**
   * Deserialize a swap transaction for signing
   * 
   * @param swapTransaction - Base64 encoded transaction from Jupiter
   * @param connection - Solana connection for recent blockhash
   */
  async deserializeSwapTransaction(
    swapTransaction: string,
    connection: Connection
  ): Promise<VersionedTransaction> {
    const transactionBuf = Buffer.from(swapTransaction, 'base64');
    return VersionedTransaction.deserialize(transactionBuf);
  }

  /**
   * Execute a complete swap flow: quote → build → sign → send
   * 
   * @param inputMint - Input token mint
   * @param outputMint - Output token mint  
   * @param amount - Amount to swap
   * @param connection - Solana connection
   * @param wallet - Wallet adapter for signing
   * @param slippageBps - Slippage tolerance
   * @param priorityFeeLamports - Priority fee
   */
  async executeSwap(
    inputMint: string,
    outputMint: string,
    amount: string,
    connection: Connection,
    wallet: any, // Wallet adapter
    slippageBps: number = 50,
    priorityFeeLamports: number = 0
  ): Promise<string> {
    if (!wallet.publicKey) {
      throw new Error('Wallet not connected');
    }

    // Get quote and swap transaction
    const { quote, swap } = await this.getQuoteAndSwap(
      inputMint,
      outputMint,
      amount,
      wallet.publicKey.toString(),
      slippageBps,
      priorityFeeLamports
    );

    // Deserialize transaction
    const transaction = await this.deserializeSwapTransaction(swap.swapTransaction, connection);

    // Sign transaction
    const signedTransaction = await wallet.signTransaction(transaction);

    // Send transaction
    const signature = await connection.sendRawTransaction(signedTransaction.serialize(), {
      skipPreflight: false,
      preflightCommitment: 'confirmed',
    });

    // Wait for confirmation
    await connection.confirmTransaction(signature, 'confirmed');

    return signature;
  }

  /**
   * Get popular tokens list for token selection
   */
  async getTokenList(): Promise<TokenInfo[]> {
    // For now, return a curated list of popular tokens
    // In production, you might want to fetch from Jupiter's token list API
    return [
      {
        address: TOKENS.SOL,
        symbol: 'SOL',
        name: 'Solana',
        decimals: 9,
        logoURI: solanaLogoImage.src,
      },
      {
        address: TOKENS.JITOSOL,
        symbol: 'JitoSOL',
        name: 'Jito Staked SOL',
        decimals: 9,
        logoURI: 'https://storage.googleapis.com/token-metadata/JitoSOL-256.png',
      },
      {
        address: TOKENS.USDC,
        symbol: 'USDC',
        name: 'USD Coin',
        decimals: 6,
        logoURI: 'https://assets.coingecko.com/coins/images/6319/large/usdc.png',
      },
      {
        address: TOKENS.USDT,
        symbol: 'USDT',
        name: 'Tether USD',
        decimals: 6,
        logoURI: 'https://assets.coingecko.com/coins/images/325/large/Tether.png',
      },
    ];
  }

  /**
   * Helper: Convert human-readable amount to token's smallest unit
   * 
   * @param amount - Human readable amount (e.g., "1.5")
   * @param decimals - Token decimals
   */
  static toTokenAmount(amount: string, decimals: number): string {
    const factor = Math.pow(10, decimals);
    return Math.floor(parseFloat(amount) * factor).toString();
  }

  /**
   * Helper: Convert token's smallest unit to human-readable amount
   * 
   * @param amount - Amount in smallest unit
   * @param decimals - Token decimals
   */
  static fromTokenAmount(amount: string, decimals: number): string {
    const factor = Math.pow(10, decimals);
    return (parseInt(amount) / factor).toString();
  }

  /**
   * Helper: Format price impact percentage for display
   * 
   * @param priceImpactPct - Price impact as string percentage
   */
  static formatPriceImpact(priceImpactPct: string): string {
    const impact = parseFloat(priceImpactPct);
    if (impact < 0.01) return '<0.01%';
    return `${impact.toFixed(2)}%`;
  }

  /**
   * Helper: Calculate minimum output amount considering slippage
   * 
   * @param outputAmount - Expected output amount
   * @param slippageBps - Slippage in basis points
   */
  static calculateMinimumOutput(outputAmount: string, slippageBps: number): string {
    const amount = parseInt(outputAmount);
    const slippageMultiplier = (10000 - slippageBps) / 10000;
    return Math.floor(amount * slippageMultiplier).toString();
  }
}
