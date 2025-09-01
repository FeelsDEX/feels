/// Unified pool configuration instruction that consolidates all pool parameter updates.
/// This replaces multiple individual configuration instructions with a single, powerful
/// instruction that uses an enum to specify the configuration type.
use anchor_lang::prelude::*;
use crate::logic::event::{PoolConfigUpdatedEvent, HookRegisteredEvent, HookUnregisteredEvent};
use crate::logic::fee_manager::FeeManager;
use crate::state::{
    FeelsProtocolError, Pool, FeeConfig, LeverageParameters, ProtectionCurveType, 
    ProtectionCurveData, RiskProfile, DynamicFeeConfig, HookRegistry, HookPermission
};
use crate::{validate_pool_authority, validate_authority};

// ============================================================================
// Unified Configuration Parameters
// ============================================================================

/// Unified configuration parameters for all pool updates
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum PoolConfigParams {
    /// Configure leverage parameters
    Leverage(LeverageConfig),
    
    /// Configure dynamic fee parameters
    DynamicFees(DynamicFeeConfig),
    
    /// Update pool authority
    Authority(AuthorityConfig),
    
    /// Register or unregister hooks
    Hook(HookConfig),
    
    /// Update oracle configuration
    Oracle(OracleConfig),
    
    /// Update redenomination parameters
    Redenomination(RedenominationConfig),
    
    /// Batch multiple configurations
    Batch(Vec<PoolConfigParams>),
}

