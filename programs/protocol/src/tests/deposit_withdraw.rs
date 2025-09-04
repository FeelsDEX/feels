use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        signature::{read_keypair_file, Keypair},
        signer::{SeedDerivable, Signer as _},
        system_instruction, system_program, sysvar,
        transaction::Transaction,
    },
    Client, Cluster,
};
use anchor_lang::prelude::*;

use anchor_spl::{
    associated_token::{
        self, get_associated_token_address, get_associated_token_address_with_program_id,
        spl_associated_token_account,
    },
    token::spl_token,
    token_2022::spl_token_2022::{self},
};

use ::borsh::BorshDeserialize;
use feels_test_utils::{
    constants::{
        FEELSSOL_CONTROLLER_KEYPAIR_PATH, FEELSSOL_PDA_SEED, FEELS_PRIVATE_KEY, JITOSOL_MINT,
        JITO_STAKE_POOL, KEEPER_KEYPAIR_PATH, KEEPER_PDA_SEED, PROTOCOL_KEYPAIR_PATH,
        PROTOCOL_PDA_SEED, TEST_KEYPAIR_PATH, TREASURY_PDA_SEED, VAULT_PDA_SEED,
    },
    helpers::{get_token2022_balance, get_token_balance},
};
use feelssol_controller::{error::FeelsSolError, state::FeelsSolController};
use spl_stake_pool::{instruction::deposit_sol, state::StakePool};

use crate::accounts;

struct TestComponents {
    payer: Keypair,
    user: Keypair,
    protocol_program_id: Pubkey,
    feelssol_controller_program_id: Pubkey,
    feels_keeper_program_id: Pubkey,
}

fn setup_test_components() -> TestComponents {
    let current_dir = std::env::current_dir().unwrap();

    // Wallet keypair path
    let wallet_path = current_dir.join(TEST_KEYPAIR_PATH);
    let payer = read_keypair_file(&wallet_path).unwrap();

    // Read program IDs from keypair files
    let protocol_program_keypair_path = current_dir.join(PROTOCOL_KEYPAIR_PATH);
    let protocol_program_keypair = read_keypair_file(&protocol_program_keypair_path)
        .expect("Protocol Program keypair should exist");
    let protocol_program_id = protocol_program_keypair.pubkey();

    let feelssol_controller_program_keypair_path =
        current_dir.join(FEELSSOL_CONTROLLER_KEYPAIR_PATH);
    let feelssol_controller_program_keypair =
        read_keypair_file(&feelssol_controller_program_keypair_path)
            .expect("Feelssol controller Program keypair should exist");
    let feelssol_controller_program_id = feelssol_controller_program_keypair.pubkey();

    let feels_keeper_program_keypair_path = current_dir.join(KEEPER_KEYPAIR_PATH);
    let feels_keeper_program_keypair = read_keypair_file(&feels_keeper_program_keypair_path)
        .expect("Feels Keeper Program keypair should exist");
    let feels_keeper_program_id = feels_keeper_program_keypair.pubkey();

    // Create a test user that we will fund with enough sol
    let user = Keypair::new();

    let transfer_amount = 50_000_000_000; // 50 SOL
    let tx = system_instruction::transfer(&payer.pubkey(), &user.pubkey(), transfer_amount);

    let client = Client::new_with_options(Cluster::Localnet, &payer, CommitmentConfig::confirmed());
    let program = client.program(protocol_program_id).unwrap();
    let recent_blockhash = program.rpc().get_latest_blockhash().unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[tx],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    program.rpc().send_and_confirm_transaction(&tx).unwrap();

    TestComponents {
        payer,
        user,
        protocol_program_id,
        feelssol_controller_program_id,
        feels_keeper_program_id,
    }
}

impl TestComponents {
    fn client(&self) -> Client<&Keypair> {
        Client::new_with_options(
            Cluster::Localnet,
            &self.payer,
            CommitmentConfig::confirmed(),
        )
    }

