use super::*;
use std::collections::HashMap;
use std::sync::Arc;
use solana_sdk::instruction::AccountMeta;

/// Builder for creating test markets with common configurations
pub struct MarketBuilder {
    token_mint_0: Option<Pubkey>,
    token_mint_1: Option<Pubkey>,
    tick_spacing: u16,
    initial_sqrt_price: Option<u128>,
    fee_tier: u16,
    with_liquidity: Vec<LiquidityConfig>,
    with_tick_arrays: Vec<i32>,
}

#[derive(Clone, Debug)]
pub struct LiquidityConfig {
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: u128,
    pub provider: Option<Arc<Keypair>>,
}

impl Default for MarketBuilder {
    fn default() -> Self {
        Self {
            token_mint_0: None,
            token_mint_1: None,
            tick_spacing: 64,
            initial_sqrt_price: None,
            fee_tier: 500, // 5 bps
            with_liquidity: Vec::new(),
            with_tick_arrays: Vec::new(),
        }
    }
}

impl MarketBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_tokens(mut self, mint_0: Pubkey, mint_1: Pubkey) -> Self {
        self.token_mint_0 = Some(mint_0);
        self.token_mint_1 = Some(mint_1);
        self
    }
    
    pub fn with_tick_spacing(mut self, spacing: u16) -> Self {
        self.tick_spacing = spacing;
        self
    }
    
    pub fn with_initial_sqrt_price(mut self, price: u128) -> Self {
        self.initial_sqrt_price = Some(price);
        self
    }
    
    pub fn with_fee_tier(mut self, fee_tier: u16) -> Self {
        self.fee_tier = fee_tier;
        self
    }
    
    pub fn add_liquidity(mut self, tick_lower: i32, tick_upper: i32, liquidity: u128) -> Self {
        self.with_liquidity.push(LiquidityConfig {
            tick_lower,
            tick_upper,
            liquidity,
            provider: None,
        });
        self
    }
    
    pub fn add_tick_array(mut self, start_index: i32) -> Self {
        self.with_tick_arrays.push(start_index);
        self
    }
    
    pub fn add_tick_arrays_around_current(mut self, current_tick: i32, count: usize) -> Self {
        let array_size = feels::state::TICK_ARRAY_SIZE as i32 * self.tick_spacing as i32;
        let current_array_start = (current_tick / array_size) * array_size;
        
        for i in 0..count {
            let offset = (i as i32 - count as i32 / 2) * array_size;
            self.with_tick_arrays.push(current_array_start + offset);
        }
        self
    }
    
    pub async fn build(self, suite: &mut TestSuite) -> TestResult<MarketTestData> {
        // Ensure token mints are provided
        let token_mint_0 = self.token_mint_0
            .ok_or_else(|| "Token mint 0 not provided")?;
        let token_mint_1 = self.token_mint_1
            .ok_or_else(|| "Token mint 1 not provided")?;
        
        // Calculate initial sqrt price if not provided
        let initial_sqrt_price = self.initial_sqrt_price
            .unwrap_or_else(|| feels::utils::sqrt_price_from_tick(0).unwrap());
        
        // Create market using initialize_market instruction
        
        // Derive the market PDA
        let (market, _) = Pubkey::find_program_address(
            &[b"market", token_mint_0.as_ref(), token_mint_1.as_ref()],
            &suite.program_id,
        );
        
        // Build accounts manually since we're in test context
        let accounts = vec![
            AccountMeta::new(suite.payer.pubkey(), true),  // authority (signer)
            AccountMeta::new(market, false),                // market account
            AccountMeta::new_readonly(token_mint_0, false), // token_0 mint
            AccountMeta::new_readonly(token_mint_1, false), // token_1 mint
            AccountMeta::new_readonly(system_program::id(), false), // system program
            AccountMeta::new_readonly(spl_token::id(), false),      // token program
        ];
        
        // Create instruction data manually
        let mut data = Vec::with_capacity(8 + 2 + 2 + 16);
        
        // Add discriminator for initialize_market (simplified - in prod, calculate proper hash)
        data.extend_from_slice(&[0u8; 8]);
        
        // Serialize parameters
        data.extend_from_slice(&self.fee_tier.to_le_bytes());
        data.extend_from_slice(&self.tick_spacing.to_le_bytes());
        data.extend_from_slice(&initial_sqrt_price.to_le_bytes());
        
        let ix = Instruction {
            program_id: suite.program_id,
            accounts,
            data,
        };
        
        let payer = suite.payer.insecure_clone();
        suite.process_transaction(&[ix], &[&payer]).await?;
        
        // Initialize tick arrays
        let mut tick_arrays = HashMap::new();
        // TODO: Uncomment when test-utils feature is enabled
        for start_index in &self.with_tick_arrays {
            // Derive tick array PDA
            let (array_pda, _) = Pubkey::find_program_address(
                &[b"tick_array", market.as_ref(), &start_index.to_le_bytes()],
                &suite.program_id,
            );
            tick_arrays.insert(*start_index, array_pda);
        }
        
        // Add liquidity positions (placeholder for future implementation)
        let mut positions = Vec::new();
        for _config in &self.with_liquidity {
            // TODO: Create position when position opening is implemented
            // For now, just add a placeholder pubkey
            positions.push(Pubkey::new_unique());
        }
        
        Ok(MarketTestData {
            market,
            token_mint_0,
            token_mint_1,
            tick_spacing: self.tick_spacing,
            fee_tier: self.fee_tier,
            tick_arrays,
            positions,
        })
    }
}

