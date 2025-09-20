/**
 * Metaplex Token Metadata configuration for different environments
 */

import { PublicKey } from '@solana/web3.js';

// Standard Metaplex Token Metadata program ID (mainnet/devnet)
const METAPLEX_MAINNET_ID = 'metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s';

// Load localnet config if available
let localnetMetaplexId: string | null = null;

// Try to load from API on client side
if (typeof window !== 'undefined') {
  fetch('/api/metaplex-config')
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
 * Get the Metaplex Token Metadata program ID for the current network
 */
export function getMetaplexProgramId(): PublicKey {
  const network = process.env.NEXT_PUBLIC_NETWORK || 'localnet';
  
  if (network === 'localnet' && localnetMetaplexId) {
    return new PublicKey(localnetMetaplexId);
  }
  
  // Default to standard Metaplex ID for all other networks
  return new PublicKey(METAPLEX_MAINNET_ID);
}

/**
 * Ensure Metaplex config is loaded (for async operations)
 */
export async function ensureMetaplexConfigLoaded(): Promise<void> {
  if (typeof window !== 'undefined' && !localnetMetaplexId) {
    try {
      const response = await fetch('/api/metaplex-config');
      const config = await response.json();
      if (config.programId) {
        localnetMetaplexId = config.programId;
      }
    } catch {
      // Ignore errors
    }
  }
}