    fn user_pubkey(&self) -> Pubkey {
        self.user.pubkey()
    }

    fn programs(
        &self,
    ) -> (
        anchor_client::Program<&Keypair>,
        anchor_client::Program<&Keypair>,
        anchor_client::Program<&Keypair>,
    ) {
        let client = self.client();
        (
            client.program(self.protocol_program_id).unwrap(),
            client.program(self.feelssol_controller_program_id).unwrap(),
            client.program(self.feels_keeper_program_id).unwrap(),
        )
    }
}

fn deploy_protocol_and_controller_on_test_validator(
    protocol: &anchor_client::Program<&Keypair>,
    feelssol_controller: &anchor_client::Program<&Keypair>,
    feels_keeper: &anchor_client::Program<&Keypair>,
) -> (Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey) {
    let (protocol_pda, _) = Pubkey::find_program_address(&[PROTOCOL_PDA_SEED], &crate::id());
    let (treasury_pda, _) = Pubkey::find_program_address(&[TREASURY_PDA_SEED], &crate::id());
    let (feelssol_pda, _) =
        Pubkey::find_program_address(&[FEELSSOL_PDA_SEED], &feelssol_controller::id());
    let (vault_pda, _) =
        Pubkey::find_program_address(&[VAULT_PDA_SEED], &feelssol_controller::id());
    let (keeper_pda, _) = Pubkey::find_program_address(&[KEEPER_PDA_SEED], &feels_keeper::id());

    // Use the feels token key to always get the same mint address
    let feelssol_mint = Keypair::from_seed(&FEELS_PRIVATE_KEY).unwrap();
    let feelssol_mint_pubkey = feelssol_mint.pubkey();

    // If this fails it's OK because it means it has already been initialized. It's useful for rerunning tests
    // Initializes the feels protocol
    let result = protocol
        .request()
        .accounts(accounts::Initialize {
            protocol_state: protocol_pda,
            treasury: treasury_pda,
            authority: protocol.payer(),
            payer: protocol.payer(),
            system_program: system_program::ID,
        })
        .args(crate::instruction::Initialize {
            token_factory: feels_token_factory::id(),
            feelssol_controller: feelssol_controller::id(),
            default_protocol_fee_rate: 2000,
            max_pool_fee_rate: 10000,
        })
        .send();
    match result {
        Ok(_) => {}
        Err(_) => {
            println!("Failed to initialize protocol. Protocol may already be initialized.");
        }
    }

    // Initialize the feels keeper
    let result = feels_keeper
        .request()
        .accounts(feels_keeper::accounts::Initialize {
            keeper: keeper_pda,
            authority: protocol.payer(),
            payer: protocol.payer(),
            system_program: system_program::ID,
        })
        .args(feels_keeper::instruction::Initialize {
            feelssol_to_lst_rate_numerator: 2,
            feelssol_to_lst_rate_denominator: 1,
        })
        .send();
    match result {
        Ok(_) => {}
        Err(_) => {
            println!("Failed to initialize keeper. It may already be initialized.");
        }
    }

    // Initialize the feelssol controller
    let result = feelssol_controller
        .request()
        .accounts(feelssol_controller::accounts::Initialize {
            feelssol: feelssol_pda,
            feels_mint: feelssol_mint_pubkey,
            payer: protocol.payer(),
            token_program: spl_token_2022::id(),
            system_program: system_program::ID,
            rent: sysvar::rent::ID,
        })
        .args(feelssol_controller::instruction::Initialize {
            underlying_mint: JITOSOL_MINT.parse().unwrap(),
            keeper: keeper_pda,
            feels_protocol: protocol.id(),
        })
        .signer(&feelssol_mint)
        .send();
    match result {
        Ok(_) => {}
        Err(_) => {
            println!("Failed to initialize controller. It may already be initialized.");
        }
    }

    (
        protocol_pda,
        treasury_pda,
        feelssol_pda,
        feelssol_mint_pubkey,
        vault_pda,
        keeper_pda,
    )
}

