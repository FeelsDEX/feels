#!/usr/bin/env tsx
/**
 * Setup JitoSOL and FeelsSOL for local testing
 * 
 * This script:
 * 1. Creates a mock JitoSOL mint (since we can't use the real Jito stake pool on localnet)
 * 2. Creates the FeelsSOL mint
 * 3. Returns the created mint addresses for use in other scripts
 */

import { 
  Connection, 
  Keypair, 
  SystemProgram,
  Transaction,
  LAMPORTS_PER_SOL,
  sendAndConfirmTransaction
} from '@solana/web3.js';
import { 
  createInitializeMintInstruction,
  createAssociatedTokenAccountInstruction,
  createMintToInstruction,
  getAssociatedTokenAddress,
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID
} from '@solana/spl-token';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

// Constants
const FEELSSOL_DECIMALS = 9;
const JITOSOL_DECIMALS = 9;

// Load wallet from local keypair file
function loadWallet(): Keypair {
  const walletPath = process.env['SOLANA_WALLET'] || path.join(os.homedir(), '.config/solana/id.json');
  if (!fs.existsSync(walletPath)) {
    throw new Error(`Wallet not found at: ${walletPath}`);
  }
  const walletData = fs.readFileSync(walletPath, 'utf-8');
  return Keypair.fromSecretKey(new Uint8Array(JSON.parse(walletData)));
}

async function main() {
  console.log('Setting up JitoSOL and FeelsSOL for local testing...\n');

  const rpcUrl = process.argv[2] || 'http://localhost:8899';

  // Connect to validator
  const connection = new Connection(rpcUrl, 'confirmed');
  const wallet = loadWallet();
  console.log('Using wallet:', wallet.publicKey.toBase58());

  // Check balance
  const balance = await connection.getBalance(wallet.publicKey);
  console.log('Wallet balance:', balance / LAMPORTS_PER_SOL, 'SOL\n');

  if (balance < 0.1 * LAMPORTS_PER_SOL) {
    console.error('Insufficient balance. Need at least 0.1 SOL for rent and fees.');
    process.exit(1);
  }

  try {
    // Generate keypairs for the mints
    const jitosolMint = Keypair.generate();
    const feelssolMint = Keypair.generate();

    console.log('Creating mints...');
    console.log('JitoSOL mint:', jitosolMint.publicKey.toBase58());
    console.log('FeelsSOL mint:', feelssolMint.publicKey.toBase58());

    // Calculate rent
    const lamports = await connection.getMinimumBalanceForRentExemption(MINT_SIZE);

    // Create JitoSOL mint
    const createJitosolMintTx = new Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: wallet.publicKey,
        newAccountPubkey: jitosolMint.publicKey,
        space: MINT_SIZE,
        lamports,
        programId: TOKEN_PROGRAM_ID,
      }),
      createInitializeMintInstruction(
        jitosolMint.publicKey,
        JITOSOL_DECIMALS,
        wallet.publicKey, // mint authority
        wallet.publicKey, // freeze authority
        TOKEN_PROGRAM_ID
      )
    );

    await sendAndConfirmTransaction(
      connection, 
      createJitosolMintTx, 
      [wallet, jitosolMint],
      { commitment: 'confirmed' }
    );

    console.log('JitoSOL mint created');

    // Create FeelsSOL mint
    const createFeelssolMintTx = new Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: wallet.publicKey,
        newAccountPubkey: feelssolMint.publicKey,
        space: MINT_SIZE,
        lamports,
        programId: TOKEN_PROGRAM_ID,
      }),
      createInitializeMintInstruction(
        feelssolMint.publicKey,
        FEELSSOL_DECIMALS,
        wallet.publicKey, // mint authority (will be transferred to program)
        null, // no freeze authority
        TOKEN_PROGRAM_ID
      )
    );

    await sendAndConfirmTransaction(
      connection, 
      createFeelssolMintTx, 
      [wallet, feelssolMint],
      { commitment: 'confirmed' }
    );

    console.log('FeelsSOL mint created');

    // Create associated token accounts
    const jitosolAta = await getAssociatedTokenAddress(
      jitosolMint.publicKey,
      wallet.publicKey,
      false,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    const feelssolAta = await getAssociatedTokenAddress(
      feelssolMint.publicKey,
      wallet.publicKey,
      false,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    // Create ATAs
    const createAtasTx = new Transaction();

    const jitosolAtaInfo = await connection.getAccountInfo(jitosolAta);
    if (!jitosolAtaInfo) {
      createAtasTx.add(
        createAssociatedTokenAccountInstruction(
          wallet.publicKey,
          jitosolAta,
          wallet.publicKey,
          jitosolMint.publicKey,
          TOKEN_PROGRAM_ID,
          ASSOCIATED_TOKEN_PROGRAM_ID
        )
      );
    }

    const feelssolAtaInfo = await connection.getAccountInfo(feelssolAta);
    if (!feelssolAtaInfo) {
      createAtasTx.add(
        createAssociatedTokenAccountInstruction(
          wallet.publicKey,
          feelssolAta,
          wallet.publicKey,
          feelssolMint.publicKey,
          TOKEN_PROGRAM_ID,
          ASSOCIATED_TOKEN_PROGRAM_ID
        )
      );
    }

    if (createAtasTx.instructions.length > 0) {
      await sendAndConfirmTransaction(
        connection,
        createAtasTx,
        [wallet],
        { commitment: 'confirmed' }
      );
      console.log('Associated token accounts created');
    }

    // Mint some test JitoSOL
    const mintAmount = 1000 * 10 ** JITOSOL_DECIMALS; // 1000 JitoSOL
    const mintTx = new Transaction().add(
      createMintToInstruction(
        jitosolMint.publicKey,
        jitosolAta,
        wallet.publicKey,
        mintAmount,
        [],
        TOKEN_PROGRAM_ID
      )
    );

    await sendAndConfirmTransaction(
      connection,
      mintTx,
      [wallet],
      { commitment: 'confirmed' }
    );

    console.log(`Minted ${mintAmount / 10 ** JITOSOL_DECIMALS} test JitoSOL`);

    // Output configuration for use in other scripts
    console.log('\n=== Setup Complete ===');
    console.log('\nUse these values for protocol initialization:');
    console.log(`JitoSOL mint: ${jitosolMint.publicKey.toBase58()}`);
    console.log(`FeelsSOL mint: ${feelssolMint.publicKey.toBase58()}`);
    console.log('\nYour token accounts:');
    console.log(`JitoSOL ATA: ${jitosolAta.toBase58()}`);
    console.log(`FeelsSOL ATA: ${feelssolAta.toBase58()}`);

  } catch (error) {
    console.error('\nError:', error);
    process.exit(1);
  }
}

if (require.main === module) {
  main().catch(console.error);
}