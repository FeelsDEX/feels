// App configuration for Feels Protocol

export const appConfig = {
  // Default token address for swap page
  // Set this to a specific token address to always navigate to that token's page when clicking "swap"
  // If not set (null), a random token will be selected from available tokens
  defaultSwapTokenAddress: process.env.NEXT_PUBLIC_DEFAULT_SWAP_TOKEN_ADDRESS || null,
  
  // Alternative: Use token symbol instead of address
  defaultSwapTokenSymbol: process.env.NEXT_PUBLIC_DEFAULT_SWAP_TOKEN_SYMBOL || null,
};

export type AppConfig = typeof appConfig;