fn get_jitosol_by_staking(
    program: &anchor_client::Program<&Keypair>,
    user: &Keypair,
    sol_amount: u64,
) -> u64 {
    let jitosol_mint = JITOSOL_MINT.parse().unwrap();
    let jito_stake_pool = JITO_STAKE_POOL.parse().unwrap();

    // Create user's jitoSOL token account first
    let user_jitosol_account = get_associated_token_address(&user.pubkey(), &jitosol_mint);

    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &program.payer(),
        &user.pubkey(),
        &jitosol_mint,
        &spl_token::id(),
    );

    // Get stake pool data to understand the structure
    let stake_pool_account = program.rpc().get_account(&jito_stake_pool).unwrap();
    let stake_pool_data = StakePool::deserialize(&mut &stake_pool_account.data[..]).unwrap();

    // When sol_deposit_authority is None, use the known Jito authority
    let sol_deposit_authority = if let Some(auth) = stake_pool_data.sol_deposit_authority {
        auth
    } else {
        // Use the known Jito sol deposit authority
        "6iQKfEyhr3bZMotVkW6beNZz5CPAkiwvgV2CTje9pVSS"
            .parse()
            .unwrap()
    };

    // Use Jito's deposit_sol instruction to get jitoSOL
    let deposit_ix = deposit_sol(
        &spl_stake_pool::id(),
        &jito_stake_pool,
        &sol_deposit_authority, // Jito allows SOL deposits
        &stake_pool_data.reserve_stake,
        &user.pubkey(),
        &user_jitosol_account,
        &stake_pool_data.manager_fee_account,
        &user_jitosol_account, // referrer token account (can be same)
        &stake_pool_data.pool_mint,
        &spl_token::id(),
        sol_amount,
    );

    program
        .request()
        .instruction(create_ata_ix)
        .instruction(deposit_ix)
        .signer(user)
        .send()
        .unwrap();

    // Return the amount of jitoSOL received
    get_token_balance(program, &user_jitosol_account)
}

