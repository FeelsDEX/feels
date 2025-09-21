#!/usr/bin/env tsx
/**
 * Mint JitoSOL to a wallet for testing
 * 
 * Usage:
 *   mint-jitosol.ts <jitosol-mint> <wallet-address> <amount> [rpc-url]
 */

import { 
  Connection, 
  Keypair, 
  PublicKey,
  Transaction,
  sendAndConfirmTransaction
} from '@solana/web3.js';
import { 
  createAssociatedTokenAccountInstruction,
  createMintToInstruction,
  getAssociatedTokenAddress,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID
} from '@solana/spl-token';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

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
  // Parse command line arguments
  const args = process.argv.slice(2);
  if (args.length < 3) {
    console.error('Usage: mint-jitosol.ts <jitosol-mint> <wallet-address> <amount> [rpc-url]');
    console.error('Example: mint-jitosol.ts BatGa... 7EL1Td... 100 http://localhost:8899');
    process.exit(1);
  }

  const jitosolMintStr = args[0];
  const targetWalletStr = args[1];
  const amount = parseFloat(args[2] ?? '0');
  const rpcUrl = args[3] || 'http://localhost:8899';

  if (isNaN(amount) || amount <= 0) {
    console.error('Amount must be a positive number');
    process.exit(1);
  }

  console.log('Minting JitoSOL...\n');

  try {
    // Connect to validator
    const connection = new Connection(rpcUrl, 'confirmed');
    const authority = loadWallet();
    
    const jitosolMint = new PublicKey(jitosolMintStr ?? '');
    const targetWallet = new PublicKey(targetWalletStr ?? '');

    console.log('JitoSOL mint:', jitosolMint.toBase58());
    console.log('Target wallet:', targetWallet.toBase58());
    console.log('Amount:', amount, 'JitoSOL');
    console.log('Authority:', authority.publicKey.toBase58());

    // Get or create associated token account
    const ata = await getAssociatedTokenAddress(
      jitosolMint,
      targetWallet,
      false,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    const tx = new Transaction();

    // Check if ATA exists
    const ataInfo = await connection.getAccountInfo(ata);
    if (!ataInfo) {
      console.log('Creating associated token account...');
      tx.add(
        createAssociatedTokenAccountInstruction(
          authority.publicKey,
          ata,
          targetWallet,
          jitosolMint,
          TOKEN_PROGRAM_ID,
          ASSOCIATED_TOKEN_PROGRAM_ID
        )
      );
    }

    // Mint tokens
    const mintAmount = Math.floor(amount * 10 ** JITOSOL_DECIMALS);
    tx.add(
      createMintToInstruction(
        jitosolMint,
        ata,
        authority.publicKey,
        mintAmount,
        [],
        TOKEN_PROGRAM_ID
      )
    );

    const signature = await sendAndConfirmTransaction(
      connection,
      tx,
      [authority],
      { commitment: 'confirmed' }
    );

    console.log('\n✓ Minted successfully!');
    console.log('Transaction:', signature);
    console.log('Token account:', ata.toBase58());
    console.log('Amount minted:', amount, 'JitoSOL');

  } catch (error) {
    console.error('\nError:', error);
    process.exit(1);
  }
}

if (require.main === module) {
  main().catch(console.error);
}