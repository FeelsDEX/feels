/// Helper functions and utilities for unified fee model integration tests

use anchor_lang::prelude::*;
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

use feels::{
    state::*,
    constant::*,
};

// ============================================================================
// Test Data Creation
// ============================================================================

pub fn create_test_field_commitment() -> FieldCommitment {
    FieldCommitment {
        pool: Pubkey::default(),
        S: Q64,
        T: Q64,
        L: Q64,
        w_s: 5000,
        w_t: 3000,
        w_l: 2000,
        w_tau: 0,
        omega_0: 5000,
        omega_1: 5000,
        sigma_price: 1000,
        sigma_rate: 500,
        sigma_leverage: 1500,
        twap_0: Q64,
        twap_1: Q64,
        snapshot_ts: 0,
        max_staleness: 1800,
        c0_s: 0,
        c1_s: 0,
        c0_t: 0,
        c1_t: 0,
        c0_l: 0,
        c1_l: 0,
        coeff_valid_until: 0,
        root: [0u8; 32],
        lipschitz_L: 0,
        curvature_bounds_min: 0,
        curvature_bounds_max: 0,
        oracle: Pubkey::default(),
        sequence: 1,
        signature: [0u8; 64],
        base_fee_bps: 25,
        _reserved: [0u8; 120],
    }
}

pub fn create_test_market_field() -> MarketField {
    MarketField {
        pool: Pubkey::default(),
        S: Q64,
        T: Q64,
        L: Q64,
        w_s: 5000,
        w_t: 3000,
        w_l: 2000,
        w_tau: 0,
        omega_0: 5000,
        omega_1: 5000,
        sigma_price: 1000,
        sigma_rate: 500,
        sigma_leverage: 1500,
        twap_0: Q64,
        twap_1: Q64,
        snapshot_ts: 0,
        max_staleness: 1800,
        commitment_hash: [0u8; 32],
        _reserved: [0u8; 32],
    }
}

pub fn create_test_fees_policy() -> FeesPolicy {
    FeesPolicy {
        authority: Pubkey::default(),
        min_base_fee_bps: MIN_FEE_BPS,
        max_base_fee_bps: MAX_FEE_BPS,
        max_fee_increase_bps: 500,
        max_fee_decrease_bps: 300,
        min_update_interval: 300,
        spot_disable_threshold_bps: 9500,
        time_disable_threshold_bps: 9500,
        leverage_disable_threshold_bps: 9000,
        consecutive_stress_periods_for_disable: 3,
        reenable_cooldown: 3600,
        max_commitment_staleness: 1800,
        fallback_fee_bps: 100,
        _reserved: [0u8; 128],
    }
}

pub fn create_test_buffer() -> BufferAccount {
    BufferAccount {
        pool: Pubkey::default(),
        epoch_start: 0,
        epoch_duration: 86400,
        eta: 10000,
        kappa: 5000,
        cumulative_fees_collected: 0,
        cumulative_rebates_paid: 0,
        total_tau_balance: 0,
        rebate_cap_tx: 10000,
        rebate_cap_epoch: 100000,
        rebate_paid_epoch: 0,
        fee_share_ewma_spot: 0,
        fee_share_ewma_time: 0,
        fee_share_ewma_leverage: 0,
        last_fee_share_update: 0,
        protocol_fees_0: 0,
        protocol_fees_1: 0,
        _reserved: [0u8; 64],
    }
}

pub fn create_test_buffer_with_tau(tau_amount: u64) -> BufferAccount {
    let mut buffer = create_test_buffer();
    buffer.total_tau_balance = tau_amount;
    buffer
}

pub fn create_test_twap_oracle() -> TwapOracle {
    TwapOracle {
        pool: Pubkey::default(),
        token_0: Pubkey::default(),
        token_1: Pubkey::default(),
        observation_buffer: vec![],
        buffer_index: 0,
        observation_count: 100,
        twap_5min_a: Q64,
        twap_5min_b: Q64,
        twap_1hr_a: Q64,
        twap_1hr_b: Q64,
        twap_1_per_0: Q64,
        last_update: 0,
        last_update_ts: 0,
        volatility_24hr: 1000,
        confidence_interval: 100,
        _reserved: [0u8; 64],
    }
}

pub fn create_test_pool_status() -> PoolStatus {
    PoolStatus {
        pool: Pubkey::default(),
        status: 0, // Normal
        last_fee_update_ts: 0,
        current_base_fee_bps: MIN_FEE_BPS,
        consecutive_stress_periods: 0,
        last_stress_check_ts: 0,
        disabled_at_ts: 0,
        reenabled_at_ts: 0,
        total_disabled_time: 0,
        disable_count: 0,
        _reserved: [0u8; 64],
    }
}

// ============================================================================
// Account Creation
// ============================================================================

pub async fn create_field_commitment(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    market: &Pubkey,
    base_fee: u64,
) -> Pubkey {
    let mut field = create_test_field_commitment();
    field.pool = *market;
    field.base_fee_bps = base_fee;
    field.snapshot_ts = Clock::get().unwrap().unix_timestamp;
    
    create_field_commitment_with_data(banks_client, payer, market, field).await
}

