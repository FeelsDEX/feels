//! Example of using anchor-client with the IDL
//! This shows how Anchor expects to interact with programs

use anchor_client::{
    solana_sdk::{
        instruction::Instruction,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        transaction::Transaction,
    },
    Client, Cluster, Program,
};
use std::rc::Rc;

pub fn example_usage() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client
    let payer = Rc::new(Keypair::new());
    let client = Client::new(Cluster::Devnet, payer.clone());
    
    // Load the program with IDL
    let program_id = "2FgA6YfdFNGgX8YyPKqSzhFGNvatRD5zi1yqCCFaSjq1".parse::<Pubkey>()?;
    let program = client.program(program_id)?;
    
    // Example: Call initialize_protocol
    let params = InitializeProtocolParams {
        mint_fee: 0,
        new_authority: None,
    };
    
    // Using anchor-client's request builder pattern
    let tx = program
        .request()
        .accounts(initialize_protocol_accounts {
            authority: payer.pubkey(),
            protocol_config: protocol_config_pda,
            system_program: solana_sdk::system_program::id(),
        })
        .args(initialize_protocol_instruction {
            params,
        })
        .send()?;
    
    println!("Transaction signature: {}", tx);
    
    // Example: Call mint_token
    let mint_params = MintTokenParams {
        ticker: "TEST".to_string(),
        name: "Test Token".to_string(),
        uri: "https://test.com".to_string(),
    };
    
    let token_mint = Keypair::new();
    let tx = program
        .request()
        .accounts(mint_token_accounts {
            creator: payer.pubkey(),
            token_mint: token_mint.pubkey(),
            escrow: escrow_pda,
            escrow_token_vault: escrow_token_vault_pda,
            escrow_feelssol_vault: escrow_feelssol_vault_pda,
            escrow_authority: escrow_authority_pda,
            metadata: metadata_pda,
            feelssol_mint: feelssol_mint,
            creator_feelssol: creator_feelssol_ata,
            protocol_config: protocol_config_pda,
            metadata_program: mpl_token_metadata::ID,
            protocol_token: protocol_token_pda,
            associated_token_program: spl_associated_token_account::id(),
            rent: solana_sdk::sysvar::rent::id(),
            token_program: spl_token::id(),
            system_program: solana_sdk::system_program::id(),
        })
        .args(mint_token_instruction {
            params: mint_params,
        })
        .signer(&token_mint)
        .send()?;
    
    Ok(())
}

// The anchor-client crate generates these structs from the IDL at runtime
// But we can also define them manually to match the IDL structure

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InitializeProtocolParams {
    pub mint_fee: u64,
    pub new_authority: Option<Pubkey>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct MintTokenParams {
    pub ticker: String,
    pub name: String,
    pub uri: String,
}

// Account structs that anchor-client expects
#[derive(Debug, Clone)]
pub struct initialize_protocol_accounts {
    pub authority: Pubkey,
    pub protocol_config: Pubkey,
    pub system_program: Pubkey,
}

#[derive(Debug, Clone)]
pub struct mint_token_accounts {
    pub creator: Pubkey,
    pub token_mint: Pubkey,
    pub escrow: Pubkey,
    pub escrow_token_vault: Pubkey,
    pub escrow_feelssol_vault: Pubkey,
    pub escrow_authority: Pubkey,
    pub metadata: Pubkey,
    pub feelssol_mint: Pubkey,
    pub creator_feelssol: Pubkey,
    pub protocol_config: Pubkey,
    pub metadata_program: Pubkey,
    pub protocol_token: Pubkey,
    pub associated_token_program: Pubkey,
    pub rent: Pubkey,
    pub token_program: Pubkey,
    pub system_program: Pubkey,
}

// Instruction structs
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct initialize_protocol_instruction {
    pub params: InitializeProtocolParams,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct mint_token_instruction {
    pub params: MintTokenParams,
}