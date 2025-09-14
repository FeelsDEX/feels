# Token Launch Sequence

This document details the complete process of launching a new token on the Feels Protocol, from acquiring FeelsSOL through token minting, pool initialization, and liquidity deployment via a bonding curve, through graduation to steady state.

## Step 1: Convert JitoSOL to FeelsSOL

The first step is to acquire FeelsSOL, the protocol's hub token. All pools in the Feels Protocol require one side of the pair to be FeelsSOL.

### Instruction: `enter_feelssol`

Parameters:
- `amount: u64` - Amount of JitoSOL to deposit

### Accounts

| Account | Description |
|---------|-------------|
| `user` | Signer, must be a system account |
| `user_jitosol` | User's JitoSOL token account |
| `user_feelssol` | User's FeelsSOL token account |
| `jitosol_mint` | JitoSOL mint |
| `feelssol_mint` | FeelsSOL mint |
| `jitosol_vault` | Protocol's JitoSOL vault (PDA: `[b"jitosol_vault", feelssol_mint]`) |
| `mint_authority` | PDA that controls FeelsSOL minting (`[b"mint_authority", feelssol_mint]`) |
| `token_program` | SPL Token program |
| `system_program` | System program |

Process:
1. Transfers JitoSOL from user to protocol vault
2. Mints FeelsSOL 1:1 to user
3. Emits FeelsSOLMinted event

## Step 2: Mint Protocol Token

Create a new SPL token with all supply allocated to the protocol escrow. Requires payment of protocol mint fee.

### Instruction: `mint_token`

Parameters (MintTokenParams):
- `ticker: String` - Token symbol (max 10 chars)
- `name: String` - Token name (max 32 chars)
- `uri: String` - Metadata URI

### Accounts

| Account | Description |
|---------|-------------|
| `creator` | Signer, must be a system account |
| `token_mint` | New token mint to create (signer) |
| `escrow` | Pre-launch escrow account for this token (PDA: `[b"escrow", token_mint]`) |
| `escrow_token_vault` | Escrow's token vault (ATA) |
| `escrow_feelssol_vault` | Escrow's FeelsSOL vault (ATA) |
| `escrow_authority` | PDA controlling escrow vaults (`[b"escrow_authority", escrow]`) |
| `metadata` | Metaplex metadata account (PDA) |
| `feelssol_mint` | FeelsSOL mint |
| `creator_feelssol` | Creator's FeelsSOL account (must have mint fee balance) |
| `protocol_config` | Protocol configuration account (PDA: `[b"protocol_config"]`) |
| `protocol_token` | Protocol token registry entry (PDA: `[b"protocol_token", token_mint]`) |
| `associated_token_program` | Associated Token program |
| `rent` | Rent sysvar |
| `token_program` | SPL Token program |
| `system_program` | System program |
| `metadata_program` | Metaplex Token Metadata program |

Process:
1. Validate parameters early (ticker ≤ 10 chars, name ≤ 32 chars, URI ≤ 200 chars)
2. Validate creator has at least mint fee balance (amount specified in protocol_config)
3. Creates new SPL token with 6 decimals
4. Mints entire supply (1,000,000,000 tokens) to escrow
5. Creates Metaplex metadata
6. Does NOT revoke mint/freeze authorities (deferred to market initialization)
7. Transfers mint fee from creator to escrow (held until market success or expiration)
8. Initializes PreLaunchEscrow state with token mint info
9. Creates ProtocolToken registry entry marking creator as authorized market launcher


## Step 3: Initialize Pool

Create a new trading pool pairing the protocol token with FeelsSOL. Only the token creator can perform this step.

### Instruction: `initialize_pool`

Parameters (InitializeMarketParams):
- `base_fee_bps: u16` - Trading fee in basis points (e.g., 30 = 0.3%)
- `tick_spacing: u16` - Tick spacing for positions
- `initial_sqrt_price: u128` - Initial price as sqrt(price) * 2^64
- `initial_buy_feelssol_amount: u64` - Set to 0 (initial buy happens during deployment)

### Accounts

| Account | Description |
|---------|-------------|
| `creator` | Signer, must match token creator |
| `token_0` | Lower pubkey mint (FeelsSOL or protocol token) |
| `token_1` | Higher pubkey mint (FeelsSOL or protocol token) |
| `pool` | Pool account to create (PDA: `[b"pool", token_0, token_1]`) |
| `pool_buffer` | Pool Buffer account (PDA: `[b"pool_buffer", pool]`) |
| `pool_oracle` | Pool Oracle state account (PDA: `[b"pool_oracle", pool]`) |
| `vault_0` | Token 0 vault (PDA: `[b"vault", pool, token_0]`) |
| `vault_1` | Token 1 vault (PDA: `[b"vault", pool, token_1]`) |
| `pool_authority` | PDA controlling vaults (`[b"authority", pool]`) |
| `feelssol_mint` | FeelsSOL mint |
| `protocol_token_0` | Protocol token registry for token_0 (or dummy if FeelsSOL) |
| `protocol_token_1` | Protocol token registry for token_1 (or dummy if FeelsSOL) |
| `escrow` | Pre-launch escrow for the protocol token |
| `creator_feelssol` | Dummy account (initial buy moved to deployment) |
| `creator_token_out` | Dummy account (initial buy moved to deployment) |
| `system_program` | System program |
| `token_program` | SPL Token program |
| `rent` | Rent sysvar |