#[test]
fn test_full_flow_deposit_withdraw_success() {
    let test_components = setup_test_components();
    let (protocol_program, feelssol_controller_program, feels_keeper_program) =
        test_components.programs();

    // Deploy the protocol and factory
    let (protocol_pda, _, feelssol_pda, feelssol_mint, vault_pda, keeper_pda) =
        deploy_protocol_and_controller_on_test_validator(
            &protocol_program,
            &feelssol_controller_program,
            &feels_keeper_program,
        );

    let staking_amount = 5_000_000_000; // 5 SOL
    let jitosol_received =
        get_jitosol_by_staking(&protocol_program, &test_components.user, staking_amount);
    assert!(jitosol_received > 0);

    let user_jitosol_account = get_associated_token_address(
        &test_components.user_pubkey(),
        &JITOSOL_MINT.parse().unwrap(),
    );
    let user_feelssol_account = get_associated_token_address_with_program_id(
        &test_components.user_pubkey(),
        &feelssol_mint,
        &spl_token_2022::id(),
    );

    let deposit_amount = jitosol_received / 2; // Deposit half of the jitoSOL received

    let vault_balance_before = get_token_balance(&protocol_program, &vault_pda);

    // Get the total_wrapped from the feelsSol state
    let feelssol_account = protocol_program.rpc().get_account(&feelssol_pda).unwrap();
    let feelssol_state = FeelsSolController::deserialize(&mut &feelssol_account.data[8..]).unwrap();
    let total_wrapped_before = feelssol_state.total_wrapped;

    protocol_program
        .request()
        .accounts(accounts::Deposit {
            protocol: protocol_pda,
            feelssol: feelssol_pda,
            feelssol_controller: feelssol_controller::id(),
            feels_mint: feelssol_mint,
            user_lst: user_jitosol_account,
            user_feelssol: user_feelssol_account,
            lst_vault: vault_pda,
            underlying_mint: JITOSOL_MINT.parse().unwrap(),
            keeper: keeper_pda,
            user: test_components.user_pubkey(),
            token_program: spl_token::ID,
            token_2022_program: spl_token_2022::ID,
            associated_token_program: associated_token::ID,
            system_program: system_program::ID,
            rent: sysvar::rent::ID,
            instructions: sysvar::instructions::ID,
        })
        .args(crate::instruction::Deposit {
            amount: deposit_amount,
        })
        .signer(&test_components.user)
        .send()
        .unwrap();

    let vault_balance_after = get_token_balance(&protocol_program, &vault_pda);
    assert_eq!(vault_balance_after, vault_balance_before + deposit_amount);

    let feelssol_account = protocol_program.rpc().get_account(&feelssol_pda).unwrap();
    let feelssol_state = FeelsSolController::deserialize(&mut &feelssol_account.data[8..]).unwrap();
    let total_wrapped_after = feelssol_state.total_wrapped;
    assert_eq!(total_wrapped_after, total_wrapped_before + deposit_amount);

    let user_initial_feelssol_balance =
        get_token2022_balance(&protocol_program, &user_feelssol_account);

    // Should have received half of the deposited amount in feelsSOL (2:1 rate)
    assert_eq!(user_initial_feelssol_balance, deposit_amount / 2);

    // Deposit the other half
    let vault_balance_before = get_token_balance(&protocol_program, &vault_pda);

    protocol_program
        .request()
        .accounts(accounts::Deposit {
            protocol: protocol_pda,
            feelssol: feelssol_pda,
            feelssol_controller: feelssol_controller::id(),
            feels_mint: feelssol_mint,
            user_lst: user_jitosol_account,
            user_feelssol: user_feelssol_account,
            lst_vault: vault_pda,
            underlying_mint: JITOSOL_MINT.parse().unwrap(),
            keeper: keeper_pda,
            user: test_components.user_pubkey(),
            token_program: spl_token::ID,
            token_2022_program: spl_token_2022::ID,
            associated_token_program: associated_token::ID,
            system_program: system_program::ID,
            rent: sysvar::rent::ID,
            instructions: sysvar::instructions::ID,
        })
        .args(crate::instruction::Deposit {
            amount: deposit_amount,
        })
        .signer(&test_components.user)
        .send()
        .unwrap();

    let vault_balance_after = get_token_balance(&protocol_program, &vault_pda);
    assert_eq!(vault_balance_after, vault_balance_before + deposit_amount);

    let user_feelssol_balance = get_token2022_balance(&protocol_program, &user_feelssol_account);
    assert_eq!(
        user_feelssol_balance,
        user_initial_feelssol_balance + (deposit_amount / 2)
    );

    // Lets try to withdraw now
    let vault_balance_before = get_token_balance(&protocol_program, &vault_pda);

    // Get FeelsSOL balance before withdraw
    let user_feelssol_balance_before_withdraw =
        get_token2022_balance(&protocol_program, &user_feelssol_account);

    let user_lst_balance_before = get_token_balance(&protocol_program, &user_jitosol_account);

    let feelssol_account = protocol_program.rpc().get_account(&feelssol_pda).unwrap();
    let feelssol_state = FeelsSolController::deserialize(&mut &feelssol_account.data[8..]).unwrap();
    let total_wrapped_before = feelssol_state.total_wrapped;

    let withdraw_amount = user_feelssol_balance / 2; // Withdraw half of the feelsSOL

    protocol_program
        .request()
        .accounts(accounts::Withdraw {
            protocol: protocol_pda,
            feelssol: feelssol_pda,
            feelssol_controller: feelssol_controller::id(),
            feels_mint: feelssol_mint,
            user_lst: user_jitosol_account,
            user_feelssol: user_feelssol_account,
            lst_vault: vault_pda,
            underlying_mint: JITOSOL_MINT.parse().unwrap(),
            keeper: keeper_pda,
            user: test_components.user_pubkey(),
            token_program: spl_token::ID,
            token_2022_program: spl_token_2022::ID,
            associated_token_program: associated_token::ID,
            system_program: system_program::ID,
            rent: sysvar::rent::ID,
            instructions: sysvar::instructions::ID,
        })
        .args(crate::instruction::Withdraw {
            amount: withdraw_amount,
        })
        .signer(&test_components.user)
        .send()
        .unwrap();

    // User LST balance should have gone up, precisely by twice amount of feelsSOL withdrawn (2:1 rate)
    let user_lst_balance_after = get_token_balance(&protocol_program, &user_jitosol_account);
    assert_eq!(
        user_lst_balance_after,
        user_lst_balance_before + (withdraw_amount * 2)
    );

    // Fewer LST should be wrapped - the difference between the user lst balance
    let feelssol_account = protocol_program.rpc().get_account(&feelssol_pda).unwrap();
    let feelssol_state = FeelsSolController::deserialize(&mut &feelssol_account.data[8..]).unwrap();
    let total_wrapped_after = feelssol_state.total_wrapped;
    assert_eq!(
        total_wrapped_after,
        total_wrapped_before - user_lst_balance_after + user_lst_balance_before
    );

    // Vault balance should have gone down in the same way
    let vault_balance_after = get_token_balance(&protocol_program, &vault_pda);
    assert_eq!(
        vault_balance_after,
        vault_balance_before - user_lst_balance_after + user_lst_balance_before
    );

    // Some feelssol should have been burned
    let user_feelssol_balance_after_withdraw =
        get_token2022_balance(&protocol_program, &user_feelssol_account);
    assert!(user_feelssol_balance_after_withdraw < user_feelssol_balance_before_withdraw);

    // Trying to withdraw more than available balance should fail
    let withdraw_amount = user_feelssol_balance_after_withdraw + 1;
    let result = protocol_program
        .request()
        .accounts(accounts::Withdraw {
            protocol: protocol_pda,
            feelssol: feelssol_pda,
            feelssol_controller: feelssol_controller::id(),
            feels_mint: feelssol_mint,
            user_lst: user_jitosol_account,
            user_feelssol: user_feelssol_account,
            lst_vault: vault_pda,
            underlying_mint: JITOSOL_MINT.parse().unwrap(),
            keeper: keeper_pda,
            user: test_components.user_pubkey(),
            token_program: spl_token::ID,
            token_2022_program: spl_token_2022::ID,
            associated_token_program: associated_token::ID,
            system_program: system_program::ID,
            rent: sysvar::rent::ID,
            instructions: sysvar::instructions::ID,
        })
        .args(crate::instruction::Withdraw {
            amount: withdraw_amount,
        })
        .signer(&test_components.user)
        .send();

    let anchor_error_code: u32 = FeelsSolError::InsufficientBalance.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));

    // Trying to withdraw zero tokens should also fail
    let result = protocol_program
        .request()
        .accounts(accounts::Withdraw {
            protocol: protocol_pda,
            feelssol: feelssol_pda,
            feelssol_controller: feelssol_controller::id(),
            feels_mint: feelssol_mint,
            user_lst: user_jitosol_account,
            user_feelssol: user_feelssol_account,
            lst_vault: vault_pda,
            underlying_mint: JITOSOL_MINT.parse().unwrap(),
            keeper: keeper_pda,
            user: test_components.user_pubkey(),
            token_program: spl_token::ID,
            token_2022_program: spl_token_2022::ID,
            associated_token_program: associated_token::ID,
            system_program: system_program::ID,
            rent: sysvar::rent::ID,
            instructions: sysvar::instructions::ID,
        })
        .args(crate::instruction::Withdraw { amount: 0 })
        .signer(&test_components.user)
        .send();

    let anchor_error_code: u32 = FeelsSolError::InvalidAmount.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}

