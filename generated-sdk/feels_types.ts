/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/feels.json`.
 */
export type Feels = {
  "address": "",
  "metadata": {
    "name": "feels",
    "version": "0.1.0",
    "spec": "0.1.0"
  },
  "instructions": [
    {
      "name": "initializeProtocol",
      "docs": [
        "Initialize protocol configuration (one-time setup)"
      ],
      "discriminator": [
        188,
        233,
        252,
        106,
        134,
        146,
        202,
        91
      ],
      "accounts": [
        {
          "name": "authority",
          "docs": [
            "Protocol authority (deployer)"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol config account"
          ],
          "writable": true
        },
        {
          "name": "systemProgram",
          "docs": [
            "System program"
          ]
        },
        {
          "name": "protocolOracle",
          "docs": [
            "Protocol oracle account (singleton)"
          ],
          "writable": true
        },
        {
          "name": "safety",
          "docs": [
            "Safety controller (singleton)"
          ],
          "writable": true
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "feels::instructions::initialize_protocol::InitializeProtocolParams"
            }
          }
        }
      ]
    },
    {
      "name": "updateFloor",
      "docs": [
        "Permissionless floor update crank (computes floor from reserves & supply)"
      ],
      "discriminator": [
        38,
        80,
        204,
        37,
        6,
        62,
        192,
        200
      ],
      "accounts": [
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "buffer",
          "docs": [
            "Buffer must be associated with this market"
          ]
        },
        {
          "name": "vault0",
          "docs": [
            "Vault 0 - must be the correct PDA for this market"
          ],
          "writable": true
        },
        {
          "name": "vault1",
          "docs": [
            "Vault 1 - must be the correct PDA for this market"
          ],
          "writable": true
        },
        {
          "name": "projectMint",
          "docs": [
            "Project mint must be the non-FeelsSOL token in this market"
          ]
        },
        {
          "name": "escrowTokenAccount",
          "docs": [
            "Optional: Pre-launch escrow token account (if tokens still in escrow)"
          ],
          "optional": true
        },
        {
          "name": "clock",
          "docs": [
            "Optional: Other protocol-owned token accounts to exclude",
            "These would be accounts holding tokens that should not be considered circulating",
            "Note: This is handled as remaining_accounts in the instruction handler"
          ]
        }
      ],
      "args": []
    },
    {
      "name": "updateProtocol",
      "docs": [
        "Update protocol configuration"
      ],
      "discriminator": [
        206,
        25,
        218,
        114,
        109,
        41,
        74,
        173
      ],
      "accounts": [
        {
          "name": "authority",
          "docs": [
            "Current protocol authority"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol config account"
          ],
          "writable": true
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "feels::instructions::initialize_protocol::UpdateProtocolParams"
            }
          }
        }
      ]
    },
    {
      "name": "setProtocolOwnedOverride",
      "docs": [
        "Set protocol owned override for floor calculation (governance only)"
      ],
      "discriminator": [
        250,
        164,
        109,
        69,
        170,
        65,
        157,
        140
      ],
      "accounts": [
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol config must exist"
          ]
        },
        {
          "name": "buffer",
          "docs": [
            "Buffer to update",
            "Note: We don't check buffer.authority here because it's set to the market creator,",
            "not the protocol authority. The protocol authority can still manage overrides",
            "as a governance function."
          ],
          "writable": true
        },
        {
          "name": "authority",
          "docs": [
            "Protocol authority - only they can set overrides"
          ],
          "signer": true
        }
      ],
      "args": [
        {
          "name": "overrideAmount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "initializePoolRegistry",
      "docs": [
        "Initialize the pool registry (one-time setup)"
      ],
      "discriminator": [
        109,
        119,
        17,
        241,
        165,
        19,
        176,
        175
      ],
      "accounts": [
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol config must exist"
          ]
        },
        {
          "name": "poolRegistry",
          "docs": [
            "Pool registry to initialize"
          ],
          "writable": true
        },
        {
          "name": "authority",
          "docs": [
            "Authority must match protocol authority"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "payer",
          "docs": [
            "Payer for account creation"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "systemProgram",
          "docs": [
            "System program"
          ]
        }
      ],
      "args": []
    },
    {
      "name": "registerPool",
      "docs": [
        "Register a pool in the registry"
      ],
      "discriminator": [
        85,
        229,
        114,
        47,
        75,
        145,
        166,
        100
      ],
      "accounts": [
        {
          "name": "poolRegistry",
          "docs": [
            "Pool registry"
          ],
          "writable": true
        },
        {
          "name": "market",
          "docs": [
            "Market to register"
          ]
        },
        {
          "name": "projectMint",
          "docs": [
            "Project token mint (non-FeelsSOL token)"
          ]
        },
        {
          "name": "creator",
          "docs": [
            "Creator registering the pool"
          ],
          "signer": true
        },
        {
          "name": "payer",
          "docs": [
            "Payer for realloc"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "systemProgram",
          "docs": [
            "System program"
          ]
        },
        {
          "name": "clock"
        }
      ],
      "args": []
    },
    {
      "name": "updatePoolPhase",
      "docs": [
        "Update pool phase in registry"
      ],
      "discriminator": [
        67,
        208,
        79,
        72,
        239,
        112,
        73,
        232
      ],
      "accounts": [
        {
          "name": "poolRegistry",
          "docs": [
            "Pool registry"
          ],
          "writable": true
        },
        {
          "name": "market",
          "docs": [
            "Market whose phase to update"
          ]
        },
        {
          "name": "authority",
          "docs": [
            "Authority (must be registry authority or market authority)"
          ],
          "signer": true
        },
        {
          "name": "clock"
        }
      ],
      "args": [
        {
          "name": "newPhase",
          "type": {
            "defined": {
              "name": "feels::state::pool_registry::PoolPhase"
            }
          }
        }
      ]
    },
    {
      "name": "initializePommPosition",
      "docs": [
        "Initialize a POMM (Protocol-Owned Market Making) position"
      ],
      "discriminator": [
        188,
        224,
        119,
        1,
        109,
        96,
        244,
        199
      ],
      "accounts": [
        {
          "name": "authority",
          "docs": [
            "Authority that can initialize POMM positions"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "docs": [
            "Market for this POMM position"
          ]
        },
        {
          "name": "buffer",
          "docs": [
            "Buffer that will own this POMM position"
          ]
        },
        {
          "name": "pommPosition",
          "docs": [
            "POMM position account to initialize",
            "Uses a PDA derived from market and position index"
          ],
          "writable": true
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol config to validate authority"
          ]
        },
        {
          "name": "systemProgram",
          "docs": [
            "System program"
          ]
        }
      ],
      "args": [
        {
          "name": "positionIndex",
          "type": "u8"
        }
      ]
    },
    {
      "name": "managePommPosition",
      "docs": [
        "Manage POMM (Protocol-Owned Market Making) positions"
      ],
      "discriminator": [
        173,
        67,
        116,
        206,
        107,
        121,
        81,
        19
      ],
      "accounts": [
        {
          "name": "authority",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "buffer",
          "writable": true
        },
        {
          "name": "pommPosition",
          "docs": [
            "POMM position account - must be initialized separately",
            "Uses a PDA derived from market and position index"
          ],
          "writable": true
        },
        {
          "name": "oracle",
          "writable": true
        },
        {
          "name": "vault0",
          "writable": true
        },
        {
          "name": "vault1",
          "writable": true
        },
        {
          "name": "bufferVault0",
          "writable": true
        },
        {
          "name": "bufferVault1",
          "writable": true
        },
        {
          "name": "bufferAuthority"
        },
        {
          "name": "protocolConfig"
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "systemProgram"
        },
        {
          "name": "rent"
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "feels::instructions::manage_pomm_position::ManagePommParams"
            }
          }
        }
      ]
    },
    {
      "name": "transitionMarketPhase",
      "docs": [
        "Transition market between phases"
      ],
      "discriminator": [
        192,
        45,
        250,
        40,
        31,
        139,
        115,
        62
      ],
      "accounts": [
        {
          "name": "authority",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "protocolConfig"
        },
        {
          "name": "oracle"
        },
        {
          "name": "buffer",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "feels::instructions::transition_market_phase::TransitionPhaseParams"
            }
          }
        }
      ]
    },
    {
      "name": "initializeMarket",
      "docs": [
        "Initialize a new market with commitment for initial liquidity",
        "Market creation and liquidity commitment are atomic, preventing",
        "front-running. Actual liquidity deployment happens separately via",
        "deploy_initial_liquidity instruction."
      ],
      "discriminator": [
        35,
        35,
        189,
        193,
        155,
        48,
        170,
        203
      ],
      "accounts": [
        {
          "name": "creator",
          "docs": [
            "Creator initializing the market"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "token0",
          "docs": [
            "Token 0 mint (lower pubkey)"
          ],
          "writable": true
        },
        {
          "name": "token1",
          "docs": [
            "Token 1 mint (higher pubkey)"
          ],
          "writable": true
        },
        {
          "name": "market",
          "docs": [
            "Market account to initialize"
          ],
          "writable": true
        },
        {
          "name": "buffer",
          "docs": [
            "Buffer account to initialize"
          ],
          "writable": true
        },
        {
          "name": "oracle",
          "docs": [
            "Oracle account to initialize"
          ],
          "writable": true
        },
        {
          "name": "vault0",
          "docs": [
            "Vault 0 for token 0"
          ],
          "writable": true
        },
        {
          "name": "vault1",
          "docs": [
            "Vault 1 for token 1"
          ],
          "writable": true
        },
        {
          "name": "marketAuthority",
          "docs": [
            "Market authority PDA"
          ]
        },
        {
          "name": "feelssolMint",
          "docs": [
            "FeelsSOL mint (hub token)"
          ]
        },
        {
          "name": "protocolToken0",
          "docs": [
            "Protocol token registry for token_0 (if not FeelsSOL)"
          ]
        },
        {
          "name": "protocolToken1",
          "docs": [
            "Protocol token registry for token_1 (if not FeelsSOL)"
          ]
        },
        {
          "name": "escrow",
          "docs": [
            "Pre-launch escrow for the protocol token"
          ],
          "writable": true
        },
        {
          "name": "creatorFeelssol",
          "docs": [
            "Creator's FeelsSOL account for initial buy"
          ]
        },
        {
          "name": "creatorTokenOut",
          "docs": [
            "Creator's token account for receiving initial buy tokens"
          ]
        },
        {
          "name": "escrowAuthority",
          "docs": [
            "Escrow authority PDA (holds mint/freeze authorities)"
          ],
          "writable": true
        },
        {
          "name": "systemProgram",
          "docs": [
            "System program"
          ]
        },
        {
          "name": "tokenProgram",
          "docs": [
            "Token program"
          ]
        },
        {
          "name": "rent",
          "docs": [
            "Rent sysvar"
          ]
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "feels::instructions::initialize_market::InitializeMarketParams"
            }
          }
        }
      ]
    },
    {
      "name": "enterFeelssol",
      "docs": [
        "Enter FeelsSOL - deposit JitoSOL to mint FeelsSOL"
      ],
      "discriminator": [
        199,
        205,
        49,
        173,
        81,
        50,
        186,
        126
      ],
      "accounts": [
        {
          "name": "user",
          "docs": [
            "User entering FeelsSOL",
            "SECURITY: Must be a system account to prevent PDA identity confusion"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "userJitosol",
          "docs": [
            "User's JitoSOL account"
          ],
          "writable": true
        },
        {
          "name": "userFeelssol",
          "docs": [
            "User's FeelsSOL account"
          ],
          "writable": true
        },
        {
          "name": "jitosolMint",
          "docs": [
            "JitoSOL mint"
          ]
        },
        {
          "name": "feelssolMint",
          "docs": [
            "FeelsSOL mint"
          ],
          "writable": true
        },
        {
          "name": "hub",
          "docs": [
            "FeelsHub PDA for reentrancy guard"
          ],
          "writable": true
        },
        {
          "name": "jitosolVault",
          "docs": [
            "JitoSOL vault (pool-owned by the FeelsSOL hub pool)"
          ],
          "writable": true
        },
        {
          "name": "mintAuthority",
          "docs": [
            "Mint authority PDA"
          ]
        },
        {
          "name": "tokenProgram",
          "docs": [
            "Token program"
          ]
        },
        {
          "name": "systemProgram",
          "docs": [
            "System program"
          ]
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "exitFeelssol",
      "docs": [
        "Exit FeelsSOL - burn FeelsSOL to redeem JitoSOL"
      ],
      "discriminator": [
        105,
        118,
        168,
        148,
        61,
        152,
        3,
        175
      ],
      "accounts": [
        {
          "name": "user",
          "docs": [
            "User exiting FeelsSOL",
            "SECURITY: Must be a system account to prevent PDA identity confusion"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "userJitosol",
          "docs": [
            "User's JitoSOL account"
          ],
          "writable": true
        },
        {
          "name": "userFeelssol",
          "docs": [
            "User's FeelsSOL account"
          ],
          "writable": true
        },
        {
          "name": "jitosolMint",
          "docs": [
            "JitoSOL mint"
          ]
        },
        {
          "name": "feelssolMint",
          "docs": [
            "FeelsSOL mint"
          ],
          "writable": true
        },
        {
          "name": "hub",
          "docs": [
            "FeelsHub PDA for FeelsSOL mint",
            "SECURITY: Provides re-entrancy guard protection"
          ],
          "writable": true
        },
        {
          "name": "safety",
          "docs": [
            "Safety controller (protocol-level)"
          ],
          "writable": true
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol config (for rate limits)"
          ]
        },
        {
          "name": "protocolOracle",
          "docs": [
            "Protocol oracle (rates)"
          ],
          "writable": true
        },
        {
          "name": "jitosolVault",
          "docs": [
            "JitoSOL vault (pool-owned by the FeelsSOL hub pool)"
          ],
          "writable": true
        },
        {
          "name": "vaultAuthority",
          "docs": [
            "Vault authority PDA"
          ]
        },
        {
          "name": "tokenProgram",
          "docs": [
            "Token program"
          ]
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "initializeHub",
      "docs": [
        "Initialize FeelsHub for enter/exit operations"
      ],
      "discriminator": [
        202,
        27,
        126,
        27,
        54,
        182,
        68,
        169
      ],
      "accounts": [
        {
          "name": "payer",
          "docs": [
            "Authority paying for the account"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "feelssolMint",
          "docs": [
            "FeelsSOL mint the hub manages"
          ]
        },
        {
          "name": "jitosolMint",
          "docs": [
            "JitoSOL mint"
          ]
        },
        {
          "name": "hub",
          "docs": [
            "The FeelsHub PDA"
          ],
          "writable": true
        },
        {
          "name": "jitosolVault",
          "docs": [
            "JitoSOL vault for the hub"
          ],
          "writable": true
        },
        {
          "name": "vaultAuthority",
          "docs": [
            "Vault authority PDA"
          ]
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "systemProgram"
        }
      ],
      "args": []
    },
    {
      "name": "swap",
      "docs": [
        "Swap tokens through the AMM"
      ],
      "discriminator": [
        248,
        198,
        158,
        145,
        225,
        117,
        135,
        200
      ],
      "accounts": [
        {
          "name": "user",
          "docs": [
            "The user initiating the swap transaction",
            "Must be a system account (not a PDA) to prevent identity confusion attacks"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "docs": [
            "The market account containing trading pair state",
            "Must be initialized, not paused, and not currently in a reentrant call"
          ],
          "writable": true
        },
        {
          "name": "vault0",
          "docs": [
            "Protocol-owned vault holding token_0 reserves",
            "PDA derived from market tokens with deterministic ordering"
          ],
          "writable": true
        },
        {
          "name": "vault1",
          "docs": [
            "Protocol-owned vault holding token_1 reserves",
            "PDA derived from market tokens with deterministic ordering"
          ],
          "writable": true
        },
        {
          "name": "marketAuthority",
          "docs": [
            "Market authority PDA that controls vault operations",
            "Used as signer for transferring tokens from vaults to users"
          ]
        },
        {
          "name": "buffer",
          "docs": [
            "Buffer account for fee collection and protocol-owned market making",
            "Accumulates impact fees for later deployment as liquidity"
          ],
          "writable": true
        },
        {
          "name": "oracle",
          "docs": [
            "Oracle account for tracking time-weighted average prices (TWAP)",
            "Updated on every swap to maintain accurate price history"
          ],
          "writable": true
        },
        {
          "name": "userTokenIn",
          "docs": [
            "User's token account for the input token being swapped",
            "Ownership and mint validation performed in handler"
          ],
          "writable": true
        },
        {
          "name": "userTokenOut",
          "docs": [
            "User's token account for the output token being received",
            "Ownership and mint validation performed in handler"
          ],
          "writable": true
        },
        {
          "name": "tokenProgram",
          "docs": [
            "SPL Token program for executing transfers"
          ]
        },
        {
          "name": "clock",
          "docs": [
            "Clock sysvar for timestamp and epoch tracking"
          ]
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol configuration account for fee rates"
          ]
        },
        {
          "name": "protocolTreasury",
          "docs": [
            "Protocol treasury token account (mandatory for protocol fees)"
          ],
          "writable": true
        },
        {
          "name": "protocolToken",
          "docs": [
            "Protocol token registry entry (optional - only for protocol-minted tokens)"
          ],
          "optional": true
        },
        {
          "name": "creatorTokenAccount",
          "docs": [
            "Creator token account (optional - only if creator fees > 0 and protocol token present)"
          ],
          "writable": true,
          "optional": true
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "feels::instructions::swap::SwapParams"
            }
          }
        }
      ]
    },
    {
      "name": "swapExactOut",
      "docs": [
        "Swap tokens with exact output amount"
      ],
      "discriminator": [
        250,
        73,
        101,
        33,
        38,
        207,
        75,
        184
      ],
      "accounts": [
        {
          "name": "user",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "buffer",
          "writable": true
        },
        {
          "name": "oracle",
          "writable": true
        },
        {
          "name": "vault0",
          "writable": true
        },
        {
          "name": "vault1",
          "writable": true
        },
        {
          "name": "userAccount0",
          "writable": true
        },
        {
          "name": "userAccount1",
          "writable": true
        },
        {
          "name": "marketAuthority"
        },
        {
          "name": "protocolConfig"
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "clock"
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "feels::instructions::swap_exact_out::SwapExactOutParams"
            }
          }
        }
      ]
    },
    {
      "name": "openPosition",
      "docs": [
        "Open a new liquidity position"
      ],
      "discriminator": [
        135,
        128,
        47,
        77,
        15,
        152,
        240,
        49
      ],
      "accounts": [
        {
          "name": "provider",
          "docs": [
            "Liquidity provider",
            "SECURITY: Must be a system account to prevent PDA identity confusion"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "docs": [
            "Market state"
          ],
          "writable": true
        },
        {
          "name": "positionMint",
          "docs": [
            "Position mint - a simple SPL token representing ownership"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "positionTokenAccount",
          "docs": [
            "Position token account - where the position token is minted"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "position",
          "docs": [
            "Position account (PDA) - stores all position state"
          ],
          "writable": true
        },
        {
          "name": "providerToken0",
          "docs": [
            "Provider's token account for token 0"
          ],
          "writable": true
        },
        {
          "name": "providerToken1",
          "docs": [
            "Provider's token account for token 1"
          ],
          "writable": true
        },
        {
          "name": "vault0",
          "docs": [
            "Market vault for token 0"
          ],
          "writable": true
        },
        {
          "name": "vault1",
          "docs": [
            "Market vault for token 1"
          ],
          "writable": true
        },
        {
          "name": "lowerTickArray",
          "docs": [
            "Tick array containing the lower tick"
          ],
          "writable": true
        },
        {
          "name": "upperTickArray",
          "docs": [
            "Tick array containing the upper tick"
          ],
          "writable": true
        },
        {
          "name": "tokenProgram",
          "docs": [
            "Token program"
          ]
        },
        {
          "name": "systemProgram",
          "docs": [
            "System program"
          ]
        }
      ],
      "args": [
        {
          "name": "tickLower",
          "type": "i32"
        },
        {
          "name": "tickUpper",
          "type": "i32"
        },
        {
          "name": "liquidityAmount",
          "type": "u128"
        }
      ]
    },
    {
      "name": "closePosition",
      "docs": [
        "Close a liquidity position"
      ],
      "discriminator": [
        123,
        134,
        81,
        0,
        49,
        68,
        98,
        98
      ],
      "accounts": [
        {
          "name": "owner",
          "docs": [
            "Position owner",
            "SECURITY: Must be a system account to prevent PDA identity confusion"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "docs": [
            "Market state"
          ],
          "writable": true
        },
        {
          "name": "positionMint",
          "docs": [
            "Position mint"
          ],
          "writable": true
        },
        {
          "name": "positionTokenAccount",
          "docs": [
            "Position token account (must hold exactly 1 token)"
          ],
          "writable": true
        },
        {
          "name": "position",
          "docs": [
            "Position account (PDA)",
            "SECURITY: Account closure handled in instruction logic to prevent fee theft"
          ],
          "writable": true
        },
        {
          "name": "ownerToken0",
          "docs": [
            "Owner's token account for token 0"
          ],
          "writable": true
        },
        {
          "name": "ownerToken1",
          "docs": [
            "Owner's token account for token 1"
          ],
          "writable": true
        },
        {
          "name": "vault0",
          "docs": [
            "Market vault for token 0 - derived from market and token_0"
          ],
          "writable": true
        },
        {
          "name": "vault1",
          "docs": [
            "Market vault for token 1 - derived from market and token_1"
          ],
          "writable": true
        },
        {
          "name": "marketAuthority",
          "docs": [
            "Unified market authority PDA"
          ]
        },
        {
          "name": "lowerTickArray",
          "docs": [
            "Tick array containing the lower tick"
          ],
          "writable": true
        },
        {
          "name": "upperTickArray",
          "docs": [
            "Tick array containing the upper tick"
          ],
          "writable": true
        },
        {
          "name": "tokenProgram",
          "docs": [
            "Token program"
          ]
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "feels::instructions::close_position::ClosePositionParams"
            }
          }
        }
      ]
    },
    {
      "name": "collectFees",
      "docs": [
        "Collect fees from a position - smart single entry point",
        "Automatically handles normal positions, wide positions, and accumulated fees"
      ],
      "discriminator": [
        164,
        152,
        207,
        99,
        30,
        186,
        19,
        182
      ],
      "accounts": [
        {
          "name": "owner",
          "docs": [
            "Position owner",
            "SECURITY: Must be a system account to prevent PDA identity confusion"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "docs": [
            "Market"
          ],
          "writable": true
        },
        {
          "name": "positionMint",
          "docs": [
            "Position mint"
          ]
        },
        {
          "name": "positionTokenAccount",
          "docs": [
            "Position token account (must hold the position token)"
          ]
        },
        {
          "name": "position",
          "docs": [
            "Position"
          ],
          "writable": true
        },
        {
          "name": "ownerToken0",
          "docs": [
            "Owner token accounts"
          ],
          "writable": true
        },
        {
          "name": "ownerToken1",
          "writable": true
        },
        {
          "name": "vault0",
          "docs": [
            "Market vault for token 0 - derived from market and token_0"
          ],
          "writable": true
        },
        {
          "name": "vault1",
          "docs": [
            "Market vault for token 1 - derived from market and token_1"
          ],
          "writable": true
        },
        {
          "name": "marketAuthority",
          "docs": [
            "Unified market authority"
          ]
        },
        {
          "name": "tokenProgram"
        }
      ],
      "args": []
    },
    {
      "name": "updatePositionFeeLower",
      "docs": [
        "Update position fee accrual for lower tick",
        "Part 1/3 of fee collection for wide positions"
      ],
      "discriminator": [
        58,
        181,
        152,
        160,
        205,
        130,
        59,
        20
      ],
      "accounts": [
        {
          "name": "owner",
          "docs": [
            "Position owner"
          ],
          "signer": true
        },
        {
          "name": "market",
          "docs": [
            "Market"
          ]
        },
        {
          "name": "position",
          "docs": [
            "Position"
          ],
          "writable": true
        },
        {
          "name": "lowerTickArray",
          "docs": [
            "Tick array containing the lower tick"
          ]
        }
      ],
      "args": []
    },
    {
      "name": "updatePositionFeeUpper",
      "docs": [
        "Update position fee accrual for upper tick",
        "Part 2/3 of fee collection for wide positions"
      ],
      "discriminator": [
        162,
        48,
        161,
        22,
        95,
        7,
        191,
        252
      ],
      "accounts": [
        {
          "name": "owner",
          "docs": [
            "Position owner"
          ],
          "signer": true
        },
        {
          "name": "market",
          "docs": [
            "Market"
          ]
        },
        {
          "name": "position",
          "docs": [
            "Position"
          ],
          "writable": true
        },
        {
          "name": "upperTickArray",
          "docs": [
            "Tick array containing the upper tick"
          ]
        }
      ],
      "args": []
    },
    {
      "name": "mintToken",
      "docs": [
        "Mint a new token with distribution"
      ],
      "discriminator": [
        172,
        137,
        183,
        14,
        207,
        110,
        234,
        56
      ],
      "accounts": [
        {
          "name": "creator",
          "docs": [
            "Token creator",
            "SECURITY: Must be a system account to prevent PDA identity confusion"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "tokenMint",
          "docs": [
            "New token mint to create"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "escrow",
          "docs": [
            "Pre-launch escrow account for this token"
          ],
          "writable": true
        },
        {
          "name": "escrowTokenVault",
          "docs": [
            "Escrow's token vault (holds all minted tokens)"
          ],
          "writable": true
        },
        {
          "name": "escrowFeelssolVault",
          "docs": [
            "Escrow's FeelsSOL vault (holds mint fee)"
          ],
          "writable": true
        },
        {
          "name": "escrowAuthority",
          "docs": [
            "Escrow authority PDA"
          ]
        },
        {
          "name": "metadata",
          "docs": [
            "Metadata account"
          ],
          "writable": true
        },
        {
          "name": "feelssolMint",
          "docs": [
            "FeelsSOL mint"
          ]
        },
        {
          "name": "creatorFeelssol",
          "docs": [
            "Creator's FeelsSOL account for paying mint fee"
          ],
          "writable": true
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol config account"
          ]
        },
        {
          "name": "metadataProgram",
          "docs": [
            "Metaplex token metadata program"
          ]
        },
        {
          "name": "protocolToken",
          "docs": [
            "Protocol token registry entry"
          ],
          "writable": true
        },
        {
          "name": "associatedTokenProgram",
          "docs": [
            "Associated token program"
          ]
        },
        {
          "name": "rent",
          "docs": [
            "Rent sysvar"
          ]
        },
        {
          "name": "tokenProgram",
          "docs": [
            "Token program"
          ]
        },
        {
          "name": "systemProgram",
          "docs": [
            "System program"
          ]
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "feels::instructions::mint_token::MintTokenParams"
            }
          }
        }
      ]
    },
    {
      "name": "deployInitialLiquidity",
      "docs": [
        "Deploy initial liquidity to a market",
        "Verifies the deployment matches the commitment made during market",
        "initialization, preventing unauthorized liquidity deployment"
      ],
      "discriminator": [
        226,
        227,
        73,
        75,
        85,
        216,
        151,
        217
      ],
      "accounts": [
        {
          "name": "deployer",
          "docs": [
            "Deployer (must be market authority)"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "docs": [
            "Market account"
          ],
          "writable": true
        },
        {
          "name": "token0Mint",
          "docs": [
            "Token mints to read decimals (production-grade)"
          ]
        },
        {
          "name": "token1Mint"
        },
        {
          "name": "deployerFeelssol",
          "docs": [
            "Deployer's FeelsSOL account (for initial buy)"
          ],
          "writable": true
        },
        {
          "name": "deployerTokenOut",
          "docs": [
            "Deployer's token account for receiving initial buy tokens"
          ],
          "writable": true
        },
        {
          "name": "vault0",
          "docs": [
            "Vault 0"
          ],
          "writable": true
        },
        {
          "name": "vault1",
          "docs": [
            "Vault 1"
          ],
          "writable": true
        },
        {
          "name": "marketAuthority",
          "docs": [
            "Market authority PDA"
          ]
        },
        {
          "name": "buffer",
          "docs": [
            "Market buffer account (for fee collection, not token escrow)",
            "Buffer is included to update deployment tracking"
          ],
          "writable": true
        },
        {
          "name": "oracle",
          "docs": [
            "Oracle account for price updates"
          ],
          "writable": true
        },
        {
          "name": "escrow",
          "docs": [
            "Pre-launch escrow for the protocol token",
            "Escrow is derived from the non-FeelsSOL token mint"
          ],
          "writable": true
        },
        {
          "name": "escrowTokenVault",
          "docs": [
            "Escrow's token vault"
          ],
          "writable": true
        },
        {
          "name": "escrowFeelssolVault",
          "docs": [
            "Escrow's FeelsSOL vault"
          ],
          "writable": true
        },
        {
          "name": "escrowAuthority",
          "docs": [
            "Escrow authority PDA"
          ]
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol config account"
          ]
        },
        {
          "name": "treasury",
          "docs": [
            "Treasury to receive mint fee"
          ],
          "writable": true
        },
        {
          "name": "tokenProgram",
          "docs": [
            "Token program"
          ]
        },
        {
          "name": "systemProgram",
          "docs": [
            "System program"
          ]
        },
        {
          "name": "tranchePlan",
          "docs": [
            "Tranche plan PDA (initialized here)"
          ],
          "writable": true
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "feels::instructions::deploy_initial_liquidity::DeployInitialLiquidityParams"
            }
          }
        }
      ]
    },
    {
      "name": "initializeTrancheTicks",
      "docs": [
        "Permissionless crank to initialize tranche TickArrays and boundary ticks"
      ],
      "discriminator": [
        118,
        74,
        31,
        238,
        66,
        167,
        66,
        93
      ],
      "accounts": [
        {
          "name": "crank",
          "docs": [
            "Anyone can crank"
          ],
          "signer": true
        },
        {
          "name": "market",
          "docs": [
            "Market whose tranche ticks to initialize"
          ],
          "writable": true
        },
        {
          "name": "tranchePlan",
          "docs": [
            "Tranche plan produced at deploy time"
          ],
          "writable": true
        },
        {
          "name": "systemProgram",
          "docs": [
            "System program for creating missing TickArrays"
          ]
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "feels::instructions::initialize_tranche_ticks::InitializeTrancheTicksParams"
            }
          }
        }
      ]
    },
    {
      "name": "cleanupBondingCurve",
      "docs": [
        "Cleanup bonding curve plan and mark cleanup complete"
      ],
      "discriminator": [
        205,
        225,
        206,
        146,
        97,
        186,
        14,
        238
      ],
      "accounts": [
        {
          "name": "authority",
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "writable": true
        },
        {
          "name": "tranchePlan",
          "writable": true
        }
      ],
      "args": []
    },
    {
      "name": "openPositionWithMetadata",
      "docs": [
        "Open a position with NFT metadata"
      ],
      "discriminator": [
        242,
        29,
        134,
        48,
        58,
        110,
        14,
        60
      ],
      "accounts": [
        {
          "name": "provider",
          "docs": [
            "Liquidity provider",
            "SECURITY: Must be a system account to prevent PDA identity confusion"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "docs": [
            "Market state"
          ],
          "writable": true
        },
        {
          "name": "positionMint",
          "docs": [
            "Position mint - will become an NFT with metadata"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "positionTokenAccount",
          "docs": [
            "Position token account"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "position",
          "docs": [
            "Position account (PDA)"
          ],
          "writable": true
        },
        {
          "name": "metadata",
          "docs": [
            "Metadata account (PDA of Metaplex Token Metadata program)"
          ],
          "writable": true
        },
        {
          "name": "providerToken0",
          "docs": [
            "Provider's token account for token 0"
          ],
          "writable": true
        },
        {
          "name": "providerToken1",
          "docs": [
            "Provider's token account for token 1"
          ],
          "writable": true
        },
        {
          "name": "vault0",
          "docs": [
            "Market vault for token 0 - derived from market and token_0"
          ],
          "writable": true
        },
        {
          "name": "vault1",
          "docs": [
            "Market vault for token 1 - derived from market and token_1"
          ],
          "writable": true
        },
        {
          "name": "lowerTickArray",
          "docs": [
            "Tick array containing the lower tick"
          ],
          "writable": true
        },
        {
          "name": "upperTickArray",
          "docs": [
            "Tick array containing the upper tick"
          ],
          "writable": true
        },
        {
          "name": "metadataProgram",
          "docs": [
            "Metaplex Token Metadata program"
          ]
        },
        {
          "name": "tokenProgram",
          "docs": [
            "Token program"
          ]
        },
        {
          "name": "systemProgram",
          "docs": [
            "System program"
          ]
        },
        {
          "name": "rent",
          "docs": [
            "Rent sysvar"
          ]
        }
      ],
      "args": [
        {
          "name": "tickLower",
          "type": "i32"
        },
        {
          "name": "tickUpper",
          "type": "i32"
        },
        {
          "name": "liquidityAmount",
          "type": "u128"
        }
      ]
    },
    {
      "name": "closePositionWithMetadata",
      "docs": [
        "Close a position with NFT metadata"
      ],
      "discriminator": [
        17,
        174,
        244,
        40,
        141,
        4,
        42,
        125
      ],
      "accounts": [
        {
          "name": "owner",
          "docs": [
            "Position owner",
            "SECURITY: Must be a system account to prevent PDA identity confusion"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "docs": [
            "Market state"
          ],
          "writable": true
        },
        {
          "name": "positionMint",
          "docs": [
            "Position mint"
          ],
          "writable": true
        },
        {
          "name": "positionTokenAccount",
          "docs": [
            "Position token account"
          ],
          "writable": true
        },
        {
          "name": "position",
          "docs": [
            "Position account (PDA)",
            "SECURITY: Removed `close = owner` to prevent fee theft vulnerability.",
            "Position must be closed in a separate instruction after verification."
          ],
          "writable": true
        },
        {
          "name": "metadata",
          "docs": [
            "Metadata account (will be closed)"
          ],
          "writable": true
        },
        {
          "name": "ownerToken0",
          "docs": [
            "Owner's token account for token 0"
          ],
          "writable": true
        },
        {
          "name": "ownerToken1",
          "docs": [
            "Owner's token account for token 1"
          ],
          "writable": true
        },
        {
          "name": "vault0",
          "docs": [
            "Market vault for token 0 - derived from market and token_0"
          ],
          "writable": true
        },
        {
          "name": "vault1",
          "docs": [
            "Market vault for token 1 - derived from market and token_1"
          ],
          "writable": true
        },
        {
          "name": "marketAuthority",
          "docs": [
            "Unified market authority PDA"
          ]
        },
        {
          "name": "lowerTickArray",
          "docs": [
            "Tick array containing the lower tick"
          ],
          "writable": true
        },
        {
          "name": "upperTickArray",
          "docs": [
            "Tick array containing the upper tick"
          ],
          "writable": true
        },
        {
          "name": "metadataProgram",
          "docs": [
            "Metaplex Token Metadata program"
          ]
        },
        {
          "name": "tokenProgram",
          "docs": [
            "Token program"
          ]
        }
      ],
      "args": [
        {
          "name": "amount0Min",
          "type": "u64"
        },
        {
          "name": "amount1Min",
          "type": "u64"
        }
      ]
    },
    {
      "name": "destroyExpiredToken",
      "docs": [
        "Destroy an expired token that hasn't had liquidity deployed"
      ],
      "discriminator": [
        72,
        107,
        101,
        121,
        217,
        54,
        144,
        155
      ],
      "accounts": [
        {
          "name": "destroyer",
          "docs": [
            "Anyone can call this instruction to destroy expired tokens"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "tokenMint",
          "docs": [
            "Token mint to destroy"
          ]
        },
        {
          "name": "protocolToken",
          "docs": [
            "Protocol token registry entry"
          ],
          "writable": true
        },
        {
          "name": "escrow",
          "docs": [
            "Pre-launch escrow account for this token"
          ],
          "writable": true
        },
        {
          "name": "escrowTokenVault",
          "docs": [
            "Escrow's token vault"
          ],
          "writable": true
        },
        {
          "name": "escrowFeelssolVault",
          "docs": [
            "Escrow's FeelsSOL vault (contains mint fee)"
          ],
          "writable": true
        },
        {
          "name": "escrowAuthority",
          "docs": [
            "Escrow authority PDA"
          ]
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol config"
          ]
        },
        {
          "name": "treasury",
          "docs": [
            "Treasury to receive 50% of mint fee"
          ],
          "writable": true
        },
        {
          "name": "destroyerFeelssol",
          "docs": [
            "Destroyer's FeelsSOL account to receive 50% of mint fee"
          ],
          "writable": true
        },
        {
          "name": "market",
          "docs": [
            "Optional: Market account if it was created"
          ],
          "writable": true,
          "optional": true
        },
        {
          "name": "associatedTokenProgram",
          "docs": [
            "Associated token program"
          ]
        },
        {
          "name": "tokenProgram",
          "docs": [
            "Token program"
          ]
        },
        {
          "name": "systemProgram",
          "docs": [
            "System program"
          ]
        }
      ],
      "args": []
    },
    {
      "name": "graduatePool",
      "docs": [
        "Graduate pool to steady state (idempotent)"
      ],
      "discriminator": [
        210,
        29,
        144,
        133,
        25,
        219,
        183,
        247
      ],
      "accounts": [
        {
          "name": "authority",
          "docs": [
            "Market authority performing graduation"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "market",
          "docs": [
            "Market to graduate"
          ],
          "writable": true
        }
      ],
      "args": []
    },
    {
      "name": "updateDexTwap",
      "docs": [
        "Update DEX TWAP for protocol oracle (keeper-updated)"
      ],
      "discriminator": [
        144,
        64,
        180,
        12,
        223,
        33,
        140,
        232
      ],
      "accounts": [
        {
          "name": "updater",
          "docs": [
            "Updater authorized in ProtocolConfig"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol config (for params and updater key)"
          ]
        },
        {
          "name": "protocolOracle",
          "docs": [
            "Protocol oracle (singleton)"
          ],
          "writable": true
        },
        {
          "name": "safety",
          "docs": [
            "Safety controller (singleton)"
          ],
          "writable": true
        },
        {
          "name": "clock",
          "docs": [
            "Clock sysvar"
          ]
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "feels::instructions::update_protocol_oracle::UpdateDexTwapParams"
            }
          }
        }
      ]
    },
    {
      "name": "updateNativeRate",
      "docs": [
        "Update native reserve rate for protocol oracle (authority)"
      ],
      "discriminator": [
        100,
        175,
        161,
        10,
        254,
        80,
        99,
        77
      ],
      "accounts": [
        {
          "name": "authority",
          "docs": [
            "Protocol authority"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "protocolConfig",
          "docs": [
            "Protocol config"
          ]
        },
        {
          "name": "protocolOracle",
          "docs": [
            "Protocol oracle"
          ],
          "writable": true
        },
        {
          "name": "safety",
          "docs": [
            "Safety controller"
          ],
          "writable": true
        },
        {
          "name": "clock"
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "feels::instructions::update_protocol_oracle::UpdateNativeRateParams"
            }
          }
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "feels::state::buffer::Buffer",
      "discriminator": [
        115,
        5,
        212,
        192,
        85,
        30,
        46,
        41
      ]
    },
    {
      "name": "feels::state::escrow::PreLaunchEscrow",
      "discriminator": [
        128,
        182,
        252,
        215,
        180,
        44,
        106,
        53
      ]
    },
    {
      "name": "feels::state::feels_hub::FeelsHub",
      "discriminator": [
        151,
        222,
        200,
        223,
        213,
        72,
        219,
        90
      ]
    },
    {
      "name": "feels::state::market::Market",
      "discriminator": [
        219,
        190,
        213,
        55,
        0,
        227,
        198,
        154
      ]
    },
    {
      "name": "feels::state::oracle::OracleState",
      "discriminator": [
        97,
        156,
        157,
        189,
        194,
        73,
        8,
        15
      ]
    },
    {
      "name": "feels::state::pool_registry::PoolRegistry",
      "discriminator": [
        113,
        149,
        124,
        60,
        130,
        240,
        64,
        157
      ]
    },
    {
      "name": "feels::state::position::Position",
      "discriminator": [
        170,
        188,
        143,
        228,
        122,
        64,
        247,
        208
      ]
    },
    {
      "name": "feels::state::protocol_config::ProtocolConfig",
      "discriminator": [
        207,
        91,
        250,
        28,
        152,
        179,
        215,
        209
      ]
    },
    {
      "name": "feels::state::protocol_oracle::ProtocolOracle",
      "discriminator": [
        252,
        90,
        103,
        97,
        37,
        251,
        8,
        237
      ]
    },
    {
      "name": "feels::state::safety_controller::SafetyController",
      "discriminator": [
        53,
        32,
        49,
        82,
        152,
        157,
        140,
        241
      ]
    },
    {
      "name": "feels::state::tick::TickArray",
      "discriminator": [
        69,
        97,
        189,
        190,
        110,
        7,
        66,
        187
      ]
    },
    {
      "name": "feels::state::token_metadata::ProtocolToken",
      "discriminator": [
        14,
        19,
        206,
        1,
        203,
        204,
        55,
        222
      ]
    },
    {
      "name": "feels::state::tranche_plan::TranchePlan",
      "discriminator": [
        104,
        24,
        192,
        150,
        169,
        184,
        251,
        102
      ]
    }
  ],
  "types": [
    {
      "name": "feels::instructions::close_position::ClosePositionParams",
      "docs": [
        "Close position parameters"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "amount0Min",
            "docs": [
              "Minimum amount of token 0 to receive"
            ],
            "type": "u64"
          },
          {
            "name": "amount1Min",
            "docs": [
              "Minimum amount of token 1 to receive"
            ],
            "type": "u64"
          },
          {
            "name": "closeAccount",
            "docs": [
              "If true, close the position account after withdrawing liquidity",
              "If false, keep the account open (useful if you want to collect fees later)"
            ],
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "feels::instructions::deploy_initial_liquidity::DeployInitialLiquidityParams",
      "docs": [
        "Deploy initial liquidity parameters"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tickStepSize",
            "docs": [
              "Number of ticks between each stair step"
            ],
            "type": "i32"
          },
          {
            "name": "initialBuyFeelssolAmount",
            "docs": [
              "Optional initial buy amount in FeelsSOL (0 = no initial buy)"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "feels::instructions::initialize_market::InitializeMarketParams",
      "docs": [
        "Initialize market parameters"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "baseFeeBps",
            "docs": [
              "Base fee in basis points (e.g., 30 = 0.3%)"
            ],
            "type": "u16"
          },
          {
            "name": "tickSpacing",
            "docs": [
              "Tick spacing for the market"
            ],
            "type": "u16"
          },
          {
            "name": "initialSqrtPrice",
            "docs": [
              "Initial price (as sqrt_price Q64)"
            ],
            "type": "u128"
          },
          {
            "name": "initialBuyFeelssolAmount",
            "docs": [
              "Optional initial buy amount in FeelsSOL (0 = no initial buy)"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "feels::instructions::initialize_protocol::InitializeProtocolParams",
      "docs": [
        "Initialize protocol parameters"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mintFee",
            "docs": [
              "Initial mint fee in FeelsSOL lamports"
            ],
            "type": "u64"
          },
          {
            "name": "treasury",
            "docs": [
              "Treasury account to receive fees"
            ],
            "type": "pubkey"
          },
          {
            "name": "defaultProtocolFeeRate",
            "docs": [
              "Default protocol fee rate (basis points, e.g. 1000 = 10%)"
            ],
            "type": {
              "option": "u16"
            }
          },
          {
            "name": "defaultCreatorFeeRate",
            "docs": [
              "Default creator fee rate for protocol tokens (basis points, e.g. 500 = 5%)"
            ],
            "type": {
              "option": "u16"
            }
          },
          {
            "name": "maxProtocolFeeRate",
            "docs": [
              "Maximum allowed protocol fee rate (basis points)"
            ],
            "type": {
              "option": "u16"
            }
          },
          {
            "name": "dexTwapUpdater",
            "docs": [
              "DEX TWAP updater authority"
            ],
            "type": "pubkey"
          },
          {
            "name": "depegThresholdBps",
            "docs": [
              "De-peg threshold (bps)"
            ],
            "type": "u16"
          },
          {
            "name": "depegRequiredObs",
            "docs": [
              "Consecutive breaches to pause"
            ],
            "type": "u8"
          },
          {
            "name": "clearRequiredObs",
            "docs": [
              "Consecutive clears to resume"
            ],
            "type": "u8"
          },
          {
            "name": "dexTwapWindowSecs",
            "docs": [
              "DEX TWAP window seconds"
            ],
            "type": "u32"
          },
          {
            "name": "dexTwapStaleAgeSecs",
            "docs": [
              "DEX TWAP stale age seconds"
            ],
            "type": "u32"
          },
          {
            "name": "dexWhitelist",
            "docs": [
              "Initial DEX whitelist (optional; empty ok)"
            ],
            "type": {
              "vec": "pubkey"
            }
          }
        ]
      }
    },
    {
      "name": "feels::instructions::initialize_protocol::UpdateProtocolParams",
      "docs": [
        "Update protocol configuration parameters"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mintFee",
            "docs": [
              "New mint fee (None to keep current)"
            ],
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "treasury",
            "docs": [
              "New treasury (None to keep current)"
            ],
            "type": {
              "option": "pubkey"
            }
          },
          {
            "name": "authority",
            "docs": [
              "New authority (None to keep current)"
            ],
            "type": {
              "option": "pubkey"
            }
          },
          {
            "name": "defaultProtocolFeeRate",
            "docs": [
              "New default protocol fee rate (None to keep current)"
            ],
            "type": {
              "option": "u16"
            }
          },
          {
            "name": "defaultCreatorFeeRate",
            "docs": [
              "New default creator fee rate (None to keep current)"
            ],
            "type": {
              "option": "u16"
            }
          },
          {
            "name": "maxProtocolFeeRate",
            "docs": [
              "New max protocol fee rate (None to keep current)"
            ],
            "type": {
              "option": "u16"
            }
          },
          {
            "name": "dexTwapUpdater",
            "docs": [
              "Optional: DEX TWAP updater"
            ],
            "type": {
              "option": "pubkey"
            }
          },
          {
            "name": "depegThresholdBps",
            "docs": [
              "Optional: safety thresholds"
            ],
            "type": {
              "option": "u16"
            }
          },
          {
            "name": "depegRequiredObs",
            "type": {
              "option": "u8"
            }
          },
          {
            "name": "clearRequiredObs",
            "type": {
              "option": "u8"
            }
          },
          {
            "name": "dexTwapWindowSecs",
            "docs": [
              "Optional: TWAP timing params"
            ],
            "type": {
              "option": "u32"
            }
          },
          {
            "name": "dexTwapStaleAgeSecs",
            "type": {
              "option": "u32"
            }
          },
          {
            "name": "dexWhitelist",
            "docs": [
              "Replace DEX whitelist (set)"
            ],
            "type": {
              "option": {
                "vec": "pubkey"
              }
            }
          },
          {
            "name": "mintPerSlotCapFeelssol",
            "docs": [
              "Optional: per-slot caps"
            ],
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "redeemPerSlotCapFeelssol",
            "type": {
              "option": "u64"
            }
          }
        ]
      }
    },
    {
      "name": "feels::instructions::initialize_tranche_ticks::InitializeTrancheTicksParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tickStepSize",
            "type": "i32"
          },
          {
            "name": "numSteps",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "feels::instructions::manage_pomm_position::ManagePommParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "positionIndex",
            "docs": [
              "Position index (0-7 for up to 8 POMM positions)"
            ],
            "type": "u8"
          },
          {
            "name": "action",
            "docs": [
              "Action to take"
            ],
            "type": {
              "defined": {
                "name": "feels::instructions::manage_pomm_position::PommAction"
              }
            }
          }
        ]
      }
    },
    {
      "name": "feels::instructions::manage_pomm_position::PommAction",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "addLiquidity"
          },
          {
            "name": "removeLiquidity",
            "fields": [
              {
                "name": "liquidityAmount",
                "type": "u128"
              }
            ]
          },
          {
            "name": "rebalance",
            "fields": [
              {
                "name": "newTickLower",
                "type": "i32"
              },
              {
                "name": "newTickUpper",
                "type": "i32"
              }
            ]
          },
          {
            "name": "collectFees"
          }
        ]
      }
    },
    {
      "name": "feels::instructions::mint_token::MintTokenParams",
      "docs": [
        "Parameters for minting a new token"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "ticker",
            "type": "string"
          },
          {
            "name": "name",
            "type": "string"
          },
          {
            "name": "uri",
            "type": "string"
          }
        ]
      }
    },
    {
      "name": "feels::instructions::swap::SwapParams",
      "docs": [
        "Parameters for swap execution",
        "",
        "These parameters control swap behavior including slippage protection,",
        "tick crossing limits, and fee caps for user protection."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "amountIn",
            "docs": [
              "Amount of input token to swap (gross amount before fees)"
            ],
            "type": "u64"
          },
          {
            "name": "minimumAmountOut",
            "docs": [
              "Minimum amount of output token to receive (after all fees)",
              "Used for slippage protection"
            ],
            "type": "u64"
          },
          {
            "name": "maxTicksCrossed",
            "docs": [
              "Maximum number of ticks to cross during swap (0 = unlimited)",
              "Prevents compute unit exhaustion and potential griefing"
            ],
            "type": "u8"
          },
          {
            "name": "maxTotalFeeBps",
            "docs": [
              "Maximum total fee in basis points (0 = no cap)",
              "Provides user protection against excessive fees"
            ],
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "feels::instructions::swap_exact_out::SwapExactOutParams",
      "docs": [
        "Parameters for exact output swap execution"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "amountOut",
            "docs": [
              "Exact amount of output token to receive (after all fees)"
            ],
            "type": "u64"
          },
          {
            "name": "maximumAmountIn",
            "docs": [
              "Maximum amount of input token willing to pay (before fees)",
              "Used for slippage protection"
            ],
            "type": "u64"
          },
          {
            "name": "maxTicksCrossed",
            "docs": [
              "Maximum number of ticks to cross during swap (0 = unlimited)"
            ],
            "type": "u8"
          },
          {
            "name": "maxTotalFeeBps",
            "docs": [
              "Maximum total fee in basis points (0 = no cap)"
            ],
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "feels::instructions::transition_market_phase::TransitionPhaseParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "targetPhase",
            "docs": [
              "Target phase to transition to"
            ],
            "type": {
              "defined": {
                "name": "feels::state::phase::MarketPhase"
              }
            }
          },
          {
            "name": "force",
            "docs": [
              "Force transition even if criteria not met (governance only)"
            ],
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "feels::instructions::update_protocol_oracle::UpdateDexTwapParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "dexTwapRateQ64",
            "type": "u128"
          },
          {
            "name": "windowSecs",
            "type": "u32"
          },
          {
            "name": "obs",
            "type": "u16"
          },
          {
            "name": "venueId",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "feels::instructions::update_protocol_oracle::UpdateNativeRateParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "nativeRateQ64",
            "type": "u128"
          }
        ]
      }
    },
    {
      "name": "feels::state::buffer::Buffer",
      "docs": [
        "Pool buffer (τ) account"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "market",
            "docs": [
              "Associated market"
            ],
            "type": "pubkey"
          },
          {
            "name": "authority",
            "docs": [
              "Authority that can manage buffer"
            ],
            "type": "pubkey"
          },
          {
            "name": "feelssolMint",
            "docs": [
              "FeelsSOL mint (for reference)"
            ],
            "type": "pubkey"
          },
          {
            "name": "feesToken0",
            "docs": [
              "Token balances (u128 to prevent overflow in high-volume scenarios)"
            ],
            "type": "u128"
          },
          {
            "name": "feesToken1",
            "type": "u128"
          },
          {
            "name": "tauSpot",
            "docs": [
              "τ partition counters (virtual partitions, u128 for overflow safety)"
            ],
            "type": "u128"
          },
          {
            "name": "tauTime",
            "type": "u128"
          },
          {
            "name": "tauLeverage",
            "type": "u128"
          },
          {
            "name": "floorTickSpacing",
            "docs": [
              "Floor LP configuration",
              "DEPRECATED: This field is no longer used. POMM width is now derived from market tick spacing.",
              "Kept for backwards compatibility only."
            ],
            "type": "i32"
          },
          {
            "name": "floorPlacementThreshold",
            "type": "u64"
          },
          {
            "name": "lastFloorPlacement",
            "type": "i64"
          },
          {
            "name": "lastRebase",
            "docs": [
              "Epoch tracking for buffer"
            ],
            "type": "i64"
          },
          {
            "name": "totalDistributed",
            "type": "u128"
          },
          {
            "name": "bufferAuthorityBump",
            "docs": [
              "Canonical bump for buffer authority PDA",
              "Storing the bump prevents ambiguity and improves performance"
            ],
            "type": "u8"
          },
          {
            "name": "jitLastSlot",
            "docs": [
              "JIT per-slot tracking (quote units)"
            ],
            "type": "u64"
          },
          {
            "name": "jitSlotUsedQ",
            "type": "u128"
          },
          {
            "name": "jitRollingConsumption",
            "docs": [
              "JIT v0.5 rolling consumption tracking"
            ],
            "type": "u128"
          },
          {
            "name": "jitRollingWindowStart",
            "type": "u64"
          },
          {
            "name": "jitLastHeavyUsageSlot",
            "type": "u64"
          },
          {
            "name": "jitTotalConsumedEpoch",
            "type": "u128"
          },
          {
            "name": "initialTauSpot",
            "docs": [
              "Initial buffer size for circuit breaker calculations"
            ],
            "type": "u128"
          },
          {
            "name": "protocolOwnedOverride",
            "docs": [
              "Protocol-owned token amount override for floor calculation",
              "If non-zero, this value is used instead of dynamically calculating",
              "Allows governance to set a fixed protocol-owned amount"
            ],
            "type": "u64"
          },
          {
            "name": "pommPositionCount",
            "docs": [
              "Number of active POMM positions"
            ],
            "type": "u8"
          },
          {
            "name": "padding",
            "docs": [
              "Padding for future use"
            ],
            "type": {
              "array": [
                "u8",
                7
              ]
            }
          }
        ]
      }
    },
    {
      "name": "feels::state::escrow::PreLaunchEscrow",
      "docs": [
        "Pre-launch escrow account for newly minted tokens",
        "This temporary account holds tokens and mint fees until market goes live"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tokenMint",
            "docs": [
              "Token mint this escrow is for"
            ],
            "type": "pubkey"
          },
          {
            "name": "creator",
            "docs": [
              "Creator who minted the token"
            ],
            "type": "pubkey"
          },
          {
            "name": "feelssolMint",
            "docs": [
              "FeelsSOL mint (for reference)"
            ],
            "type": "pubkey"
          },
          {
            "name": "createdAt",
            "docs": [
              "Creation timestamp (used for expiration)"
            ],
            "type": "i64"
          },
          {
            "name": "market",
            "docs": [
              "Associated market (set when market is initialized)"
            ],
            "type": "pubkey"
          },
          {
            "name": "escrowAuthorityBump",
            "docs": [
              "Canonical bump for escrow authority PDA"
            ],
            "type": "u8"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved space for future expansion"
            ],
            "type": {
              "array": [
                "u8",
                128
              ]
            }
          }
        ]
      }
    },
    {
      "name": "feels::state::feels_hub::FeelsHub",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "feelssolMint",
            "docs": [
              "FeelsSOL mint this hub controls"
            ],
            "type": "pubkey"
          },
          {
            "name": "reentrancyGuard",
            "docs": [
              "Reentrancy guard for mint/redeem flows"
            ],
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "feels::state::market::FeatureFlags",
      "docs": [
        "Feature flags for future phases (all OFF in MVP)"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "dynamicFees",
            "type": "bool"
          },
          {
            "name": "precisionMode",
            "type": "bool"
          },
          {
            "name": "autopilotLambda",
            "type": "bool"
          },
          {
            "name": "autopilotWeights",
            "type": "bool"
          },
          {
            "name": "targetsAdaptive",
            "type": "bool"
          },
          {
            "name": "timeDomain",
            "type": "bool"
          },
          {
            "name": "leverageDomain",
            "type": "bool"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "bool",
                9
              ]
            }
          }
        ]
      }
    },
    {
      "name": "feels::state::market::Market",
      "docs": [
        "Main market account"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "version",
            "docs": [
              "Version for upgradability"
            ],
            "type": "u8"
          },
          {
            "name": "isInitialized",
            "docs": [
              "Market status"
            ],
            "type": "bool"
          },
          {
            "name": "isPaused",
            "type": "bool"
          },
          {
            "name": "token0",
            "docs": [
              "Token configuration"
            ],
            "type": "pubkey"
          },
          {
            "name": "token1",
            "type": "pubkey"
          },
          {
            "name": "feelssolMint",
            "type": "pubkey"
          },
          {
            "name": "token0Type",
            "docs": [
              "Token types (for future Token-2022 support)"
            ],
            "type": {
              "defined": {
                "name": "feels::state::token_metadata::TokenType"
              }
            }
          },
          {
            "name": "token1Type",
            "type": {
              "defined": {
                "name": "feels::state::token_metadata::TokenType"
              }
            }
          },
          {
            "name": "token0Origin",
            "docs": [
              "Token origins (for market creation restrictions)"
            ],
            "type": {
              "defined": {
                "name": "feels::state::token_metadata::TokenOrigin"
              }
            }
          },
          {
            "name": "token1Origin",
            "type": {
              "defined": {
                "name": "feels::state::token_metadata::TokenOrigin"
              }
            }
          },
          {
            "name": "vault0",
            "docs": [
              "Vault accounts"
            ],
            "type": "pubkey"
          },
          {
            "name": "vault1",
            "type": "pubkey"
          },
          {
            "name": "hubProtocol",
            "docs": [
              "Hub protocol reference (optional)"
            ],
            "type": {
              "option": "pubkey"
            }
          },
          {
            "name": "sqrtPrice",
            "docs": [
              "Spot AMM state (simplified constant product for MVP)"
            ],
            "type": "u128"
          },
          {
            "name": "liquidity",
            "type": "u128"
          },
          {
            "name": "currentTick",
            "docs": [
              "CLMM tick state"
            ],
            "type": "i32"
          },
          {
            "name": "tickSpacing",
            "type": "u16"
          },
          {
            "name": "globalLowerTick",
            "docs": [
              "Floor liquidity bounds (TEMPORARY - will be removed when POMM uses pure positions)",
              "These currently serve as bounds for pool-owned liquidity but will be",
              "replaced with actual position NFTs in a future upgrade.",
              "Global swap bounds - hard limits for all swaps in this market"
            ],
            "type": "i32"
          },
          {
            "name": "globalUpperTick",
            "type": "i32"
          },
          {
            "name": "floorLiquidity",
            "docs": [
              "Liquidity at the global bounds (legacy POMM field, kept for compatibility)"
            ],
            "type": "u128"
          },
          {
            "name": "feeGrowthGlobal0X64",
            "docs": [
              "Global fee growth (Q64) per liquidity unit"
            ],
            "type": "u128"
          },
          {
            "name": "feeGrowthGlobal1X64",
            "type": "u128"
          },
          {
            "name": "feeGrowthGlobal0",
            "docs": [
              "Global fee growth without x64 suffix (for compatibility)"
            ],
            "type": "u128"
          },
          {
            "name": "feeGrowthGlobal1",
            "type": "u128"
          },
          {
            "name": "baseFeeBps",
            "docs": [
              "Fee configuration"
            ],
            "type": "u16"
          },
          {
            "name": "buffer",
            "docs": [
              "Buffer (τ) reference"
            ],
            "type": "pubkey"
          },
          {
            "name": "authority",
            "docs": [
              "Authority"
            ],
            "type": "pubkey"
          },
          {
            "name": "lastEpochUpdate",
            "docs": [
              "Epoch tracking"
            ],
            "type": "i64"
          },
          {
            "name": "epochNumber",
            "type": "u64"
          },
          {
            "name": "oracle",
            "docs": [
              "Oracle account reference",
              "Oracle data is stored in a separate account to reduce stack usage"
            ],
            "type": "pubkey"
          },
          {
            "name": "oracleBump",
            "docs": [
              "Oracle account bump seed"
            ],
            "type": "u8"
          },
          {
            "name": "policy",
            "docs": [
              "Policy configuration"
            ],
            "type": {
              "defined": {
                "name": "feels::state::market::PolicyV1"
              }
            }
          },
          {
            "name": "marketAuthorityBump",
            "docs": [
              "Canonical bump for market authority PDA",
              "Storing prevents recomputation and ensures consistency"
            ],
            "type": "u8"
          },
          {
            "name": "vault0Bump",
            "docs": [
              "Canonical bumps for vault PDAs"
            ],
            "type": "u8"
          },
          {
            "name": "vault1Bump",
            "type": "u8"
          },
          {
            "name": "reentrancyGuard",
            "docs": [
              "Re-entrancy guard",
              "Set to true at the start of sensitive operations and false at the end",
              "Prevents re-entrant calls during critical state transitions"
            ],
            "type": "bool"
          },
          {
            "name": "initialLiquidityDeployed",
            "docs": [
              "Initial liquidity deployment status"
            ],
            "type": "bool"
          },
          {
            "name": "jitEnabled",
            "docs": [
              "JIT v0.5 parameters (per-market)"
            ],
            "type": "bool"
          },
          {
            "name": "jitBaseCapBps",
            "docs": [
              "JIT v0.5 configuration"
            ],
            "type": "u16"
          },
          {
            "name": "jitPerSlotCapBps",
            "type": "u16"
          },
          {
            "name": "jitConcentrationWidth",
            "type": "u32"
          },
          {
            "name": "jitMaxMultiplier",
            "type": "u8"
          },
          {
            "name": "jitDrainProtectionBps",
            "type": "u16"
          },
          {
            "name": "jitCircuitBreakerBps",
            "type": "u16"
          },
          {
            "name": "floorTick",
            "docs": [
              "Floor management (MVP)"
            ],
            "type": "i32"
          },
          {
            "name": "floorBufferTicks",
            "type": "i32"
          },
          {
            "name": "lastFloorRatchetTs",
            "type": "i64"
          },
          {
            "name": "floorCooldownSecs",
            "type": "i64"
          },
          {
            "name": "steadyStateSeeded",
            "docs": [
              "Graduation flags (idempotent)"
            ],
            "type": "bool"
          },
          {
            "name": "cleanupComplete",
            "type": "bool"
          },
          {
            "name": "phase",
            "docs": [
              "Market phase tracking"
            ],
            "type": "u8"
          },
          {
            "name": "phaseStartSlot",
            "type": "u64"
          },
          {
            "name": "phaseStartTimestamp",
            "type": "i64"
          },
          {
            "name": "lastPhaseTransitionSlot",
            "docs": [
              "Phase transition history (last transition)"
            ],
            "type": "u64"
          },
          {
            "name": "lastPhaseTrigger",
            "type": "u8"
          },
          {
            "name": "totalVolumeToken0",
            "docs": [
              "Cumulative metrics for phase transitions"
            ],
            "type": "u64"
          },
          {
            "name": "totalVolumeToken1",
            "type": "u64"
          },
          {
            "name": "rollingBuyVolume",
            "docs": [
              "JIT v0.5 directional tracking (rolling window)"
            ],
            "type": "u128"
          },
          {
            "name": "rollingSellVolume",
            "type": "u128"
          },
          {
            "name": "rollingTotalVolume",
            "type": "u128"
          },
          {
            "name": "rollingWindowStartSlot",
            "type": "u64"
          },
          {
            "name": "tickSnapshot1hr",
            "docs": [
              "Price movement tracking for circuit breaker"
            ],
            "type": "i32"
          },
          {
            "name": "lastSnapshotTimestamp",
            "type": "i64"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved space for future expansion"
            ],
            "type": {
              "array": [
                "u8",
                1
              ]
            }
          }
        ]
      }
    },
    {
      "name": "feels::state::market::PolicyV1",
      "docs": [
        "Policy configuration"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "version",
            "type": "u8"
          },
          {
            "name": "featureFlags",
            "type": {
              "defined": {
                "name": "feels::state::market::FeatureFlags"
              }
            }
          },
          {
            "name": "baseFeeBps",
            "type": "u16"
          },
          {
            "name": "maxSurchargeBps",
            "type": "u16"
          },
          {
            "name": "maxInstantaneousFeeBps",
            "type": "u16"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          }
        ]
      }
    },
    {
      "name": "feels::state::oracle::Observation",
      "docs": [
        "Single price observation"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "blockTimestamp",
            "docs": [
              "Block timestamp of this observation"
            ],
            "type": "i64"
          },
          {
            "name": "tickCumulative",
            "docs": [
              "Cumulative tick value (tick * time)"
            ],
            "type": "i128"
          },
          {
            "name": "initialized",
            "docs": [
              "Whether this observation has been initialized"
            ],
            "type": "bool"
          },
          {
            "name": "padding",
            "docs": [
              "Padding for alignment"
            ],
            "type": {
              "array": [
                "u8",
                7
              ]
            }
          }
        ]
      }
    },
    {
      "name": "feels::state::oracle::OracleState",
      "docs": [
        "Oracle state account that stores price observations"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "poolId",
            "docs": [
              "Pool ID this oracle belongs to"
            ],
            "type": "pubkey"
          },
          {
            "name": "observationIndex",
            "docs": [
              "Index of the most recent observation"
            ],
            "type": "u16"
          },
          {
            "name": "observationCardinality",
            "docs": [
              "Current number of observations (grows from 1 to MAX)"
            ],
            "type": "u16"
          },
          {
            "name": "observationCardinalityNext",
            "docs": [
              "Next observation cardinality (for future expansion)"
            ],
            "type": "u16"
          },
          {
            "name": "oracleBump",
            "docs": [
              "Bump seed for the oracle PDA"
            ],
            "type": "u8"
          },
          {
            "name": "observations",
            "docs": [
              "Array of observations"
            ],
            "type": {
              "array": [
                {
                  "defined": {
                    "name": "feels::state::oracle::Observation"
                  }
                },
                12
              ]
            }
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved for future use"
            ],
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          }
        ]
      }
    },
    {
      "name": "feels::state::phase::MarketPhase",
      "docs": [
        "Market lifecycle phase"
      ],
      "repr": {
        "kind": "rust"
      },
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "created"
          },
          {
            "name": "bondingCurve"
          },
          {
            "name": "transitioning"
          },
          {
            "name": "steadyState"
          },
          {
            "name": "graduated"
          },
          {
            "name": "paused"
          },
          {
            "name": "deprecated"
          }
        ]
      }
    },
    {
      "name": "feels::state::pool_registry::PoolEntry",
      "docs": [
        "Registry entry for a single pool"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "market",
            "docs": [
              "Market pubkey"
            ],
            "type": "pubkey"
          },
          {
            "name": "tokenMint",
            "docs": [
              "Project token mint"
            ],
            "type": "pubkey"
          },
          {
            "name": "feelssolMint",
            "docs": [
              "FeelsSOL mint (always token_0 in markets)"
            ],
            "type": "pubkey"
          },
          {
            "name": "phase",
            "docs": [
              "Current phase"
            ],
            "type": {
              "defined": {
                "name": "feels::state::pool_registry::PoolPhase"
              }
            }
          },
          {
            "name": "createdAt",
            "docs": [
              "Creation timestamp"
            ],
            "type": "i64"
          },
          {
            "name": "updatedAt",
            "docs": [
              "Last update timestamp"
            ],
            "type": "i64"
          },
          {
            "name": "creator",
            "docs": [
              "Creator/launcher"
            ],
            "type": "pubkey"
          },
          {
            "name": "symbol",
            "docs": [
              "Token symbol (up to 10 chars)"
            ],
            "type": {
              "array": [
                "u8",
                10
              ]
            }
          },
          {
            "name": "symbolLen",
            "docs": [
              "Symbol length"
            ],
            "type": "u8"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved for future use"
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    },
    {
      "name": "feels::state::pool_registry::PoolPhase",
      "docs": [
        "Pool phase for lifecycle tracking"
      ],
      "repr": {
        "kind": "rust"
      },
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "bondingCurve"
          },
          {
            "name": "steadyState"
          },
          {
            "name": "paused"
          },
          {
            "name": "deprecated"
          }
        ]
      }
    },
    {
      "name": "feels::state::pool_registry::PoolRegistry",
      "docs": [
        "Central pool registry"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "authority",
            "docs": [
              "Protocol authority"
            ],
            "type": "pubkey"
          },
          {
            "name": "poolCount",
            "docs": [
              "Number of pools registered"
            ],
            "type": "u64"
          },
          {
            "name": "pools",
            "docs": [
              "Pools array (paginated access)"
            ],
            "type": {
              "vec": {
                "defined": {
                  "name": "feels::state::pool_registry::PoolEntry"
                }
              }
            }
          },
          {
            "name": "bump",
            "docs": [
              "Canonical bump"
            ],
            "type": "u8"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved for future use"
            ],
            "type": {
              "array": [
                "u8",
                128
              ]
            }
          }
        ]
      }
    },
    {
      "name": "feels::state::position::Position",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "nftMint",
            "docs": [
              "Position NFT mint (Metaplex Core asset ID)"
            ],
            "type": "pubkey"
          },
          {
            "name": "market",
            "docs": [
              "Market this position belongs to"
            ],
            "type": "pubkey"
          },
          {
            "name": "owner",
            "docs": [
              "Owner of the position"
            ],
            "type": "pubkey"
          },
          {
            "name": "tickLower",
            "docs": [
              "Tick range"
            ],
            "type": "i32"
          },
          {
            "name": "tickUpper",
            "type": "i32"
          },
          {
            "name": "liquidity",
            "docs": [
              "Liquidity amount"
            ],
            "type": "u128"
          },
          {
            "name": "feeGrowthInside0LastX64",
            "docs": [
              "Fee growth inside the position at last update (Q64 fixed point)"
            ],
            "type": "u128"
          },
          {
            "name": "feeGrowthInside1LastX64",
            "type": "u128"
          },
          {
            "name": "tokensOwed0",
            "docs": [
              "Tokens owed to position (collected fees + removed liquidity)"
            ],
            "type": "u64"
          },
          {
            "name": "tokensOwed1",
            "type": "u64"
          },
          {
            "name": "positionBump",
            "docs": [
              "Canonical bump for position PDA",
              "Storing prevents recomputation when minting/burning"
            ],
            "type": "u8"
          },
          {
            "name": "isPomm",
            "docs": [
              "Whether this is a POMM position"
            ],
            "type": "bool"
          },
          {
            "name": "lastUpdatedSlot",
            "docs": [
              "Last slot this position was updated"
            ],
            "type": "u64"
          },
          {
            "name": "feeGrowthInside0Last",
            "docs": [
              "Fee growth inside at last action (for proper accounting)"
            ],
            "type": "u128"
          },
          {
            "name": "feeGrowthInside1Last",
            "type": "u128"
          },
          {
            "name": "feesOwed0",
            "docs": [
              "Accumulated fees owed"
            ],
            "type": "u64"
          },
          {
            "name": "feesOwed1",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "feels::state::protocol_config::ProtocolConfig",
      "docs": [
        "Protocol configuration account"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "authority",
            "docs": [
              "Authority that can update protocol parameters"
            ],
            "type": "pubkey"
          },
          {
            "name": "mintFee",
            "docs": [
              "Fee for minting a new token (in FeelsSOL lamports)"
            ],
            "type": "u64"
          },
          {
            "name": "treasury",
            "docs": [
              "Treasury account to receive protocol fees"
            ],
            "type": "pubkey"
          },
          {
            "name": "defaultProtocolFeeRate",
            "docs": [
              "Default protocol fee rate (basis points, e.g. 1000 = 10%)"
            ],
            "type": "u16"
          },
          {
            "name": "defaultCreatorFeeRate",
            "docs": [
              "Default creator fee rate for protocol tokens (basis points, e.g. 500 = 5%)"
            ],
            "type": "u16"
          },
          {
            "name": "maxProtocolFeeRate",
            "docs": [
              "Maximum allowed protocol fee rate (basis points)"
            ],
            "type": "u16"
          },
          {
            "name": "tokenExpirationSeconds",
            "docs": [
              "Time window (in seconds) for deploying liquidity after token mint",
              "If liquidity isn't deployed within this window, token can be destroyed"
            ],
            "type": "i64"
          },
          {
            "name": "depegThresholdBps",
            "docs": [
              "De-peg circuit breaker threshold (bps of divergence)"
            ],
            "type": "u16"
          },
          {
            "name": "depegRequiredObs",
            "docs": [
              "Required consecutive breach observations to pause"
            ],
            "type": "u8"
          },
          {
            "name": "clearRequiredObs",
            "docs": [
              "Required consecutive clear observations to resume"
            ],
            "type": "u8"
          },
          {
            "name": "dexTwapWindowSecs",
            "docs": [
              "DEX TWAP window and staleness thresholds (seconds)"
            ],
            "type": "u32"
          },
          {
            "name": "dexTwapStaleAgeSecs",
            "type": "u32"
          },
          {
            "name": "dexTwapUpdater",
            "docs": [
              "Authorized updater for DEX TWAP feed (MVP single updater)"
            ],
            "type": "pubkey"
          },
          {
            "name": "dexWhitelist",
            "docs": [
              "DEX whitelist (venues/pools) - fixed size for MVP"
            ],
            "type": {
              "array": [
                "pubkey",
                8
              ]
            }
          },
          {
            "name": "dexWhitelistLen",
            "type": "u8"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved for future protocol parameters"
            ],
            "type": {
              "array": [
                "u8",
                7
              ]
            }
          },
          {
            "name": "mintPerSlotCapFeelssol",
            "docs": [
              "Optional per-slot caps for mint/redeem (FeelsSOL units). 0 = unlimited."
            ],
            "type": "u64"
          },
          {
            "name": "redeemPerSlotCapFeelssol",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "feels::state::protocol_oracle::ProtocolOracle",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "nativeRateQ64",
            "docs": [
              "Native reserve rate (Q64)"
            ],
            "type": "u128"
          },
          {
            "name": "dexTwapRateQ64",
            "docs": [
              "Filtered DEX TWAP rate (Q64)"
            ],
            "type": "u128"
          },
          {
            "name": "dexLastUpdateSlot",
            "docs": [
              "Last update slot for DEX TWAP"
            ],
            "type": "u64"
          },
          {
            "name": "nativeLastUpdateSlot",
            "docs": [
              "Last update slot for native rate"
            ],
            "type": "u64"
          },
          {
            "name": "dexLastUpdateTs",
            "docs": [
              "Last update timestamp for DEX TWAP"
            ],
            "type": "i64"
          },
          {
            "name": "nativeLastUpdateTs",
            "docs": [
              "Last update timestamp for native rate"
            ],
            "type": "i64"
          },
          {
            "name": "dexWindowSecs",
            "docs": [
              "Observation window (seconds) for DEX TWAP"
            ],
            "type": "u32"
          },
          {
            "name": "flags",
            "docs": [
              "Current flags (bitmask)"
            ],
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "feels::state::safety_controller::DegradeFlags",
      "docs": [
        "Degraded mode flags for various safety conditions"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "gtwapStale",
            "docs": [
              "GTWAP stale: disable advanced features"
            ],
            "type": "bool"
          },
          {
            "name": "oracleStale",
            "docs": [
              "Protocol oracle stale: pause exits"
            ],
            "type": "bool"
          },
          {
            "name": "highVolatility",
            "docs": [
              "High volatility detected: raise minimum fees"
            ],
            "type": "bool"
          },
          {
            "name": "lowLiquidity",
            "docs": [
              "Low liquidity: restrict large trades"
            ],
            "type": "bool"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved flags for future use"
            ],
            "type": {
              "array": [
                "bool",
                4
              ]
            }
          }
        ]
      }
    },
    {
      "name": "feels::state::safety_controller::SafetyController",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "redemptionsPaused",
            "docs": [
              "Whether redemptions are paused due to de-peg"
            ],
            "type": "bool"
          },
          {
            "name": "consecutiveBreaches",
            "docs": [
              "Consecutive divergence observations over threshold"
            ],
            "type": "u8"
          },
          {
            "name": "consecutiveClears",
            "docs": [
              "Consecutive safe observations since last breach"
            ],
            "type": "u8"
          },
          {
            "name": "lastChangeSlot",
            "docs": [
              "Last state change slot"
            ],
            "type": "u64"
          },
          {
            "name": "mintLastSlot",
            "docs": [
              "Per-slot mint tracking (FeelsSOL units)"
            ],
            "type": "u64"
          },
          {
            "name": "mintSlotAmount",
            "type": "u64"
          },
          {
            "name": "redeemLastSlot",
            "docs": [
              "Per-slot redeem tracking (FeelsSOL units)"
            ],
            "type": "u64"
          },
          {
            "name": "redeemSlotAmount",
            "type": "u64"
          },
          {
            "name": "lastDivergenceCheckSlot",
            "docs": [
              "Last slot when divergence was checked (prevents double-counting)"
            ],
            "type": "u64"
          },
          {
            "name": "degradeFlags",
            "docs": [
              "Degraded mode flags"
            ],
            "type": {
              "defined": {
                "name": "feels::state::safety_controller::DegradeFlags"
              }
            }
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved for future use"
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    },
    {
      "name": "feels::state::tick::Tick",
      "docs": [
        "Individual tick within an array",
        "Must be exactly aligned with no padding for zero_copy"
      ],
      "serialization": "bytemuck",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "liquidityNet",
            "type": "i128"
          },
          {
            "name": "liquidityGross",
            "type": "u128"
          },
          {
            "name": "feeGrowthOutside0X64",
            "type": "u128"
          },
          {
            "name": "feeGrowthOutside1X64",
            "type": "u128"
          },
          {
            "name": "initialized",
            "type": "u8"
          },
          {
            "name": "pad",
            "type": {
              "array": [
                "u8",
                15
              ]
            }
          }
        ]
      }
    },
    {
      "name": "feels::state::tick::TickArray",
      "serialization": "bytemuck",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "startTickIndex",
            "type": "i32"
          },
          {
            "name": "pad0",
            "type": {
              "array": [
                "u8",
                12
              ]
            }
          },
          {
            "name": "ticks",
            "type": {
              "array": [
                {
                  "defined": {
                    "name": "feels::state::tick::Tick"
                  }
                },
                64
              ]
            }
          },
          {
            "name": "initializedTickCount",
            "type": "u16"
          },
          {
            "name": "pad1",
            "type": {
              "array": [
                "u8",
                14
              ]
            }
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    },
    {
      "name": "feels::state::token_metadata::ProtocolToken",
      "docs": [
        "Registry entry for protocol-minted tokens"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "docs": [
              "Token mint address"
            ],
            "type": "pubkey"
          },
          {
            "name": "creator",
            "docs": [
              "Creator who minted the token"
            ],
            "type": "pubkey"
          },
          {
            "name": "tokenType",
            "docs": [
              "Token type (SPL or Token-2022)"
            ],
            "type": {
              "defined": {
                "name": "feels::state::token_metadata::TokenType"
              }
            }
          },
          {
            "name": "createdAt",
            "docs": [
              "Creation timestamp"
            ],
            "type": "i64"
          },
          {
            "name": "canCreateMarkets",
            "docs": [
              "Whether this token can create markets (for future use)"
            ],
            "type": "bool"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved for future use"
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    },
    {
      "name": "feels::state::token_metadata::TokenOrigin",
      "docs": [
        "Origin of a token"
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "protocolMinted"
          },
          {
            "name": "external"
          },
          {
            "name": "feelsSol"
          }
        ]
      }
    },
    {
      "name": "feels::state::token_metadata::TokenType",
      "docs": [
        "Token type enum for tracking SPL vs Token-2022"
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "spl"
          },
          {
            "name": "token2022"
          }
        ]
      }
    },
    {
      "name": "feels::state::tranche_plan::TrancheEntry",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tickLower",
            "type": "i32"
          },
          {
            "name": "tickUpper",
            "type": "i32"
          },
          {
            "name": "liquidity",
            "type": "u128"
          }
        ]
      }
    },
    {
      "name": "feels::state::tranche_plan::TranchePlan",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "market",
            "type": "pubkey"
          },
          {
            "name": "applied",
            "type": "bool"
          },
          {
            "name": "count",
            "type": "u8"
          },
          {
            "name": "entries",
            "type": {
              "vec": {
                "defined": {
                  "name": "feels::state::tranche_plan::TrancheEntry"
                }
              }
            }
          }
        ]
      }
    }
  ]
};
