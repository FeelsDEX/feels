#!/usr/bin/env node

/**
 * Setup JitoSOL and FeelsSOL for local testing
 * 
 * This script:
 * 1. Creates a mock JitoSOL mint (since we can't use the real Jito stake pool on localnet)
 * 2. Creates the FeelsSOL mint
 * 3. Initializes the FeelsSOL hub that connects JitoSOL <-> FeelsSOL
 * 4. Provides functions to mint test JitoSOL
 */

const { 
  Connection, 
  Keypair, 
  PublicKey, 
  SystemProgram,
  Transaction,
  LAMPORTS_PER_SOL,
  sendAndConfirmTransaction
} = require('@solana/web3.js');
const { 
  createInitializeMintInstruction,
  createAssociatedTokenAccountInstruction,
  createMintToInstruction,
  getAssociatedTokenAddress,
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID
} = require('@solana/spl-token');
const { Program, AnchorProvider, BN } = require('@coral-xyz/anchor');
const fs = require('fs');
const path = require('path');

// Constants
const FEELSSOL_DECIMALS = 9;
const JITOSOL_DECIMALS = 9;
const FEELS_PROGRAM_ID = new PublicKey('Cbv2aa2zMJdwAwzLnRZuWQ8efpr6Xb9zxpJhEzLe3v6N');

// Load wallet from local keypair file
function loadWallet() {
  const walletPath = process.env.SOLANA_WALLET || path.join(process.env.HOME, '.config/solana/id.json');
  const walletData = fs.readFileSync(walletPath, 'utf-8');
  return Keypair.fromSecretKey(new Uint8Array(JSON.parse(walletData)));
}

// Load IDL
function loadIDL() {
  const idlPath = path.join(__dirname, '../../target/idl/feels.json');
  return JSON.parse(fs.readFileSync(idlPath, 'utf-8'));
}

