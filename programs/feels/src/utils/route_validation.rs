//! Route validation for hub-and-spoke model
//! 
//! Enforces ≤2 hop routes through FeelsSOL hub

use anchor_lang::prelude::*;
use crate::error::FeelsError;

/// Represents a trading route
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Route {
    /// Direct swap with FeelsSOL (1 hop)
    Direct {
        from: Pubkey,
        to: Pubkey,
    },
    /// Two-hop swap through FeelsSOL hub
    TwoHop {
        from: Pubkey,
        intermediate: Pubkey, // Must be FeelsSOL
        to: Pubkey,
    },
}

impl Route {
    /// Create a route from token 0 to token 1
    pub fn new(token_0: Pubkey, token_1: Pubkey, feelssol_mint: Pubkey) -> Result<Self> {
        // Cannot swap token to itself
        require!(token_0 != token_1, FeelsError::InvalidRoute);
        
        // Check if either token is FeelsSOL
        if token_0 == feelssol_mint || token_1 == feelssol_mint {
            // Direct route when one token is FeelsSOL
            Ok(Route::Direct {
                from: token_0,
                to: token_1,
            })
        } else {
            // Two-hop route through FeelsSOL for non-FeelsSOL pairs
            Ok(Route::TwoHop {
                from: token_0,
                intermediate: feelssol_mint,
                to: token_1,
            })
        }
    }
    
    /// Validate that a route follows hub-and-spoke rules
    pub fn validate(&self, feelssol_mint: Pubkey) -> Result<()> {
        match self {
            Route::Direct { from, to } => {
                // At least one token must be FeelsSOL for direct routes
                require!(
                    *from == feelssol_mint || *to == feelssol_mint,
                    FeelsError::InvalidRoute
                );
            }
            Route::TwoHop { from, intermediate, to } => {
                // Intermediate must be FeelsSOL
                require!(*intermediate == feelssol_mint, FeelsError::InvalidRoute);
                
                // Neither from nor to should be FeelsSOL (otherwise use direct)
                require!(
                    *from != feelssol_mint && *to != feelssol_mint,
                    FeelsError::InvalidRoute
                );
                
                // No duplicate tokens
                require!(*from != *to, FeelsError::InvalidRoute);
            }
        }
        Ok(())
    }
    
    /// Get the number of hops in this route
    pub fn hop_count(&self) -> u8 {
        match self {
            Route::Direct { .. } => 1,
            Route::TwoHop { .. } => 2,
        }
    }
    
    /// Check if this route involves a specific token
    pub fn includes_token(&self, token: &Pubkey) -> bool {
        match self {
            Route::Direct { from, to } => from == token || to == token,
            Route::TwoHop { from, intermediate, to } => {
                from == token || intermediate == token || to == token
            }
        }
    }
}

/// Validate a swap follows hub-and-spoke routing rules
pub fn validate_swap_route(
    token_in: Pubkey,
    token_out: Pubkey,
    feelssol_mint: Pubkey,
) -> Result<Route> {
    let route = Route::new(token_in, token_out, feelssol_mint)?;
    route.validate(feelssol_mint)?;
    
    // Ensure we don't exceed 2 hops
    require!(route.hop_count() <= 2, FeelsError::RouteTooLong);
    
    Ok(route)
}
