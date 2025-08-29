/// Token ticker validation utilities for preventing restricted token creation
///
/// Maintains a restricted list of token tickers that cannot be used when creating
/// fungible tokens on the platform. This prevents confusion with existing
/// major tokens and protocol-specific tokens.
use anchor_lang::prelude::*;

/// List of restricted token tickers that cannot be used for token creation
///
/// This includes:
/// - Protocol tokens (FeelsSOL)
/// - Major blockchain tokens (SOL)  
/// - Major stablecoins (USDC, USDT)
/// - Other reserved names
pub const RESTRICTED_TICKERS: &[&str] = &[
    // Protocol tokens
    "FEELSSOL", "FEELS", // Major blockchain tokens
    "SOL", "WSOL", "MSOL", "STSOL", "JSOL", "BSOL", // Major stablecoins
    "USDC", "USDT", "USDS", "DAI", "FRAX", "LUSD", "BUSD", // Other major tokens
    "ETH", "WETH", "BTC", "WBTC",
    // Top Solana ecosystem tokens
    "JUP",    // Jupiter - dominant DEX
    "PYTH",   // Pyth Network - oracle
    "RENDER", // Render Network
    "BONK",   // Bonk meme token
    "RAY",    // Raydium DEX
    "ORCA",   // Orca DEX
    "SRM",    // Serum DEX
    "WIF",    // Dogwifhat meme token
    "ATLAS",  // Star Atlas
    "HNT",    // Helium Network
    "SLND",   // Solend
    "INJ",    // Injective
    "TRUMP",  // Trump meme token
    "LINK",   // Chainlink
    "PENGU",  // Pengu token
    "GT",     // GateToken
    // Common reserved words
    "TOKEN", "COIN", "CRYPTO", "CURRENCY", "MONEY", "CASH", "DOLLAR", "EURO", "YEN", "POUND",
    "GOLD", "SILVER", "BITCOIN", "ETHEREUM", "SOLANA",
];

/// Check if a token ticker is restricted
///
/// Performs case-insensitive comparison with the restricted list
///
/// # Arguments
/// * `ticker` - The token ticker to check
///
/// # Returns
/// * `true` if the ticker is restricted and cannot be used
/// * `false` if the ticker is allowed
pub fn is_ticker_restricted(ticker: &str) -> bool {
    let ticker_upper = ticker.to_uppercase();
    RESTRICTED_TICKERS.contains(&ticker_upper.as_str())
}

/// Validate that a token ticker is not restricted
///
/// # Arguments
/// * `ticker` - The token ticker to validate
///
/// # Returns
/// * `Ok(())` if the ticker is allowed
/// * `Err(FeelsProtocolError::RestrictedTicker)` if the ticker is restricted
pub fn validate_ticker_not_restricted(ticker: &str) -> Result<()> {
    if is_ticker_restricted(ticker) {
        return Err(error!(crate::state::FeelsProtocolError::RestrictedTicker));
    }
    Ok(())
}

/// Comprehensive validation for token ticker format and restrictions
///
/// Checks that the ticker meets basic requirements:
/// - Length between 1 and 12 characters
/// - Contains only alphanumeric characters
/// - Not restricted
///
/// # Arguments
/// * `ticker` - The token ticker to validate
///
/// # Returns
/// * `Ok(())` if the ticker is valid
/// * `Err(PoolError)` with appropriate error if validation fails
pub fn validate_ticker_format(ticker: &str) -> Result<()> {
    // Check length
    if ticker.is_empty() {
        return Err(error!(crate::state::FeelsProtocolError::InvalidTickerLength));
    }

    if ticker.len() > 12 {
        return Err(error!(crate::state::FeelsProtocolError::InvalidTickerLength));
    }

    // Check characters - only alphanumeric allowed
    if !ticker.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(error!(crate::state::FeelsProtocolError::InvalidTickerFormat));
    }

    // Check restrictions
    validate_ticker_not_restricted(ticker)?;

    Ok(())
}

