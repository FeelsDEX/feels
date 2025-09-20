use crate::core::{FeeEstimate, SdkError, SdkResult};

/// Calculate fees for a swap based on amount and market parameters
pub fn calculate_swap_fees(
    amount_in: u64,
    base_fee_bps: u16,
    liquidity: u128,
    sqrt_price: u128,
    is_buy: bool,
) -> SdkResult<FeeEstimate> {
    // Calculate base fee
    let base_fee = amount_in
        .checked_mul(base_fee_bps as u64)
        .and_then(|v| v.checked_div(10000))
        .ok_or(SdkError::MathOverflow)?;

    // Calculate price impact (simplified)
    let impact_bps = calculate_price_impact(amount_in, liquidity, sqrt_price, is_buy)?;
    let impact_fee = amount_in
        .checked_mul(impact_bps as u64)
        .and_then(|v| v.checked_div(10000))
        .ok_or(SdkError::MathOverflow)?;

    let total_fee = base_fee.saturating_add(impact_fee);
    let fee_bps = ((total_fee as u128 * 10000) / amount_in as u128) as u16;

    Ok(FeeEstimate {
        base_fee,
        impact_fee,
        total_fee,
        fee_bps,
        price_impact_bps: impact_bps,
    })
}

/// Calculate price impact in basis points
fn calculate_price_impact(
    amount: u64,
    liquidity: u128,
    sqrt_price: u128,
    is_buy: bool,
) -> SdkResult<u16> {
    if liquidity == 0 {
        return Ok(10000); // 100% impact if no liquidity
    }

    // Simplified impact calculation
    let amount_128 = amount as u128;
    let impact_factor = if is_buy { 12000 } else { 8000 };

    let raw_impact = amount_128
        .saturating_mul(impact_factor)
        .saturating_mul(sqrt_price)
        .checked_div(liquidity.saturating_mul(10000))
        .unwrap_or(10000);

    Ok(raw_impact.min(10000) as u16)
}

/// Distribution of fees between protocol components
#[derive(Debug, Clone)]
pub struct FeeDistribution {
    pub buffer_share: u64,
    pub treasury_share: u64,
    pub creator_share: u64,
    pub lp_share: u64,
}

/// Calculate fee distribution based on protocol rules
pub fn calculate_fee_distribution(
    total_fee: u64,
    has_creator: bool,
    buffer_share_bps: u16,
    treasury_share_bps: u16,
    creator_share_bps: u16,
) -> SdkResult<FeeDistribution> {
    // Ensure shares don't exceed 100%
    let total_share = buffer_share_bps + treasury_share_bps + creator_share_bps;
    if total_share > 10000 {
        return Err(SdkError::InvalidParameters(
            "Fee shares exceed 100%".to_string(),
        ));
    }

    let buffer_share = (total_fee as u128 * buffer_share_bps as u128 / 10000) as u64;
    let treasury_share = (total_fee as u128 * treasury_share_bps as u128 / 10000) as u64;
    let creator_share = if has_creator {
        (total_fee as u128 * creator_share_bps as u128 / 10000) as u64
    } else {
        0
    };

    let lp_share = total_fee
        .saturating_sub(buffer_share)
        .saturating_sub(treasury_share)
        .saturating_sub(creator_share);

    Ok(FeeDistribution {
        buffer_share,
        treasury_share,
        creator_share,
        lp_share,
    })
}

/// Apply fee cap to ensure user protection
pub fn apply_fee_cap(fee: u64, amount: u64, max_fee_bps: u16) -> u64 {
    if max_fee_bps == 0 {
        return fee;
    }

    let max_fee = (amount as u128 * max_fee_bps as u128 / 10000) as u64;
    fee.min(max_fee)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_calculation() {
        let estimate = calculate_swap_fees(10000, 30, 1_000_000_000, 1_000_000, true).unwrap();
        assert_eq!(estimate.base_fee, 30); // 0.3% of 10000
        assert!(estimate.total_fee >= estimate.base_fee);
    }

    #[test]
    fn test_fee_distribution() {
        let dist = calculate_fee_distribution(1000, true, 2000, 1000, 500).unwrap();
        assert_eq!(dist.buffer_share, 200); // 20%
        assert_eq!(dist.treasury_share, 100); // 10%
        assert_eq!(dist.creator_share, 50); // 5%
        assert_eq!(dist.lp_share, 650); // 65%
        assert_eq!(
            dist.buffer_share + dist.treasury_share + dist.creator_share + dist.lp_share,
            1000
        );
    }
}