/// Result of building a test market
#[derive(Debug, Clone)]
pub struct MarketTestData {
    pub market: Pubkey,
    pub token_mint_0: Pubkey,
    pub token_mint_1: Pubkey,
    pub tick_spacing: u16,
    pub fee_tier: u16,
    pub tick_arrays: HashMap<i32, Pubkey>,
    pub positions: Vec<Pubkey>,
}

/// Builder for swap tests
pub struct SwapTestBuilder {
    market: Option<Pubkey>,
    user: Option<Keypair>,
    amount_in: u64,
    minimum_amount_out: u64,
    tick_arrays: Vec<Pubkey>,
    zero_for_one: bool,
    max_ticks_crossed: Option<u8>,
}

impl Default for SwapTestBuilder {
    fn default() -> Self {
        Self {
            market: None,
            user: None,
            amount_in: 0,
            minimum_amount_out: 0,
            tick_arrays: Vec::new(),
            zero_for_one: true,
            max_ticks_crossed: None,
        }
    }
}

impl SwapTestBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_market(mut self, market: Pubkey) -> Self {
        self.market = Some(market);
        self
    }
    
    pub fn with_user(mut self, user: Keypair) -> Self {
        self.user = Some(user);
        self
    }
    
    pub fn with_amount(mut self, amount: u64) -> Self {
        self.amount_in = amount;
        self
    }
    
    pub fn with_minimum_output(mut self, amount: u64) -> Self {
        self.minimum_amount_out = amount;
        self
    }
    
    pub fn with_tick_arrays(mut self, arrays: Vec<Pubkey>) -> Self {
        self.tick_arrays = arrays;
        self
    }
    
    pub fn zero_for_one(mut self, direction: bool) -> Self {
        self.zero_for_one = direction;
        self
    }
    
    pub fn with_max_ticks(mut self, max: u8) -> Self {
        self.max_ticks_crossed = Some(max);
        self
    }
    
    pub async fn execute(self, suite: &mut TestSuite) -> TestResult<SwapResult> {
        let market = self.market
            .ok_or_else(|| "Market not provided")?;
        let user = self.user
            .ok_or_else(|| "User not provided")?;
        
        // Get market data to determine token accounts
        let market_data = suite.get_account_data::<Market>(&market).await?;
        
        // Create or get user token accounts
        let (user_token_in, user_token_out) = if self.zero_for_one {
            // Get or create user's token accounts for token_0 and token_1
            let user_token_0 = spl_associated_token_account::get_associated_token_address(
                &user.pubkey(),
                &market_data.token_0,
            );
            let user_token_1 = spl_associated_token_account::get_associated_token_address(
                &user.pubkey(),
                &market_data.token_1,
            );
            
            // Create ATAs if they don't exist
            suite.create_ata_if_needed(&user.pubkey(), &market_data.token_0).await?;
            suite.create_ata_if_needed(&user.pubkey(), &market_data.token_1).await?;
            
            (user_token_0, user_token_1)
        } else {
            // Get or create user's token accounts for token_1 and token_0
            let user_token_0 = spl_associated_token_account::get_associated_token_address(
                &user.pubkey(),
                &market_data.token_0,
            );
            let user_token_1 = spl_associated_token_account::get_associated_token_address(
                &user.pubkey(),
                &market_data.token_1,
            );
            
            // Create ATAs if they don't exist
            suite.create_ata_if_needed(&user.pubkey(), &market_data.token_0).await?;
            suite.create_ata_if_needed(&user.pubkey(), &market_data.token_1).await?;
            
            (user_token_1, user_token_0)
        };
        
        // Call swap_with_arrays
        suite.swap_with_arrays(
            market,
            &user,
            user_token_in,
            user_token_out,
            self.amount_in,
            self.minimum_amount_out,
            self.tick_arrays,
        ).await
    }
}

