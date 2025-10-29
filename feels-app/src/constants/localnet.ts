/**
 * Localnet configuration and dynamic token addresses
 * 
 * This module provides access to dynamically created JitoSOL, FeelsSOL, and Metaplex
 * program IDs for local testing. The actual addresses are generated when running setup scripts.
 */

import { PublicKey } from '@solana/web3.js';

// Standard Metaplex Token Metadata program ID (mainnet/devnet)
const METAPLEX_MAINNET_ID = 'metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s';

// Import config synchronously for server-side rendering
// Note: Config files are generated at runtime by setup scripts
let localnetConfig: any = {};

// Default to test values or loaded config
let JITOSOL_MINT = localnetConfig.jitosol?.mint || 'So11111111111111111111111111111111111111112';
let FEELSSOL_MINT = localnetConfig.feelssol?.mint || 'So11111111111111111111111111111111111111112';
let localnetMetaplexId: string | null = null;

// Store promises for async loading
let tokensPromise: Promise<void> | null = null;
let metaplexPromise: Promise<void> | null = null;

// Try to load from generated configs if available on client
if (typeof window !== 'undefined') {
  // Client-side: Try to fetch from API routes for dynamic updates
  tokensPromise = fetch('/api/localnet-tokens')
    .then(res => res.json())
    .then(config => {
      if (config.jitosol?.mint) {
        JITOSOL_MINT = config.jitosol.mint;
      }
      if (config.feelssol?.mint) {
        FEELSSOL_MINT = config.feelssol.mint;
      }
    })
    .catch(() => {
      // Ignore errors, use defaults
    });

  metaplexPromise = fetch('/api/metaplex-config')
    .then(res => res.json())
    .then(config => {
      if (config.programId) {
        localnetMetaplexId = config.programId;
      }
    })
    .catch(() => {
      // Ignore errors
    });
}

/**
 * Ensure localnet tokens are loaded
 */
export async function ensureLocalnetTokensLoaded(): Promise<void> {
  if (tokensPromise) {
    await tokensPromise;
  }
}

/**
 * Ensure Metaplex config is loaded (for async operations)
 */
export async function ensureMetaplexConfigLoaded(): Promise<void> {
  if (metaplexPromise) {
    await metaplexPromise;
  }
}

/**
 * Get the JitoSOL mint address for the current environment
 */
export function getJitoSOLMint(): PublicKey {
  const network = process.env['NEXT_PUBLIC_NETWORK'] || 'localnet';
  
  if (network === 'mainnet-beta') {
    // Real JitoSOL on mainnet
    return new PublicKey('J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn');
  }
  
  // Local test JitoSOL
  return new PublicKey(JITOSOL_MINT);
}

/**
 * Get the FeelsSOL mint address for the current environment
 */
export function getFeelsSOLMint(): PublicKey {
  return new PublicKey(FEELSSOL_MINT);
}

/**
 * Get the Metaplex Token Metadata program ID for the current network
 */
export function getMetaplexProgramId(): PublicKey {
  const network = process.env['NEXT_PUBLIC_NETWORK'] || 'localnet';
  
  if (network === 'localnet' && localnetMetaplexId) {
    return new PublicKey(localnetMetaplexId);
  }
  
  // Default to standard Metaplex ID for all other networks
  return new PublicKey(METAPLEX_MAINNET_ID);
}

/**
 * Check if we're using localnet tokens
 */
export function isLocalnet(): boolean {
  const network = process.env['NEXT_PUBLIC_NETWORK'] || 'localnet';
  return network === 'localnet';
}