#[test]
fn test_deposit_fails_zero_amount() {
    let test_components = setup_test_components();
    let (protocol_program, feelssol_controller_program, keeper_program) =
        test_components.programs();

    // Deploy the protocol and factory
    let (protocol_pda, _, feelssol_pda, feelssol_mint, vault_pda, keeper_pda) =
        deploy_protocol_and_controller_on_test_validator(
            &protocol_program,
            &feelssol_controller_program,
            &keeper_program,
        );

    let staking_amount = 5_000_000_000; // 5 SOL
    let jitosol_received =
        get_jitosol_by_staking(&protocol_program, &test_components.user, staking_amount);
    assert!(jitosol_received > 0);

    let user_jitosol_account = get_associated_token_address(
        &test_components.user_pubkey(),
        &JITOSOL_MINT.parse().unwrap(),
    );
    let user_feelssol_account = get_associated_token_address_with_program_id(
        &test_components.user_pubkey(),
        &feelssol_mint,
        &spl_token_2022::id(),
    );

    let result = protocol_program
        .request()
        .accounts(accounts::Deposit {
            protocol: protocol_pda,
            feelssol: feelssol_pda,
            feelssol_controller: feelssol_controller::id(),
            feels_mint: feelssol_mint,
            user_lst: user_jitosol_account,
            user_feelssol: user_feelssol_account,
            lst_vault: vault_pda,
            underlying_mint: JITOSOL_MINT.parse().unwrap(),
            keeper: keeper_pda,
            user: test_components.user_pubkey(),
            token_program: spl_token::ID,
            token_2022_program: spl_token_2022::ID,
            associated_token_program: associated_token::ID,
            system_program: system_program::ID,
            rent: sysvar::rent::ID,
            instructions: sysvar::instructions::ID,
        })
        .args(crate::instruction::Deposit { amount: 0 })
        .signer(&test_components.user)
        .send();

    let anchor_error_code: u32 = FeelsSolError::InvalidAmount.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}