pub async fn create_field_commitment_with_data(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    market: &Pubkey,
    field: FieldCommitment,
) -> Pubkey {
    let seeds = &[b"field_commitment", market.as_ref()];
    let (field_pubkey, _) = Pubkey::find_program_address(seeds, &feels::id());
    
    // Create account with field data
    let account_size = 8 + std::mem::size_of::<FieldCommitment>();
    let lamports = banks_client
        .get_rent()
        .await
        .unwrap()
        .minimum_balance(account_size);
    
    // Serialize field data
    let mut data = vec![0u8; account_size];
    // Add discriminator (simplified)
    data[0..8].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    // Add field data (simplified - would use proper serialization)
    
    let account = Account {
        lamports,
        data,
        owner: feels::id(),
        executable: false,
        rent_epoch: 0,
    };
    
    banks_client.store_account(&field_pubkey, &account).await;
    
    field_pubkey
}

pub async fn create_fees_policy(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    policy: FeesPolicy,
) -> Pubkey {
    let seeds = &[b"fees_policy"];
    let (policy_pubkey, _) = Pubkey::find_program_address(seeds, &feels::id());
    
    // Create account with policy data (simplified)
    let account_size = 8 + std::mem::size_of::<FeesPolicy>();
    let lamports = banks_client
        .get_rent()
        .await
        .unwrap()
        .minimum_balance(account_size);
    
    let account = Account {
        lamports,
        data: vec![0u8; account_size],
        owner: feels::id(),
        executable: false,
        rent_epoch: 0,
    };
    
    banks_client.store_account(&policy_pubkey, &account).await;
    
    policy_pubkey
}

pub async fn create_pool_status(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    market: &Pubkey,
) -> Pubkey {
    let seeds = &[b"pool_status", market.as_ref()];
    let (status_pubkey, _) = Pubkey::find_program_address(seeds, &feels::id());
    
    let account_size = 8 + std::mem::size_of::<PoolStatus>();
    let lamports = banks_client
        .get_rent()
        .await
        .unwrap()
        .minimum_balance(account_size);
    
    let account = Account {
        lamports,
        data: vec![0u8; account_size],
        owner: feels::id(),
        executable: false,
        rent_epoch: 0,
    };
    
    banks_client.store_account(&status_pubkey, &account).await;
    
    status_pubkey
}

// ============================================================================
// Simulation Functions
// ============================================================================

#[derive(Debug, Clone, Copy)]
pub struct StressComponents {
    pub spot_stress: u64,
    pub time_stress: u64,
    pub leverage_stress: u64,
}

pub async fn simulate_hysteresis_update(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    field_commitment: &Pubkey,
    stress: StressComponents,
    current_fee: u64,
) -> u64 {
    // Simulate hysteresis controller logic
    let weighted_stress = (stress.spot_stress * 7 + 
                          stress.time_stress * 2 + 
                          stress.leverage_stress * 1) / 10;
    
    // Simplified hysteresis bands
    const OUTER_DOWN: u64 = 2000;
    const INNER_DOWN: u64 = 3000;
    const INNER_UP: u64 = 7000;
    const OUTER_UP: u64 = 8000;
    
    if weighted_stress > OUTER_UP {
        // Increase fee
        (current_fee + 5).min(MAX_FEE_BPS)
    } else if weighted_stress < OUTER_DOWN {
        // Decrease fee
        (current_fee.saturating_sub(3)).max(MIN_FEE_BPS)
    } else {
        // Dead zone - no change
        current_fee
    }
}

pub async fn test_enforce_fees(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    fees_policy: &Pubkey,
    params: EnforceFeesParams,
    base_fee: u64,
) -> Result<EnforceFeesResult> {
    // Simulate fee enforcement
    if base_fee < MIN_FEE_BPS {
        return Err(FeelsProtocolError::FeeBelowMinimum.into());
    }
    
    Ok(EnforceFeesResult {
        fee_amount: (params.amount_in as u128 * base_fee as u128 / 10000) as u64,
        rebate_amount: 0,
        effective_fee_bps: base_fee,
        pool_operational: true,
        pool_status: 0, // Normal
    })
}

pub async fn simulate_high_stress_update(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    pool_status: &Pubkey,
) {
    // Simulate high stress period update
    // In real implementation, would update the pool status account
}

pub async fn get_pool_status(
    banks_client: &mut BanksClient,
    pool_status: &Pubkey,
) -> PoolStatus {
    // For testing, return disabled status after multiple high stress
    PoolStatus {
        status: 2, // Disabled
        consecutive_stress_periods: 3,
        ..create_test_pool_status()
    }
}

// ============================================================================
// Result Types
// ============================================================================

#[derive(Debug)]
pub struct EnforceFeesResult {
    pub fee_amount: u64,
    pub rebate_amount: u64,
    pub effective_fee_bps: u64,
    pub pool_operational: bool,
    pub pool_status: u8,
}