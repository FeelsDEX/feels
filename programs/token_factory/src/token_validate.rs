/// Token symbol validation utilities for preventing restricted token creation
///
/// Maintains a restricted list of token symbols that cannot be used when creating
/// fungible tokens on the platform. This prevents confusion with existing
/// major tokens and protocol-specific tokens.
use anchor_lang::prelude::*;

use crate::error::TokenFactoryError;

const MAX_DECIMALS: u8 = 18;
const MAX_NAME_LENGTH: usize = 32;
const MAX_SYMBOL_LENGTH: usize = 12;

/// List of restricted token symbols that cannot be used for token creation
///
/// This includes:
/// - Protocol tokens (FeelsSOL)
/// - Major blockchain tokens (SOL)  
/// - Major stablecoins (USDC, USDT)
/// - Other reserved names
pub const RESTRICTED_SYMBOLS: &[&str] = &[
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
    "UNI",    // Uniswap
    "AAVE",   // Aave
    "PENGU",  // Pengu token
    "GT",     // GateToken
    // Common reserved words
    "TOKEN", "COIN", "CRYPTO", "CURRENCY", "MONEY", "CASH", "DOLLAR", "EURO", "YEN", "POUND",
    "GOLD", "SILVER", "BITCOIN", "ETHEREUM", "SOLANA",
];

/// Check if a token symbol is restricted
///
/// Performs case-insensitive comparison with the restricted list
///
/// # Arguments
/// * `symbol` - The token symbol to check
///
/// # Returns
/// * `true` if the symbol is restricted and cannot be used
/// * `false` if the symbol is allowed
pub fn is_symbol_restricted(symbol: &str) -> bool {
    let symbol_upper = symbol.to_uppercase();
    RESTRICTED_SYMBOLS.contains(&symbol_upper.as_str())
}

/// Validate that a token symbol is not restricted
///
/// # Arguments
/// * `symbol` - The token symbol to validate
///
/// # Returns
/// * `Ok(())` if the symbol is allowed
/// * `Err(TokenFactoryError::SymbolIsRestricted)` if the symbol is restricted
pub fn validate_symbol_not_restricted(symbol: &str) -> Result<()> {
    if is_symbol_restricted(symbol) {
        return Err(error!(TokenFactoryError::SymbolIsRestricted));
    }
    Ok(())
}

/// Comprehensive validation for token symbol format and restrictions
///
/// Checks that the symbol meets basic requirements:
/// - Contains only alphanumeric characters
/// - Not restricted
///
/// # Arguments
/// * `symbol` - The token symbol to validate
///
/// # Returns
/// * `Ok(())` if the symbol is valid
/// * `Err(TokenFactoryError)` with appropriate error if validation fails
pub fn validate_symbol_format(symbol: &str) -> Result<()> {
    // Check length
    if symbol.is_empty() {
        return Err(error!(TokenFactoryError::SymbolIsEmpty));
    }
    require!(
        symbol.len() <= MAX_SYMBOL_LENGTH,
        TokenFactoryError::SymbolTooLong
    );

    // Check symbol is uppercase
    require!(
        symbol == symbol.to_uppercase(),
        TokenFactoryError::SymbolNotUppercase
    );

    // Check characters - only alphanumeric allowed
    if !symbol.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(error!(TokenFactoryError::SymbolNotAlphanumeric));
    }

    // Check restrictions
    validate_symbol_not_restricted(symbol)?;

    Ok(())
}

/// Validate token creation parameters
pub fn validate_token(symbol: &str, name: &str, decimals: u8) -> Result<()> {
    // Validate symbol
    validate_symbol_format(symbol)?;

    require!(
        decimals <= MAX_DECIMALS,
        TokenFactoryError::DecimalsTooLarge
    );

    require!(!name.is_empty(), TokenFactoryError::NameIsEmpty);

    require!(
        name.len() <= MAX_NAME_LENGTH,
        TokenFactoryError::NameTooLong
    );

    Ok(())
}

#[test]
fn test_restricted_symbols() {
    // Test protocol tokens
    assert!(is_symbol_restricted("FeelsSOL"));
    assert!(is_symbol_restricted("FEELSSOL"));
    assert!(is_symbol_restricted("feelssol"));
    assert!(is_symbol_restricted("Feels"));

    // Test major tokens
    assert!(is_symbol_restricted("SOL"));
    assert!(is_symbol_restricted("sol"));
    assert!(is_symbol_restricted("USDC"));
    assert!(is_symbol_restricted("usdc"));
    assert!(is_symbol_restricted("USDT"));
    assert!(is_symbol_restricted("usdt"));

    // Test case insensitivity
    assert!(is_symbol_restricted("Bitcoin"));
    assert!(is_symbol_restricted("ETHEREUM"));
    assert!(is_symbol_restricted("solana"));
}

#[test]
fn test_allowed_symbols() {
    // Test allowed symbols
    assert!(!is_symbol_restricted("MYTOKEN"));
    assert!(!is_symbol_restricted("TEST123"));
    assert!(!is_symbol_restricted("CUSTOM"));
    assert!(!is_symbol_restricted("NEWCOIN"));
    assert!(!is_symbol_restricted("DEFI"));
    assert!(!is_symbol_restricted("MOON"));
}

#[test]
fn test_symbol_validation() {
    // Test valid symbols
    assert!(validate_symbol_format("MYTOKEN").is_ok());
    assert!(validate_symbol_format("TEST123").is_ok());
    assert!(validate_symbol_format("A").is_ok());

    // Test invalid length
    assert!(validate_symbol_format("").is_err());

    // Test too long
    assert!(validate_symbol_format("AAAAAAAAAAAAAAAAAAAAAAAAA").is_err());

    // Test invalid characters
    assert!(validate_symbol_format("MY-TOKEN").is_err());
    assert!(validate_symbol_format("MY_TOKEN").is_err());
    assert!(validate_symbol_format("MY.TOKEN").is_err());
    assert!(validate_symbol_format("MY TOKEN").is_err());
    assert!(validate_symbol_format("MY@TOKEN").is_err());

    // Test restricted symbols
    assert!(validate_symbol_format("SOL").is_err());
    assert!(validate_symbol_format("USDC").is_err());
    assert!(validate_symbol_format("FeelsSOL").is_err());
}

#[test]
fn test_comprehensive_restriction_coverage() {
    // Test that major categories are covered
    assert!(is_symbol_restricted("FEELSSOL")); // Protocol
    assert!(is_symbol_restricted("SOL")); // Blockchain
    assert!(is_symbol_restricted("USDC")); // Stablecoin
    assert!(is_symbol_restricted("BTC")); // Major crypto
    assert!(is_symbol_restricted("ETH")); // Major crypto
    assert!(is_symbol_restricted("TOKEN")); // Reserved word
    assert!(is_symbol_restricted("MONEY")); // Reserved word
}