Process:
1. Validate tick spacing and fee tier parameters early
2. If initial buy specified, validate creator has required FeelsSOL balance
3. Validates token order (lower pubkey must be token_0)
4. Ensures at least one token is FeelsSOL
5. Verifies creator authorization for protocol-minted tokens
6. Revokes mint and freeze authorities for protocol-minted tokens
7. Initializes Pool state with initial price
8. Initializes PoolBuffer (separate from pre-launch escrow)
9. Initializes Pool Oracle state
10. Creates empty vaults controlled by pool authority
11. Updates escrow account to link to new pool


## Step 4: Deploy Bonding Curve Liquidity

This step bootstraps the pool with initial liquidity to facilitate price discovery. We deploy a discretized bonding curve (virtual `x*y=k`) using the CLMM engine. After graduation, the protocol transitions to steady state (Floor + JIT).

### Instruction: `deploy_bonding_curve_liquidity`

Parameters (DeployInitialLiquidityParams):
- `tick_step_size: i32` - Number of ticks between stair steps (e.g., 100)
- `initial_buy_feelssol_amount: u64` - 1000 FeelsSOL worth for initial buy

### Accounts

| Account | Description |
|---------|-------------|
| `deployer` | Signer, must be pool authority (creator) |
| `pool` | Pool account |
| `deployer_feelssol` | Deployer's FeelsSOL account for initial buy |
| `deployer_token_out` | Deployer's account to receive bought tokens |
| `vault_0` | Token 0 vault |
| `vault_1` | Token 1 vault |
| `pool_authority` | Pool authority PDA |
| `pool_buffer` | Pool Buffer account (for fee collection, separate from pre-launch escrow) |
| `escrow` | Pre-launch escrow for the protocol token |
| `escrow_token_vault` | Escrow's token vault (from mint_token) |
| `escrow_feelssol_vault` | Escrow's FeelsSOL vault |
| `escrow_authority` | Escrow authority PDA |
| `protocol_config` | Protocol configuration |
| `treasury` | Treasury to receive mint fee |
| `token_program` | SPL Token program |
| `system_program` | System program |

Process:
1. Validate discretization params (N, step size aligned to tick spacing)
2. Verify deployer has required fees if any
3. Calculate deployment amounts from escrow
4. Validate sufficient tokens in escrow

### Step 4.1: Deploy Discretized Curve
1. Transfers seed tokens from escrow to pool vault(s)
2. Creates N−1 micro‑range positions with liquidity sized to approximate `x*y=k`
3. Aligns ticks to spacing and updates active liquidity

### Step 4.2: (Optional) Initial Buy
1. Transfers initial buy FeelsSOL from deployer to appropriate vault
2. Calculates output amount using current pool price
3. Transfers calculated tokens from vault to deployer's token account

### Step 4.3: Finalize Deployment
1. Transfer mint fee from escrow to treasury
2. Transfer deployment fee from deployer to treasury (if applicable)
3. Mark pool as having bonding curve deployed

The initial buy executes at the current market price, guaranteeing the deployer gets the best available price as the first buyer.

## Step 5: Token Expiration (Optional)

If liquidity is not deployed within the expiration window (set in protocol_config.token_expiration_seconds), the token can be destroyed by anyone to reclaim resources and prevent ticker squatting.

### Instruction: `destroy_expired_token`

### Accounts

| Account | Description |
|---------|-------------|
| `destroyer` | Anyone can call this instruction |
| `token_mint` | Token mint to destroy |
| `protocol_token` | Protocol token registry entry (closed) |
| `escrow` | Pre-launch escrow account (closed) |
| `escrow_token_vault` | Escrow's token vault (closed) |
| `escrow_feelssol_vault` | Escrow's FeelsSOL vault (closed) |
| `escrow_authority` | Escrow authority PDA |
| `protocol_config` | Protocol configuration |
| `treasury` | Treasury to receive 50% of mint fee |
| `destroyer_feelssol` | Destroyer's account to receive 50% of mint fee |
| `pool` | Optional pool account if it was created (closed if exists) |
| `associated_token_program` | Associated Token program |
| `token_program` | SPL Token program |
| `system_program` | System program |

