#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::hook::validate_hook_accounts;
    use crate::state::PoolError;
    use anchor_lang::prelude::*;
    use std::mem;

    #[test]
    fn test_v9_2_hook_account_validation_rejects_system_accounts() {
        let system_program_id = anchor_lang::solana_program::system_program::id();
        let rent_sysvar_id = anchor_lang::solana_program::sysvar::rent::id();
        let clock_sysvar_id = anchor_lang::solana_program::sysvar::clock::id();
        
        // Create mock account info for system program
        let mut lamports = 0;
        let mut data = vec![];
        let system_account = AccountInfo::new(
            &system_program_id,
            false,
            false,
            &mut lamports,
            &mut data,
            &system_program_id,
            false,
            0,
        );
        
        // Should reject system program access
        let result = validate_hook_accounts(&[system_account]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string().contains("InvalidAuthority"), true);
        
        // Test rent sysvar
        let mut lamports2 = 0;
        let mut data2 = vec![];
        let rent_account = AccountInfo::new(
            &rent_sysvar_id,
            false,
            false,
            &mut lamports2,
            &mut data2,
            &system_program_id,
            false,
            0,
        );
        
        let result = validate_hook_accounts(&[rent_account]);
        assert!(result.is_err());
        
        // Test clock sysvar
        let mut lamports3 = 0;
        let mut data3 = vec![];
        let clock_account = AccountInfo::new(
            &clock_sysvar_id,
            false,
            false,
            &mut lamports3,
            &mut data3,
            &system_program_id,
            false,
            0,
        );
        
        let result = validate_hook_accounts(&[clock_account]);
        assert!(result.is_err());
    }

    #[test]
    fn test_v9_2_hook_account_validation_allows_valid_owners() {
        let token_program_id = anchor_spl::token::ID;
        let system_program_id = anchor_lang::solana_program::system_program::id();
        let our_program_id = crate::ID;
        let test_account_key = Pubkey::new_unique();
        
        // Test token program owned account (valid)
        let mut lamports = 0;
        let mut data = vec![];
        let token_account = AccountInfo::new(
            &test_account_key,
            false,
            false,
            &mut lamports,
            &mut data,
            &token_program_id,
            false,
            0,
        );
        
        let result = validate_hook_accounts(&[token_account]);
        assert!(result.is_ok());
        
        // Test our program owned account (valid)
        let mut lamports2 = 0;
        let mut data2 = vec![];
        let our_account = AccountInfo::new(
            &test_account_key,
            false,
            false,
            &mut lamports2,
            &mut data2,
            &our_program_id,
            false,
            0,
        );
        
        let result = validate_hook_accounts(&[our_account]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_v9_2_hook_account_validation_rejects_invalid_owner() {
        let invalid_program_id = Pubkey::new_unique();
        let test_account_key = Pubkey::new_unique();
        
        // Test account owned by unknown program (invalid)
        let mut lamports = 0;
        let mut data = vec![];
        let invalid_account = AccountInfo::new(
            &test_account_key,
            false,
            false,
            &mut lamports,
            &mut data,
            &invalid_program_id,
            false,
            0,
        );
        
        let result = validate_hook_accounts(&[invalid_account]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string().contains("InvalidHookProgram"), true);
    }
}