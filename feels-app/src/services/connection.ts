import { Connection, clusterApiUrl } from '@solana/web3.js';

// Singleton connection instance
let connection: Connection | null = null;

export function getConnection(cluster: 'devnet' | 'mainnet-beta' = 'devnet'): Connection {
  if (!connection) {
    connection = new Connection(clusterApiUrl(cluster), {
      commitment: 'confirmed',
      // Enable preflight checks caching
      confirmTransactionInitialTimeout: 60000,
      // WebSocket config for better performance
      wsEndpoint: cluster === 'devnet' 
        ? 'wss://api.devnet.solana.com' 
        : 'wss://api.mainnet-beta.solana.com'
    });
  }
  return connection;
}

// Pre-warm the connection only after user interaction or longer delay
if (typeof window !== 'undefined') {
  // Wait for page to be fully loaded and user to potentially interact
  const warmConnection = () => {
    const conn = getConnection();
    // Warm up the connection with a lightweight call
    conn.getSlot().catch(() => {
      // Ignore errors during warm-up
    });
  };

  // Option 1: Warm on first user interaction
  let hasWarmed = false;
  const warmOnInteraction = () => {
    if (!hasWarmed) {
      hasWarmed = true;
      warmConnection();
      // Remove listeners after warming
      document.removeEventListener('click', warmOnInteraction);
      document.removeEventListener('keydown', warmOnInteraction);
    }
  };
  
  document.addEventListener('click', warmOnInteraction, { once: true });
  document.addEventListener('keydown', warmOnInteraction, { once: true });
  
  // Option 2: Fallback warm after 5 seconds if no interaction
  setTimeout(() => {
    if (!hasWarmed) {
      hasWarmed = true;
      warmConnection();
    }
  }, 5000);
}