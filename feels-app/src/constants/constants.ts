// Protocol constants
export const PROTOCOL_CONSTANTS = {
  // Test FeelsSOL mint address used in development
  // In production, this should be the actual FeelsSOL mint created during protocol initialization
  FEELSSOL_MINT: "11111111111111111111111111111112",
  
  // JitoSOL mint on mainnet (used as the underlying asset for FeelsSOL)
  JITOSOL_MINT_MAINNET: "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn",
  
  // For local testing, JitoSOL will be created dynamically
  // Check scripts/localnet-tokens.json after running setup:jitosol
  
  // Program ID for the Feels protocol
  FEELS_PROGRAM_ID: "Cbv2aa2zMJdwAwzLnRZuWQ8efpr6Xb9zxpJhEzLe3v6N",
  
  // Metaplex Token Metadata program
  METAPLEX_TOKEN_METADATA_PROGRAM_ID: "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s",
  
  // Token program IDs
  TOKEN_PROGRAM_ID: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
  TOKEN_2022_PROGRAM_ID: "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
  ASSOCIATED_TOKEN_PROGRAM_ID: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
  
  // Default market parameters
  DEFAULT_TICK_SPACING: 60,
  DEFAULT_BASE_FEE_BPS: 30, // 0.3%
  DEFAULT_TICK_STEP_SIZE: 100,
  
  // Decimals
  FEELSSOL_DECIMALS: 9,
  JITOSOL_DECIMALS: 9,
} as const;

// PDA seeds used in the protocol
export const PDA_SEEDS = {
  PROTOCOL_CONFIG: "protocol_config",
  PROTOCOL_ORACLE: "protocol_oracle",
  SAFETY_CONTROLLER: "safety_controller",
  FEELS_HUB: "feels_hub",
  MARKET: "market",
  BUFFER: "buffer",
  ORACLE: "oracle",
  VAULT: "vault",
  MARKET_AUTHORITY: "market_authority",
  ESCROW: "escrow",
  ESCROW_AUTHORITY: "escrow_authority",
  PROTOCOL_TOKEN: "protocol_token",
  TRANCHE_PLAN: "tranche_plan",
  TREASURY: "treasury",
  METADATA: "metadata",
} as const;