Process:
1. Verifies token has expired (current_time > created_at + expiration_seconds)
2. Ensures no liquidity was deployed if market exists
3. Burns all tokens remaining in escrow
4. Splits mint fee: 50% to destroyer as reward, 50% to treasury
5. Closes all accounts, returning rent to destroyer
6. Emits TokenDestroyed event

---

## Complete Example

**Sequence Diagram**

```mermaid
sequenceDiagram
    participant User
    participant Program
    participant Protocol_Config
    participant JitoSOL_Vault
    participant FeelsSOL_Mint
    participant Token_Mint
    participant Escrow
    participant Market
    participant Market_Vaults
    participant Treasury
    participant Anyone_Destroyer
    
    Note over User: Starting with 3000 FeelsSOL
    
    rect rgb(240, 240, 240)
        Note right of User: Step 2: Mint Token
        User->>Program: mint_token("FEELS") + 1000 FeelsSOL fee
        Program->>Protocol_Config: Check mint fee
        Program->>Escrow: Store 1000 FeelsSOL fee (not treasury yet)
        Program->>Token_Mint: Create SPL token
        Program->>Escrow: Initialize escrow
        Program->>Token_Mint: Mint 1B tokens
        Token_Mint->>Escrow: 1B FEELS to escrow
        Note over Token_Mint: Authorities NOT revoked yet
    end
    
    rect rgb(236, 247, 235)
        Note right of User: Step 3: Initialize Pool
        User->>Program: initialize_pool(price=0.001)
        Program->>Program: Verify creator authorization
        Program->>Token_Mint: Revoke mint authority (None)
        Program->>Token_Mint: Revoke freeze authority (None)
        Note over Token_Mint: Fixed supply, unfrozen forever
        Program->>Market: Create market account
        Program->>Market: Set initial sqrt_price
        Program->>Market_Vaults: Create empty vaults
        Program->>Escrow: Link escrow to market
    end
    
    rect rgb(240, 240, 240)
        Note right of User: Step 4: Deploy Bonding Curve
        User->>Program: deploy_bonding_curve_liquidity(initial_buy=1000)
        
        Note over Program: Deploy Staircase
        Program->>Escrow: Transfer 80% of tokens
        Escrow->>Market_Vaults: 800M FEELS
        Program->>Pool: Create discretized curve positions
        
        Note over Program: Transfer fees
        Escrow->>Treasury: Transfer 1000 FeelsSOL mint fee
        User->>Treasury: Transfer 1000 FeelsSOL deployment fee
        
        Note over Program: Execute Initial Buy
        User->>Market_Vaults: 1000 FeelsSOL
        Program->>Program: Calculate output (~1M FEELS)
        Pool_Vaults->>User: Receive ~1M FEELS
        
        Note over Pool: Pool is now live
        Note over Escrow: 200M FEELS (20%) retained
    end
    
    alt        
        Note over Program: If no liquidity deployed before expiry...
        Anyone_Destroyer->>Program: destroy_expired_token()
        Program->>Protocol_Config: Check expiration time
        Program->>Pool: Check if liquidity deployed (if pool exists)
        
        Note over Program: Token has expired, proceed with destruction
        
        Program->>Escrow: Get mint fee balance (1000 FeelsSOL)
        Program->>Escrow: Transfer 50% fee to destroyer
        Escrow->>Anyone_Destroyer: 500 FeelsSOL reward
        Program->>Escrow: Transfer 50% fee to treasury
        Escrow->>Treasury: 500 FeelsSOL
        
        Program->>Escrow: Burn all tokens in escrow
        Program->>Escrow: Close escrow account
        Program->>Token_Mint: Close token accounts
        Program->>Pool: Close pool (if exists)
        
        Note over Anyone_Destroyer: Earned rent + 50% mint fee
        Note over Token_Mint: Token destroyed, ticker available again
    end
```

**Example Flow**

Starting with 3000 FeelsSOL for a complete token launch:

1. Mint Token: 
   - Pay 1000 FeelsSOL mint fee (stored in escrow, not sent to treasury)
   - Create FEELS token with 1B supply to escrow
   - Mint and freeze authorities NOT revoked yet
2. Initialize Market: 
   - Verify creator authorization
   - Revoke mint and freeze authorities permanently
   - Set initial price at 0.001 FeelsSOL per FEELS (sqrt_price ≈ 2071319988 << 64)
   - Link escrow to market
3. Deploy Liquidity: 
   - Protocol deploys 800M FEELS (80%) across 10 positions
   - Pay 1000 FeelsSOL deployment fee from user to treasury
   - Transfer mint fee (1000 FeelsSOL) from escrow to treasury
   - User includes 1000 FeelsSOL for initial buy
   - User receives ~1,000,000 FEELS at the initial price

Final state:
- Treasury: 2000 FeelsSOL (mint fee + deployment fee)
- Escrow: 200M FEELS (20% retained)
- Market liquidity: 800M FEELS distributed across 10 positions
- User: ~1M FEELS from initial buy
- Market is live and tradeable
