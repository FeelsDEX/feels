#!/usr/bin/env node

/**
 * Create FeelsSOL mint with proper authority
 * 
 * This script creates the FeelsSOL mint token that will be controlled
 * by the Feels program's mint authority PDA.
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
  MINT_SIZE,
  TOKEN_PROGRAM_ID
} = require('@solana/spl-token');
const fs = require('fs');
const path = require('path');

// Constants
const FEELSSOL_DECIMALS = 9;

// Get program ID from environment or use default
const programIdStr = process.env.FEELS_PROGRAM_ID || process.env.NEXT_PUBLIC_FEELS_PROGRAM_ID || '9dGWDZq8nkb9BXGS6EZzEtTz1WhTEQLXXoNUkRLcKdG8';
const FEELS_PROGRAM_ID = new PublicKey(programIdStr);

// Load wallet from local keypair file
function loadWallet() {
  const walletPath = process.env.SOLANA_WALLET || path.join(process.env.HOME, '.config/solana/id.json');
  const walletData = fs.readFileSync(walletPath, 'utf-8');
  return Keypair.fromSecretKey(new Uint8Array(JSON.parse(walletData)));
}

async function main() {
  console.log('Creating FeelsSOL mint for local testing...\n');
  
  try {
    // Connect to local validator
    const connection = new Connection('http://localhost:8899', 'confirmed');
    const wallet = loadWallet();
    console.log('Using wallet:', wallet.publicKey.toBase58());
    console.log('Using program ID:', FEELS_PROGRAM_ID.toBase58());

    // Check balance
    const balance = await connection.getBalance(wallet.publicKey);
    console.log('Wallet balance:', balance / LAMPORTS_PER_SOL, 'SOL\n');

    if (balance < LAMPORTS_PER_SOL) {
      console.log('Requesting airdrop...');
      const airdropSig = await connection.requestAirdrop(wallet.publicKey, 10 * LAMPORTS_PER_SOL);
      await connection.confirmTransaction(airdropSig);
      console.log('Airdrop complete\n');
    }

    // Create FeelsSOL mint keypair
    const feelssol = Keypair.generate();
    
    // Derive the mint authority PDA that the program will use
    const [mintAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from('mint_authority'), feelssol.publicKey.toBuffer()],
      FEELS_PROGRAM_ID
    );
    
    console.log('Creating FeelsSOL mint...');
    console.log('  Mint:', feelssol.publicKey.toBase58());
    console.log('  Authority (PDA):', mintAuthority.toBase58());
    
    // Create mint account
    const createMintTx = new Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: wallet.publicKey,
        newAccountPubkey: feelssol.publicKey,
        space: MINT_SIZE,
        lamports: await connection.getMinimumBalanceForRentExemption(MINT_SIZE),
        programId: TOKEN_PROGRAM_ID,
      }),
      createInitializeMintInstruction(
        feelssol.publicKey,
        FEELSSOL_DECIMALS,
        mintAuthority,        // mint authority is the PDA
        null,                 // no freeze authority
        TOKEN_PROGRAM_ID
      )
    );

    await sendAndConfirmTransaction(connection, createMintTx, [wallet, feelssol]);
    console.log('Success: FeelsSOL mint created!');

    // Save configuration
    const configPath = path.join(__dirname, 'localnet-tokens.json');
    let config = {};
    
    // Load existing config if it exists
    if (fs.existsSync(configPath)) {
      config = JSON.parse(fs.readFileSync(configPath, 'utf-8'));
    }
    
    // Update with FeelsSOL info
    config.feelssol = {
      mint: feelssol.publicKey.toBase58(),
      authority: mintAuthority.toBase58(),
      authorityKeypair: [], // PDA doesn't have a keypair
    };
    
    fs.writeFileSync(configPath, JSON.stringify(config, null, 2));
    console.log('\nConfiguration saved to:', configPath);
    
    console.log('\nNext steps:');
    console.log('1. Deploy the Feels program if not already deployed');
    console.log('2. Run initialize-protocol.js to initialize the protocol');
    console.log('3. Run setup-jitosol.js to create JitoSOL and initialize the hub');
    
  } catch (error) {
    console.error('Error:', error);
    process.exit(1);
  }
}

main().catch(console.error);