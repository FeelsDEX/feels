#!/usr/bin/env node

/**
 * Initialize Feels Protocol
 * This must be run once before any other operations
 * 
 * Usage:
 *   node scripts/initialize-protocol.js
 */

const { 
  Connection, 
  Keypair, 
  PublicKey,
  SystemProgram,
  LAMPORTS_PER_SOL
} = require('@solana/web3.js');
const { Program, AnchorProvider, BN } = require('@coral-xyz/anchor');
const fs = require('fs');
const path = require('path');

// Get program ID from environment or use default
const programIdStr = process.env.FEELS_PROGRAM_ID || process.env.NEXT_PUBLIC_FEELS_PROGRAM_ID || '9dGWDZq8nkb9BXGS6EZzEtTz1WhTEQLXXoNUkRLcKdG8';
const FEELS_PROGRAM_ID = new PublicKey(programIdStr);

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

// Load configuration
function loadConfig() {
  const configPath = path.join(__dirname, 'localnet-tokens.json');
  if (!fs.existsSync(configPath)) {
    throw new Error('Configuration not found. Please run setup-jitosol.js first.');
  }
  return JSON.parse(fs.readFileSync(configPath, 'utf-8'));
}

async function main() {
  console.log('Initializing Feels Protocol...\n');

  try {
    // Connect to local validator
    const connection = new Connection('http://localhost:8899', 'confirmed');
    const wallet = loadWallet();
    console.log('Using wallet:', wallet.publicKey.toBase58());

    // Check balance
    const balance = await connection.getBalance(wallet.publicKey);
    console.log('Wallet balance:', balance / LAMPORTS_PER_SOL, 'SOL\n');

    // Create provider and program
    const provider = new AnchorProvider(
      connection,
      { publicKey: wallet.publicKey, signTransaction: async (tx) => tx, signAllTransactions: async (txs) => txs },
      { commitment: 'confirmed' }
    );
    
    const idl = loadIDL();
    idl.address = FEELS_PROGRAM_ID.toBase58();
    const program = new Program(idl, provider);

    // Load token configuration
    const config = loadConfig();
    const jitosolMint = new PublicKey(config.jitosol.mint);
    
    console.log('JitoSOL mint:', jitosolMint.toBase58());

    // Derive PDAs
    const [protocolConfig] = PublicKey.findProgramAddressSync(
      [Buffer.from('protocol_config')],
      program.programId
    );

    const [protocolOracle] = PublicKey.findProgramAddressSync(
      [Buffer.from('protocol_oracle')],
      program.programId
    );

    const [safetyController] = PublicKey.findProgramAddressSync(
      [Buffer.from('safety_controller')],
      program.programId
    );

    // Check if already initialized
    const configAccount = await connection.getAccountInfo(protocolConfig);
    if (configAccount) {
      console.log('✓ Protocol already initialized');
      console.log('  Config:', protocolConfig.toBase58());
      console.log('  Oracle:', protocolOracle.toBase58());
      console.log('  Safety:', safetyController.toBase58());
      return;
    }

    // Initialize protocol parameters
    const params = {
      mintFee: new BN(1000000), // 0.001 SOL in lamports
      treasury: wallet.publicKey,
      defaultProtocolFeeRate: 8, // 0.08%
      defaultCreatorFeeRate: 2, // 0.02%
      maxProtocolFeeRate: 50, // 0.5%
      tokenExpirationSeconds: new BN(3600), // 1 hour
      jitosol: jitosolMint,
      dexTwapUpdater: wallet.publicKey,
      depegThresholdBps: 50,
      depegRequiredObs: 3,
      clearRequiredObs: 5,
      dexTwapWindowSecs: 900,
      dexTwapStaleAgeSecs: 1800,
      dexWhitelist: [], // Empty whitelist for now
      mintPerSlotCapFeelssol: new BN(0), // No cap
      redeemPerSlotCapFeelssol: new BN(0), // No cap
    };

    console.log('Initializing protocol with parameters:');
    console.log('  Mint fee:', params.mintFee.toString(), 'lamports');
    console.log('  Treasury:', params.treasury.toBase58());
    console.log('  Protocol fee rate:', params.defaultProtocolFeeRate / 100, '%');
    console.log('  Creator fee rate:', params.defaultCreatorFeeRate / 100, '%');
    console.log('  JitoSOL:', params.jitosol.toBase58());
    console.log('  Token expiration:', params.tokenExpirationSeconds.toString(), 'seconds');

    try {
      const tx = await program.methods
        .initializeProtocol(params)
        .accounts({
          authority: wallet.publicKey,
          protocolConfig,
          systemProgram: SystemProgram.programId,
          protocolOracle,
          safety: safetyController,
        })
        .signers([wallet])
        .rpc();

      console.log('\n✓ Protocol initialized successfully!');
      console.log('Transaction:', tx);
      console.log('\nPDAs created:');
      console.log('  Protocol config:', protocolConfig.toBase58());
      console.log('  Protocol oracle:', protocolOracle.toBase58());
      console.log('  Safety controller:', safetyController.toBase58());
      
    } catch (error) {
      console.error('Failed to initialize protocol:', error);
      if (error.logs) {
        console.error('Program logs:', error.logs);
      }
      process.exit(1);
    }

  } catch (error) {
    console.error('Error:', error);
    process.exit(1);
  }
}

main().catch(console.error);