/// Leverage configuration parameters
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct LeverageConfig {
    /// Operation type
    pub operation: LeverageOperation,
    /// Maximum leverage allowed (6 decimals, e.g., 10_000_000 = 10x)
    pub max_leverage: Option<u64>,
    /// Current leverage ceiling
    pub current_ceiling: Option<u64>,
    /// Protection curve configuration
    pub protection_curve: Option<ProtectionCurveConfig>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum LeverageOperation {
    /// Enable leverage for the first time
    Enable,
    /// Update existing leverage parameters
    Update,
    /// Disable leverage (set max to 1x)
    Disable,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ProtectionCurveConfig {
    /// Curve type (0 = Linear, 1 = Exponential, 2 = Piecewise)
    pub curve_type: u8,
    /// Decay rate for exponential curve
    pub decay_rate: Option<u64>,
    /// Points for piecewise curve
    pub points: Option<[[u64; 2]; 8]>,
}

/// Authority configuration
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct AuthorityConfig {
    /// New authority pubkey
    pub new_authority: Pubkey,
    /// New fee authority (if different from main authority)
    pub new_fee_authority: Option<Pubkey>,
}

/// Hook configuration
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum HookConfig {
    /// Register a new hook
    Register {
        /// Hook program to register
        hook_program: Pubkey,
        /// Hook permission level
        permission: HookPermission,
        /// Events this hook is interested in
        event_mask: u32,
        /// Stages this hook runs in
        stage_mask: u32,
    },
    /// Unregister an existing hook
    Unregister {
        /// Index of hook to unregister
        hook_index: u8,
    },
    /// Enable or disable all hooks
    Toggle {
        /// Whether hooks should be enabled
        enabled: bool,
    },
}

/// Oracle configuration
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct OracleConfig {
    /// Oracle account to use (Pubkey::default() to disable)
    pub oracle: Pubkey,
    /// Oracle confidence threshold (basis points)
    pub confidence_threshold: u16,
    /// Maximum allowed price age (seconds)
    pub max_price_age: u32,
}

/// Redenomination configuration
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct RedenominationConfig {
    /// Redenomination threshold percentage (basis points)
    pub threshold_bps: u16,
    /// Minimum time between redenominations (seconds)
    pub min_interval: i64,
    /// Whether automatic redenomination is enabled
    pub auto_enabled: bool,
}

// ============================================================================
// Handler Function
// ============================================================================

/// Configure pool with unified parameters
pub fn handler(
    ctx: Context<crate::ConfigurePool>,
    params: PoolConfigParams,
) -> Result<()> {
    let authority = ctx.accounts.authority.key();
    
    // Process configuration based on type
    match params {
        PoolConfigParams::Leverage(config) => {
            configure_leverage(ctx, config)?;
        }
        PoolConfigParams::DynamicFees(config) => {
            configure_dynamic_fees(ctx, config)?;
        }
        PoolConfigParams::Authority(config) => {
            configure_authority(ctx, config)?;
        }
        PoolConfigParams::Hook(config) => {
            configure_hook(ctx, config)?;
        }
        PoolConfigParams::Oracle(config) => {
            configure_oracle(ctx, config)?;
        }
        PoolConfigParams::Redenomination(config) => {
            configure_redenomination(ctx, config)?;
        }
        PoolConfigParams::Batch(configs) => {
            // Process batch configurations
            for config in configs {
                // Recursive call with same context
                handler(ctx.clone(), config)?;
            }
        }
    }
    
    // Emit configuration event
    emit!(PoolConfigUpdatedEvent {
        pool: ctx.accounts.pool.key(),
        authority,
        config_type: format!("{:?}", params),
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    Ok(())
}

// ============================================================================
// Configuration Handlers
// ============================================================================

/// Configure leverage parameters
fn configure_leverage(
    ctx: Context<crate::ConfigurePool>,
    config: LeverageConfig,
) -> Result<()> {
    let mut pool = ctx.accounts.pool.load_mut()?;
    
    // Validate authority using centralized macro
    validate_pool_authority!(ctx.accounts.authority, ctx.accounts.pool);
    
    match config.operation {
        LeverageOperation::Enable => {
            // Validate required parameters
            let max_leverage = config.max_leverage
                .ok_or(FeelsProtocolError::InvalidLeverage)?;
            let current_ceiling = config.current_ceiling
                .unwrap_or(max_leverage); // Default to max if not specified
            
            require!(
                max_leverage >= 1_000_000 && max_leverage <= 100_000_000, // 1x to 100x
                FeelsProtocolError::InvalidLeverage
            );
            require!(
                current_ceiling >= 1_000_000 && current_ceiling <= max_leverage,
                FeelsProtocolError::InvalidLeverage
            );
            
            // Configure leverage parameters
            pool.leverage_params = LeverageParameters {
                max_leverage,
                current_ceiling,
                protection_curve_type: ProtectionCurveType {
                    curve_type: config.protection_curve
                        .as_ref()
                        .map(|pc| pc.curve_type)
                        .unwrap_or(0), // Linear by default
                    _padding: [0; 7],
                },
                protection_curve_data: configure_protection_curve_data(&config.protection_curve),
                last_ceiling_update: Clock::get()?.slot,
                _padding: [0; 8],
            };
            
            msg!("Leverage enabled: max={}x, ceiling={}x", 
                max_leverage / 1_000_000, current_ceiling / 1_000_000);
        }
        
        LeverageOperation::Update => {
            // Validate leverage is already enabled
            require!(
                pool.leverage_params.max_leverage > RiskProfile::LEVERAGE_SCALE,
                FeelsProtocolError::LeverageNotEnabled
            );
            
            // Update individual parameters if provided
            if let Some(max_leverage) = config.max_leverage {
                require!(
                    max_leverage >= 1_000_000 && max_leverage <= 100_000_000,
                    FeelsProtocolError::InvalidLeverage
                );
                pool.leverage_params.max_leverage = max_leverage;
            }
            
            if let Some(current_ceiling) = config.current_ceiling {
                require!(
                    current_ceiling >= 1_000_000 && 
                    current_ceiling <= pool.leverage_params.max_leverage,
                    FeelsProtocolError::InvalidLeverage
                );
                pool.leverage_params.current_ceiling = current_ceiling;
                pool.leverage_params.last_ceiling_update = Clock::get()?.slot;
            }
            
            if let Some(curve_config) = &config.protection_curve {
                pool.leverage_params.protection_curve_type.curve_type = curve_config.curve_type;
                pool.leverage_params.protection_curve_data = configure_protection_curve_data(&config.protection_curve);
            }
            
            msg!("Leverage updated");
        }
        
        LeverageOperation::Disable => {
            // Set max leverage to 1x (disabled)
            pool.leverage_params.max_leverage = RiskProfile::LEVERAGE_SCALE;
            pool.leverage_params.current_ceiling = RiskProfile::LEVERAGE_SCALE;
            msg!("Leverage disabled");
        }
    }
    
    Ok(())
}

/// Configure dynamic fees
fn configure_dynamic_fees(
    ctx: Context<crate::ConfigurePool>,
    config: DynamicFeeConfig,
) -> Result<()> {
    let pool = ctx.accounts.pool.load()?;
    
    // Validate authority using centralized macro
    validate_pool_authority!(ctx.accounts.authority, ctx.accounts.pool);
    
    // Validate fee parameters
    require!(
        config.min_fee <= config.base_fee && config.base_fee <= config.max_fee,
        FeelsProtocolError::InvalidFeeRate
    );
    require!(
        config.max_fee <= 1000, // Max 10%
        FeelsProtocolError::InvalidFeeRate
    );
    
    // Update dynamic fee configuration on FeeConfig account
    let fee_config = &mut ctx.accounts.fee_config;
    FeeManager::update_dynamic_fee_config(fee_config, config.clone())?;
    
    msg!("Dynamic fees updated: base={} bps, range={}-{} bps", 
        config.base_fee, config.min_fee, config.max_fee);
    
    Ok(())
}

/// Configure authority
fn configure_authority(
    ctx: Context<crate::ConfigurePool>,
    config: AuthorityConfig,
) -> Result<()> {
    let mut pool = ctx.accounts.pool.load_mut()?;
    
    // Validate current authority using centralized macro
    validate_pool_authority!(ctx.accounts.authority, ctx.accounts.pool);
    
    // Update authority
    let old_authority = pool.authority;
    pool.authority = config.new_authority;
    
    // Update fee authority if provided
    if let Some(new_fee_authority) = config.new_fee_authority {
        // Update on FeeConfig account
        let fee_config = &mut ctx.accounts.fee_config;
        fee_config.update_authority = new_fee_authority;
    }
    
    msg!("Authority updated: {} -> {}", old_authority, config.new_authority);
    
    Ok(())
}

/// Configure hooks
fn configure_hook(
    ctx: Context<crate::ConfigurePool>,
    config: HookConfig,
) -> Result<()> {
    let registry = ctx.accounts.hook_registry
        .as_mut()
        .ok_or(FeelsProtocolError::AccountNotFound)?;
    let clock = Clock::get()?;
    
    // Validate authority
    validate_authority!(ctx.accounts.authority, registry.authority);
    
    match config {
        HookConfig::Register { hook_program, permission, event_mask, stage_mask } => {
            // Validate parameters
            require!(
                hook_program != Pubkey::default(),
                FeelsProtocolError::InvalidPool
            );
            require!(
                event_mask > 0 && stage_mask > 0,
                FeelsProtocolError::InvalidAmount
            );
            
            // Register the hook
            let hook_index = registry.register_hook(
                hook_program,
                event_mask,
                stage_mask as u8,
                permission,
            )?;
            
            registry.last_update_timestamp = clock.unix_timestamp;
            
            emit!(HookRegisteredEvent {
                pool: registry.pool,
                hook_program,
                event_mask,
                stage_mask: stage_mask as u8,
                permission: permission as u8,
                index: hook_index as u8,
                timestamp: clock.unix_timestamp,
            });
            
            msg!("Hook registered: {} at index {}", hook_program, hook_index);
        }
        
        HookConfig::Unregister { hook_index } => {
            // Validate hook index
            require!(
                hook_index < registry.hook_count,
                FeelsProtocolError::InvalidAmount
            );
            
            // Get hook info before removal
            let hook = registry.hooks[hook_index as usize];
            
            // Remove hook
            let last_index = registry.hook_count - 1;
            if hook_index < last_index {
                registry.hooks[hook_index as usize] = registry.hooks[last_index as usize];
            }
            registry.hooks[last_index as usize] = Default::default();
            registry.hook_count -= 1;
            registry.last_update_timestamp = clock.unix_timestamp;
            
            emit!(HookUnregisteredEvent {
                pool: registry.pool,
                hook_program: hook.program_id,
                timestamp: clock.unix_timestamp,
            });
            
            msg!("Hook unregistered: {} from index {}", hook.program_id, hook_index);
        }
        
        HookConfig::Toggle { enabled } => {
            registry.hooks_enabled = enabled;
            registry.last_update_timestamp = clock.unix_timestamp;
            msg!("Hooks {}", if enabled { "enabled" } else { "disabled" });
        }
    }
    
    Ok(())
}

/// Configure oracle
fn configure_oracle(
    ctx: Context<crate::ConfigurePool>,
    config: OracleConfig,
) -> Result<()> {
    let mut pool = ctx.accounts.pool.load_mut()?;
    
    // Validate authority using centralized macro
    validate_pool_authority!(ctx.accounts.authority, ctx.accounts.pool);
    
    // Update oracle
    let old_oracle = pool.oracle;
    pool.oracle = config.oracle;
    
    // If oracle is being set (not disabled), validate the oracle account exists
    if config.oracle != Pubkey::default() {
        let oracle_account = ctx.accounts.oracle
            .as_ref()
            .ok_or(FeelsProtocolError::AccountNotFound)?;
        
        // Validate oracle account belongs to pool
        let oracle_data = oracle_account.try_borrow_data()?;
        // Basic validation - in production would deserialize and validate fully
        require!(
            oracle_data.len() >= 8,
            FeelsProtocolError::InvalidOracle
        );
    }
    
    msg!("Oracle updated: {} -> {}", old_oracle, config.oracle);
    
    Ok(())
}

/// Configure redenomination parameters
fn configure_redenomination(
    ctx: Context<crate::ConfigurePool>,
    config: RedenominationConfig,
) -> Result<()> {
    let mut pool = ctx.accounts.pool.load_mut()?;
    
    // Validate authority using centralized macro
    validate_pool_authority!(ctx.accounts.authority, ctx.accounts.pool);
    
    // Validate parameters
    require!(
        config.threshold_bps > 0 && config.threshold_bps <= 10000, // 0-100%
        FeelsProtocolError::InvalidAmount
    );
    require!(
        config.min_interval >= 3600, // At least 1 hour
        FeelsProtocolError::InvalidDuration
    );
    
    // Update redenomination threshold (convert basis points to raw value)
    pool.redenomination_threshold = (config.threshold_bps as u64) * 10_000;
    
    msg!("Redenomination configured: threshold={} bps, min_interval={} seconds", 
        config.threshold_bps, config.min_interval);
    
    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Configure protection curve data based on config
fn configure_protection_curve_data(config: &Option<ProtectionCurveConfig>) -> ProtectionCurveData {
    match config {
        Some(curve) => ProtectionCurveData {
            decay_rate: curve.decay_rate.unwrap_or(0),
            points: curve.points.unwrap_or([[0; 2]; 8]),
        },
        None => ProtectionCurveData {
            decay_rate: 0,
            points: [[0; 2]; 8],
        },
    }
}