/// Get a list of suggested alternative tickers when a restricted one is used
///
/// # Arguments
/// * `attempted_ticker` - The restricted ticker that was attempted
///
/// # Returns
/// * Vector of suggested alternative ticker names
pub fn get_ticker_alternatives(attempted_ticker: &str) -> Vec<String> {
    let base = attempted_ticker.to_uppercase();

    vec![
        format!("{}2", base),
        format!("{}V2", base),
        format!("NEW{}", base),
        format!("{}TOKEN", base),
        format!("{}COIN", base),
        format!("WRAP{}", base),
        format!("{}PROTOCOL", base),
        format!("{}DAO", base),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restricted_tickers() {
        // Test protocol tokens
        assert!(is_ticker_restricted("FeelsSOL"));
        assert!(is_ticker_restricted("FEELSSOL"));
        assert!(is_ticker_restricted("feelssol"));
        assert!(is_ticker_restricted("Feels"));

        // Test major tokens
        assert!(is_ticker_restricted("SOL"));
        assert!(is_ticker_restricted("sol"));
        assert!(is_ticker_restricted("USDC"));
        assert!(is_ticker_restricted("usdc"));
        assert!(is_ticker_restricted("USDT"));
        assert!(is_ticker_restricted("usdt"));

        // Test case insensitivity
        assert!(is_ticker_restricted("Bitcoin"));
        assert!(is_ticker_restricted("ETHEREUM"));
        assert!(is_ticker_restricted("solana"));
    }

    #[test]
    fn test_allowed_tickers() {
        // Test allowed tickers
        assert!(!is_ticker_restricted("MYTOKEN"));
        assert!(!is_ticker_restricted("TEST123"));
        assert!(!is_ticker_restricted("CUSTOM"));
        assert!(!is_ticker_restricted("NEWCOIN"));
        assert!(!is_ticker_restricted("DEFI"));
        assert!(!is_ticker_restricted("MOON"));
    }

    #[test]
    fn test_ticker_validation() {
        // Test valid tickers
        assert!(validate_ticker_format("MYTOKEN").is_ok());
        assert!(validate_ticker_format("TEST123").is_ok());
        assert!(validate_ticker_format("A").is_ok());
        assert!(validate_ticker_format("ABCDEFGHIJKL").is_ok()); // 12 chars

        // Test invalid length
        assert!(validate_ticker_format("").is_err());
        assert!(validate_ticker_format("ABCDEFGHIJKLM").is_err()); // 13 chars

        // Test invalid characters
        assert!(validate_ticker_format("MY-TOKEN").is_err());
        assert!(validate_ticker_format("MY_TOKEN").is_err());
        assert!(validate_ticker_format("MY.TOKEN").is_err());
        assert!(validate_ticker_format("MY TOKEN").is_err());
        assert!(validate_ticker_format("MY@TOKEN").is_err());

        // Test restricted tickers
        assert!(validate_ticker_format("SOL").is_err());
        assert!(validate_ticker_format("USDC").is_err());
        assert!(validate_ticker_format("FeelsSOL").is_err());
    }

    #[test]
    fn test_ticker_alternatives() {
        let alternatives = get_ticker_alternatives("SOL");

        assert!(alternatives.contains(&"SOL2".to_string()));
        assert!(alternatives.contains(&"SOLV2".to_string()));
        assert!(alternatives.contains(&"NEWSOL".to_string()));
        assert!(alternatives.contains(&"SOLTOKEN".to_string()));
        assert!(alternatives.contains(&"SOLCOIN".to_string()));
        assert!(alternatives.contains(&"WRAPSOL".to_string()));
        assert!(alternatives.contains(&"SOLPROTOCOL".to_string()));
        assert!(alternatives.contains(&"SOLDAO".to_string()));
    }

    #[test]
    fn test_comprehensive_restriction_coverage() {
        // Test that major categories are covered
        assert!(is_ticker_restricted("FEELSSOL")); // Protocol
        assert!(is_ticker_restricted("SOL")); // Blockchain
        assert!(is_ticker_restricted("USDC")); // Stablecoin
        assert!(is_ticker_restricted("BTC")); // Major crypto
        assert!(is_ticker_restricted("ETH")); // Major crypto
        assert!(is_ticker_restricted("TOKEN")); // Reserved word
        assert!(is_ticker_restricted("MONEY")); // Reserved word
    }
}
