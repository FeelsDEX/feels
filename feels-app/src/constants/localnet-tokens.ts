/**
 * Utilities for working with localnet tokens
 * 
 * This module provides access to the dynamically created JitoSOL and FeelsSOL
 * tokens for local testing. The actual addresses are generated when running
 * the setup:jitosol script.
 */

import { PublicKey } from '@solana/web3.js';

// Import config synchronously for server-side rendering
let localnetConfig: any = {};
try {
  // Try to load config at build time
  localnetConfig = require('../../scripts/localnet-tokens.json');
} catch (e) {
  // Config not available yet
}

// Default to test values or loaded config
let JITOSOL_MINT = localnetConfig.jitosol?.mint || 'So11111111111111111111111111111111111111112';
let FEELSSOL_MINT = localnetConfig.feelssol?.mint || '11111111111111111111111111111112';

// Store promise for async loading
let configPromise: Promise<void> | null = null;

// Try to load from generated config if available on client
if (typeof window !== 'undefined') {
  // Client-side: Try to fetch from API route for dynamic updates
  configPromise = fetch('/api/localnet-tokens')
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
}

/**
 * Ensure localnet tokens are loaded
 */
export async function ensureLocalnetTokensLoaded(): Promise<void> {
  if (configPromise) {
    await configPromise;
  }
}

/**
 * Get the JitoSOL mint address for the current environment
 */
export function getJitoSOLMint(): PublicKey {
  const network = process.env.NEXT_PUBLIC_NETWORK || 'localnet';
  
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
 * Check if we're using localnet tokens
 */
export function isLocalnet(): boolean {
  const network = process.env.NEXT_PUBLIC_NETWORK || 'localnet';
  return network === 'localnet';
}