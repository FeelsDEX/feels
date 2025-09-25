use crate::prelude::*;
use solana_sdk::instruction::{AccountMeta, Instruction};

use crate::core::{program_id, SdkError, SdkResult};

/// Trait for building instructions with consistent patterns
pub trait InstructionBuilder: AnchorSerialize {
    /// The 8-byte instruction discriminator
    const DISCRIMINATOR: [u8; 8];

    /// Build the instruction data (discriminator + serialized params)
    fn build_data(&self) -> SdkResult<Vec<u8>> {
        let mut data = Self::DISCRIMINATOR.to_vec();
        data.extend_from_slice(
            &self
                .try_to_vec()
                .map_err(|e| SdkError::SerializationError(e.to_string()))?,
        );
        Ok(data)
    }
}

/// Builder for constructing Solana instructions
pub struct FeelsInstructionBuilder {
    accounts: Vec<AccountMeta>,
    data: Vec<u8>,
}

impl FeelsInstructionBuilder {
    pub fn new() -> Self {
        Self {
            accounts: Vec::new(),
            data: Vec::new(),
        }
    }

    /// Add a writable signer account
    pub fn add_signer(mut self, pubkey: Pubkey) -> Self {
        self.accounts.push(AccountMeta::new(pubkey, true));
        self
    }

    /// Add a writable non-signer account
    pub fn add_writable(mut self, pubkey: Pubkey) -> Self {
        self.accounts.push(AccountMeta::new(pubkey, false));
        self
    }

    /// Add a readonly account
    pub fn add_readonly(mut self, pubkey: Pubkey) -> Self {
        self.accounts.push(AccountMeta::new_readonly(pubkey, false));
        self
    }

    /// Add an optional account (uses default pubkey if None)
    pub fn add_optional(mut self, pubkey: Option<Pubkey>) -> Self {
        match pubkey {
            Some(key) => self.accounts.push(AccountMeta::new_readonly(key, false)),
            None => self
                .accounts
                .push(AccountMeta::new_readonly(Pubkey::default(), false)),
        }
        self
    }

    /// Add multiple accounts
    pub fn add_accounts(mut self, accounts: Vec<AccountMeta>) -> Self {
        self.accounts.extend(accounts);
        self
    }

    /// Set the instruction data
    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    /// Build the final instruction
    pub fn build(self) -> Instruction {
        Instruction {
            program_id: program_id(),
            accounts: self.accounts,
            data: self.data,
        }
    }
}

impl Default for FeelsInstructionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro for implementing InstructionBuilder for a params struct
#[macro_export]
macro_rules! impl_instruction {
    ($name:ident, $discriminator:expr) => {
        impl $crate::instructions::InstructionBuilder for $name {
            const DISCRIMINATOR: [u8; 8] = $discriminator;
        }
    };
}