/// Builder for position tests
pub struct PositionBuilder {
    market: Option<Pubkey>,
    owner: Option<Keypair>,
    tick_lower: Option<i32>,
    tick_upper: Option<i32>,
    liquidity_amount: u128,
    token_0_amount: Option<u64>,
    token_1_amount: Option<u64>,
}

impl Default for PositionBuilder {
    fn default() -> Self {
        Self {
            market: None,
            owner: None,
            tick_lower: None,
            tick_upper: None,
            liquidity_amount: 0,
            token_0_amount: None,
            token_1_amount: None,
        }
    }
}

impl PositionBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_market(mut self, market: Pubkey) -> Self {
        self.market = Some(market);
        self
    }
    
    pub fn with_owner(mut self, owner: Keypair) -> Self {
        self.owner = Some(owner);
        self
    }
    
    pub fn with_ticks(mut self, lower: i32, upper: i32) -> Self {
        self.tick_lower = Some(lower);
        self.tick_upper = Some(upper);
        self
    }
    
    pub fn with_liquidity(mut self, amount: u128) -> Self {
        self.liquidity_amount = amount;
        self
    }
    
    pub fn with_tokens(mut self, amount_0: u64, amount_1: u64) -> Self {
        self.token_0_amount = Some(amount_0);
        self.token_1_amount = Some(amount_1);
        self
    }
    
    pub async fn open(self, suite: &mut TestSuite) -> TestResult<Pubkey> {
        let market = self.market
            .ok_or_else(|| "Market not provided")?;
        let owner = self.owner
            .ok_or_else(|| "Owner not provided")?;
        let tick_lower = self.tick_lower
            .ok_or_else(|| "Lower tick not provided")?;
        let tick_upper = self.tick_upper
            .ok_or_else(|| "Upper tick not provided")?;
        
        // Open position using open_position instruction
        
        // Generate a new position mint
        let position_mint = Keypair::new();
        
        // Derive position PDA
        let (position, _) = Pubkey::find_program_address(
            &[b"position", position_mint.pubkey().as_ref()],
            &suite.program_id,
        );
        
        // Create position metadata account
        let (metadata, _) = feels::utils::derive_metadata(&position_mint.pubkey());
        
        // Get market data
        let market_data = suite.get_account_data::<Market>(&market).await?;
        
        // Derive vaults
        let (vault_0, _) = Market::derive_vault_address(&market, &market_data.token_0, &suite.program_id);
        let (vault_1, _) = Market::derive_vault_address(&market, &market_data.token_1, &suite.program_id);
        let (market_authority, _) = Market::derive_market_authority(&market, &suite.program_id);
        
        // Create user token accounts if needed
        let owner_token_0 = suite.create_token_account(&market_data.token_0, &owner.pubkey()).await?;
        let owner_token_1 = suite.create_token_account(&market_data.token_1, &owner.pubkey()).await?;
        
        // Create position token account
        let position_token_account = suite.create_token_account(&position_mint.pubkey(), &owner.pubkey()).await?;
        
        // Get tick arrays
        // Derive tick array addresses for the position's tick range
        let tick_spacing = market_data.tick_spacing;
        let tick_array_size = 64; // TICK_ARRAY_SIZE constant
        
        // Calculate tick array start indices
        let lower_tick_array_start = (tick_lower / (tick_spacing as i32 * tick_array_size)) * (tick_spacing as i32 * tick_array_size);
        let upper_tick_array_start = (tick_upper / (tick_spacing as i32 * tick_array_size)) * (tick_spacing as i32 * tick_array_size);
        
        // Derive tick array PDAs
        let (lower_tick_array, _) = Pubkey::find_program_address(
            &[
                b"tick_array",
                market.as_ref(),
                &lower_tick_array_start.to_le_bytes(),
            ],
            &suite.program_id,
        );
        
        let (upper_tick_array, _) = Pubkey::find_program_address(
            &[
                b"tick_array",
                market.as_ref(),
                &upper_tick_array_start.to_le_bytes(),
            ],
            &suite.program_id,
        );
        
        // Initialize tick arrays if they don't exist
        suite.ensure_tick_array_initialized(&market, lower_tick_array_start).await?;
        suite.ensure_tick_array_initialized(&market, upper_tick_array_start).await?;
        
        // Mint tokens to owner accounts if token amounts were specified
        if let Some(amount_0) = self.token_0_amount {
            suite.mint_tokens(&market_data.token_0, &owner_token_0.pubkey(), amount_0).await?;
        }
        if let Some(amount_1) = self.token_1_amount {
            suite.mint_tokens(&market_data.token_1, &owner_token_1.pubkey(), amount_1).await?;
        }
        
        // Build the open position instruction using manual account metas
        let accounts = vec![
            AccountMeta::new(owner.pubkey(), true),                          // provider (signer)
            AccountMeta::new(market, false),                                 // market
            AccountMeta::new(position_mint.pubkey(), false),                 // position_mint
            AccountMeta::new(position_token_account.pubkey(), false),        // position_token_account
            AccountMeta::new(position, false),                               // position PDA
            AccountMeta::new(owner_token_0.pubkey(), false),                 // provider_token_0
            AccountMeta::new(owner_token_1.pubkey(), false),                 // provider_token_1
            AccountMeta::new(vault_0, false),                                // vault_0
            AccountMeta::new(vault_1, false),                                // vault_1
            AccountMeta::new(lower_tick_array, false),                       // lower_tick_array
            AccountMeta::new(upper_tick_array, false),                       // upper_tick_array
            AccountMeta::new_readonly(market_authority, false),              // market_authority
            AccountMeta::new_readonly(spl_token::id(), false),              // token_program
            AccountMeta::new_readonly(system_program::id(), false),         // system_program
        ];
        
        // Create instruction data manually
        let mut data = Vec::with_capacity(8 + 4 + 4 + 16);
        
        // Add discriminator for open_position (simplified - in prod, calculate proper hash)
        data.extend_from_slice(&[0u8; 8]);
        
        // Serialize parameters
        data.extend_from_slice(&tick_lower.to_le_bytes());
        data.extend_from_slice(&tick_upper.to_le_bytes());
        data.extend_from_slice(&self.liquidity_amount.to_le_bytes());
        
        let ix = Instruction {
            program_id: suite.program_id,
            accounts,
            data,
        };
        
        // Execute transaction
        suite.process_transaction(&[ix], &[&owner, &position_mint]).await?;
        
        Ok(position)
    }
}

