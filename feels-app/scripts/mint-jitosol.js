#!/usr/bin/env node

/**
 * Mint JitoSOL to a wallet for testing
 * 
 * Usage:
 *   node scripts/mint-jitosol.js <wallet-address> <amount>
 *   node scripts/mint-jitosol.js 7EL1Td8apSB1fJmPtfxGWJND2WUZMRaWkjkHFhR4DdVQ 100
 */

const { 
  Connection, 
  Keypair, 
  PublicKey,
  Transaction,
  sendAndConfirmTransaction
} = require('@solana/web3.js');
const { 
  createAssociatedTokenAccountInstruction,
  createMintToInstruction,
  getAssociatedTokenAddress,
  TOKEN_PROGRAM_ID
} = require('@solana/spl-token');
const fs = require('fs');
const path = require('path');

const JITOSOL_DECIMALS = 9;

// Load wallet from local keypair file
function loadWallet() {
  const walletPath = process.env.SOLANA_WALLET || path.join(process.env.HOME, '.config/solana/id.json');
  const walletData = fs.readFileSync(walletPath, 'utf-8');
  return Keypair.fromSecretKey(new Uint8Array(JSON.parse(walletData)));
}

// Load configuration from setup script
function loadConfig() {
  const configPath = path.join(__dirname, 'localnet-tokens.json');
  if (!fs.existsSync(configPath)) {
    throw new Error('Configuration not found. Please run setup-jitosol.js first.');
  }
  return JSON.parse(fs.readFileSync(configPath, 'utf-8'));
}

async function main() {
  // Parse command line arguments
  const args = process.argv.slice(2);
  if (args.length !== 2) {
    console.error('Usage: node scripts/mint-jitosol.js <wallet-address> <amount>');
    console.error('Example: node scripts/mint-jitosol.js 7EL1Td8apSB1fJmPtfxGWJND2WUZMRaWkjkHFhR4DdVQ 100');
    process.exit(1);
  }

  const recipientAddress = args[0];
  const amount = parseFloat(args[1]);

  if (!recipientAddress || isNaN(amount) || amount <= 0) {
    console.error('Invalid arguments');
    process.exit(1);
  }

  try {
    const recipient = new PublicKey(recipientAddress);
    
    console.log('Minting JitoSOL...');
    console.log('Recipient:', recipient.toBase58());
    console.log('Amount:', amount);

    // Connect to local validator
    const connection = new Connection('http://localhost:8899', 'confirmed');
    const wallet = loadWallet();

    // Load configuration
    const config = loadConfig();
    const jitosolMint = new PublicKey(config.jitosol.mint);
    const jitosolAuthority = Keypair.fromSecretKey(new Uint8Array(config.jitosol.authorityKeypair));

    // Get or create associated token account
    const recipientJitosol = await getAssociatedTokenAddress(
      jitosolMint,
      recipient
    );

    console.log('JitoSOL mint:', jitosolMint.toBase58());
    console.log('Recipient JitoSOL account:', recipientJitosol.toBase58());

    // Check if ATA exists
    const ataAccount = await connection.getAccountInfo(recipientJitosol);
    
    const tx = new Transaction();
    
    if (!ataAccount) {
      console.log('Creating associated token account...');
      tx.add(
        createAssociatedTokenAccountInstruction(
          wallet.publicKey,
          recipientJitosol,
          recipient,
          jitosolMint
        )
      );
    }

    // Mint tokens
    const mintAmount = amount * Math.pow(10, JITOSOL_DECIMALS);
    tx.add(
      createMintToInstruction(
        jitosolMint,
        recipientJitosol,
        jitosolAuthority.publicKey,
        mintAmount
      )
    );

    const signature = await sendAndConfirmTransaction(
      connection, 
      tx, 
      [wallet, jitosolAuthority]
    );

    console.log('✓ Successfully minted', amount, 'JitoSOL');
    console.log('Transaction:', signature);

    // Check balance
    const tokenAccountInfo = await connection.getTokenAccountBalance(recipientJitosol);
    console.log('New balance:', tokenAccountInfo.value.uiAmount, 'JitoSOL');

  } catch (error) {
    console.error('Failed to mint JitoSOL:', error);
    process.exit(1);
  }
}

main().catch(console.error);