async function main() {
  console.log('Setting up JitoSOL and FeelsSOL for local testing...\n');

  // Connect to local validator
  const connection = new Connection('http://localhost:8899', 'confirmed');
  const wallet = loadWallet();
  console.log('Using wallet:', wallet.publicKey.toBase58());

  // Check balance
  const balance = await connection.getBalance(wallet.publicKey);
  console.log('Wallet balance:', balance / LAMPORTS_PER_SOL, 'SOL\n');

  if (balance < LAMPORTS_PER_SOL) {
    console.log('Requesting airdrop...');
    const airdropSig = await connection.requestAirdrop(wallet.publicKey, 10 * LAMPORTS_PER_SOL);
    await connection.confirmTransaction(airdropSig);
    console.log('Airdrop complete\n');
  }

  // Create provider and program
  const provider = new AnchorProvider(
    connection,
    { publicKey: wallet.publicKey, signTransaction: async (tx) => tx, signAllTransactions: async (txs) => txs },
    { commitment: 'confirmed' }
  );
  
  const idl = loadIDL();
  // Add the address field to the IDL
  idl.address = FEELS_PROGRAM_ID.toBase58();
  const program = new Program(idl, provider);

  try {
    // Step 0: Check protocol initialization
    console.log('Step 0: Checking protocol initialization...');
    const [protocolConfig] = PublicKey.findProgramAddressSync(
      [Buffer.from('protocol_config')],
      program.programId
    );
    
    const protocolAccount = await connection.getAccountInfo(protocolConfig);
    if (!protocolAccount) {
      console.log('Protocol not initialized.');
      console.log('Please initialize the protocol separately before running this script.');
      console.log('Continuing with token setup...\n');
    } else {
      console.log('✓ Protocol already initialized\n');
    }

    // Step 1: Create mock JitoSOL mint
    console.log('Step 1: Creating mock JitoSOL mint...');
    const jitosol = Keypair.generate();
    const jitosolAuthority = Keypair.generate();
    
    const createJitosolMintTx = new Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: wallet.publicKey,
        newAccountPubkey: jitosol.publicKey,
        space: MINT_SIZE,
        lamports: await connection.getMinimumBalanceForRentExemption(MINT_SIZE),
        programId: TOKEN_PROGRAM_ID,
      }),
      createInitializeMintInstruction(
        jitosol.publicKey,
        JITOSOL_DECIMALS,
        jitosolAuthority.publicKey,
        null,
        TOKEN_PROGRAM_ID
      )
    );

    await sendAndConfirmTransaction(connection, createJitosolMintTx, [wallet, jitosol]);
    console.log('✓ JitoSOL mint created:', jitosol.publicKey.toBase58());
    console.log('  Authority:', jitosolAuthority.publicKey.toBase58());

    // Step 2: Use existing FeelsSOL mint from config or create new one
    console.log('\nStep 2: Setting up FeelsSOL mint...');
    
    // Check if we have an existing config with FeelsSOL mint
    const configPath = path.join(__dirname, 'localnet-tokens.json');
    let feelssol, mintAuthority;
    
    if (fs.existsSync(configPath)) {
      const existingConfig = JSON.parse(fs.readFileSync(configPath, 'utf-8'));
      if (existingConfig.feelssol?.mint) {
        console.log('Using existing FeelsSOL mint from config');
        feelssol = { publicKey: new PublicKey(existingConfig.feelssol.mint) };
        mintAuthority = new PublicKey(existingConfig.feelssol.authority);
        console.log('✓ FeelsSOL mint:', feelssol.publicKey.toBase58());
        console.log('  Authority:', mintAuthority.toBase58());
      }
    }
    
    if (!feelssol) {
      console.log('No existing FeelsSOL mint found.');
      console.log('Please run create-feelssol-mint.js first to create a FeelsSOL mint with proper authority.');
      process.exit(1);
    }

    // Step 3: Initialize FeelsSOL hub
    console.log('\nStep 3: Initializing FeelsSOL hub...');
    
    // Derive PDAs
    const [feelsHub] = PublicKey.findProgramAddressSync(
      [Buffer.from('feels_hub'), feelssol.publicKey.toBuffer()],
      program.programId
    );
    
    const [jitosolVault] = PublicKey.findProgramAddressSync(
      [Buffer.from('jitosol_vault'), feelssol.publicKey.toBuffer()],
      program.programId
    );
    
    const [vaultAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from('vault_authority'), feelssol.publicKey.toBuffer()],
      program.programId
    );

    // Check if hub already exists
    const hubAccount = await connection.getAccountInfo(feelsHub);
    if (!hubAccount) {
      try {
        const tx = await program.methods
          .initializeHub()
          .accounts({
            payer: wallet.publicKey,
            feelssolMint: feelssol.publicKey,
            jitosolMint: jitosol.publicKey,
            hub: feelsHub,
            jitosolVault,
            vaultAuthority,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([wallet])
          .rpc();

        console.log('✓ FeelsSOL hub initialized:', tx);
        console.log('  Hub:', feelsHub.toBase58());
        console.log('  JitoSOL vault:', jitosolVault.toBase58());
      } catch (e) {
        console.error('Failed to initialize hub:', e.message || e);
        console.log('Note: The hub initialization may fail if the protocol is not properly initialized.');
        console.log('Please ensure the protocol is initialized before running this script.');
      }
    } else {
      console.log('✓ FeelsSOL hub already exists:', feelsHub.toBase58());
    }

    // Step 4: Update configuration with new values
    console.log('\nStep 4: Updating configuration...');
    
    // Load existing config to preserve FeelsSOL info
    let config = {};
    if (fs.existsSync(configPath)) {
      config = JSON.parse(fs.readFileSync(configPath, 'utf-8'));
    }
    
    // Update with new values
    config.jitosol = {
      mint: jitosol.publicKey.toBase58(),
      authority: jitosolAuthority.publicKey.toBase58(),
      authorityKeypair: Array.from(jitosolAuthority.secretKey),
    };
    
    // Keep existing feelssol config
    if (!config.feelssol) {
      config.feelssol = {
        mint: feelssol.publicKey.toBase58(),
        authority: mintAuthority.toBase58(),
        authorityKeypair: [], // PDA doesn't have a keypair
      };
    }
    
    config.hub = feelsHub.toBase58();
    config.jitosolVault = jitosolVault.toBase58();
    config.vaultAuthority = vaultAuthority.toBase58();

    fs.writeFileSync(configPath, JSON.stringify(config, null, 2));
    console.log('✓ Configuration updated');

    // Step 5: Mint some test JitoSOL to the wallet
    console.log('\nStep 5: Minting test JitoSOL...');
    
    // Create associated token account for JitoSOL
    const walletJitosol = await getAssociatedTokenAddress(
      jitosol.publicKey,
      wallet.publicKey
    );

    // Check if account exists
    const jitosolAccount = await connection.getAccountInfo(walletJitosol);
    if (!jitosolAccount) {
      const createAtaTx = new Transaction().add(
        createAssociatedTokenAccountInstruction(
          wallet.publicKey,
          walletJitosol,
          wallet.publicKey,
          jitosol.publicKey
        )
      );
      await sendAndConfirmTransaction(connection, createAtaTx, [wallet]);
      console.log('✓ Created JitoSOL ATA:', walletJitosol.toBase58());
    }

    // Mint 1000 JitoSOL for testing
    const mintAmount = 1000 * Math.pow(10, JITOSOL_DECIMALS);
    const mintTx = new Transaction().add(
      createMintToInstruction(
        jitosol.publicKey,
        walletJitosol,
        jitosolAuthority.publicKey,
        mintAmount
      )
    );
    await sendAndConfirmTransaction(connection, mintTx, [wallet, jitosolAuthority]);
    console.log('✓ Minted', mintAmount / Math.pow(10, JITOSOL_DECIMALS), 'JitoSOL to wallet');

    console.log('\n✅ Setup complete!');
    console.log('\nYou can now:');
    console.log('1. Use the CreateMarket component to create new markets');
    console.log('2. Enter FeelsSOL by swapping JitoSOL');
    console.log('3. Trade on FeelsSOL markets');
    
  } catch (error) {
    console.error('Setup failed:', error);
    process.exit(1);
  }
}

main().catch(console.error);