/// Test data factory for common scenarios
pub struct TestDataFactory;

impl TestDataFactory {
    /// Create a standard test market with default settings
    pub fn standard_market() -> MarketBuilder {
        MarketBuilder::new()
            .with_tick_spacing(64)
            .with_fee_tier(500)
            .add_tick_arrays_around_current(0, 5)
            .add_liquidity(-1000, 1000, 1_000_000_000)
    }
    
    /// Create a narrow range position
    pub fn narrow_position(market: Pubkey, owner: Keypair) -> PositionBuilder {
        PositionBuilder::new()
            .with_market(market)
            .with_owner(owner)
            .with_ticks(-100, 100)
            .with_liquidity(100_000_000)
    }
    
    /// Create a wide range position  
    pub fn wide_position(market: Pubkey, owner: Keypair) -> PositionBuilder {
        PositionBuilder::new()
            .with_market(market)
            .with_owner(owner)
            .with_ticks(-10000, 10000)
            .with_liquidity(1_000_000_000)
    }
    
    /// Create a small swap
    pub fn small_swap(market: Pubkey, user: Keypair) -> SwapTestBuilder {
        SwapTestBuilder::new()
            .with_market(market)
            .with_user(user)
            .with_amount(100_000)
    }
    
    /// Create a large swap that crosses multiple ticks
    pub fn large_swap(market: Pubkey, user: Keypair) -> SwapTestBuilder {
        SwapTestBuilder::new()
            .with_market(market)
            .with_user(user)
            .with_amount(100_000_000)
            .with_max_ticks(10)
    }
}