#[test]
fn test_deposit_fails_not_enough_lst() {
    let test_components = setup_test_components();
    let (protocol_program, feelssol_controller_program, keeper_program) =
        test_components.programs();

    // Deploy the protocol and factory
    let (protocol_pda, _, feelssol_pda, feelssol_mint, vault_pda, keeper_pda) =
        deploy_protocol_and_controller_on_test_validator(
            &protocol_program,
            &feelssol_controller_program,
            &keeper_program,
        );

    let staking_amount = 5_000_000_000; // 5 SOL
    let jitosol_received =
        get_jitosol_by_staking(&protocol_program, &test_components.user, staking_amount);
    assert!(jitosol_received > 0);

    let user_jitosol_account = get_associated_token_address(
        &test_components.user_pubkey(),
        &JITOSOL_MINT.parse().unwrap(),
    );
    let user_feelssol_account = get_associated_token_address_with_program_id(
        &test_components.user_pubkey(),
        &feelssol_mint,
        &spl_token_2022::id(),
    );

    let result = protocol_program
        .request()
        .accounts(accounts::Deposit {
            protocol: protocol_pda,
            feelssol: feelssol_pda,
            feelssol_controller: feelssol_controller::id(),
            feels_mint: feelssol_mint,
            user_lst: user_jitosol_account,
            user_feelssol: user_feelssol_account,
            lst_vault: vault_pda,
            underlying_mint: JITOSOL_MINT.parse().unwrap(),
            keeper: keeper_pda,
            user: test_components.user_pubkey(),
            token_program: spl_token::ID,
            token_2022_program: spl_token_2022::ID,
            associated_token_program: associated_token::ID,
            system_program: system_program::ID,
            rent: sysvar::rent::ID,
            instructions: sysvar::instructions::ID,
        })
        .args(crate::instruction::Deposit {
            amount: 10_000_000_000,
        })
        .signer(&test_components.user)
        .send();

    let anchor_error_code: u32 = FeelsSolError::InsufficientBalance.into();
    let anchor_hex_error_code = format!("{:x}", anchor_error_code);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains(&anchor_hex_error_code));
}
