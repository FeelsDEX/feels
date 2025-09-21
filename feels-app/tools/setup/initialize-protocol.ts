#!/usr/bin/env tsx
/**
 * Initialize Feels Protocol
 * This must be run once before any other operations
 */

import { 
  Connection, 
  Keypair, 
  PublicKey,
  SystemProgram,
  LAMPORTS_PER_SOL
} from '@solana/web3.js';
import { Program, AnchorProvider, BN } from '@coral-xyz/anchor';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

// Load wallet from local keypair file
function loadWallet(): Keypair {
  const walletPath = process.env['SOLANA_WALLET'] || path.join(os.homedir(), '.config/solana/id.json');
  if (!fs.existsSync(walletPath)) {
    throw new Error(`Wallet not found at: ${walletPath}`);
  }
  const walletData = fs.readFileSync(walletPath, 'utf-8');
  return Keypair.fromSecretKey(new Uint8Array(JSON.parse(walletData)));
}

// Load IDL
function loadIDL(programId: string) {
  const idlPath = path.resolve(__dirname, '../../../target/idl/feels.json');
  if (!fs.existsSync(idlPath)) {
    throw new Error(`IDL not found at: ${idlPath}`);
  }
  const idl = JSON.parse(fs.readFileSync(idlPath, 'utf-8'));
  idl.address = programId;
  return idl;
}

async function main() {
  console.log('Initializing Feels Protocol...\n');

  // Parse command line args
  const args = process.argv.slice(2);
  const programIdStr = args[0] || process.env['FEELS_PROGRAM_ID'] || process.env['NEXT_PUBLIC_FEELS_PROGRAM_ID'];
  const jitosolMintStr = args[1];
  const rpcUrl = args[2] || 'http://localhost:8899';

  if (!programIdStr || !jitosolMintStr) {
    console.error('Usage: initialize-protocol.ts <program-id> <jitosol-mint> [rpc-url]');
    console.error('Example: initialize-protocol.ts 9dGWD... BatGa... http://localhost:8899');
    process.exit(1);
  }

  const FEELS_PROGRAM_ID = new PublicKey(programIdStr);
  const jitosolMint = new PublicKey(jitosolMintStr);

  try {
    // Connect to validator
    const connection = new Connection(rpcUrl, 'confirmed');
    const wallet = loadWallet();
    console.log('Using wallet:', wallet.publicKey.toBase58());

    // Check balance
    const balance = await connection.getBalance(wallet.publicKey);
    console.log('Wallet balance:', balance / LAMPORTS_PER_SOL, 'SOL\n');

    if (balance < 0.01 * LAMPORTS_PER_SOL) {
      throw new Error('Insufficient balance. Need at least 0.01 SOL');
    }

    // Create provider and program
    const provider = new AnchorProvider(
      connection,
      { publicKey: wallet.publicKey, signTransaction: async (tx) => tx, signAllTransactions: async (txs) => txs },
      { commitment: 'confirmed' }
    );
    
    const idl = loadIDL(FEELS_PROGRAM_ID.toBase58());
    const program = new Program(idl, provider);

    console.log('Program ID:', FEELS_PROGRAM_ID.toBase58());
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

    console.log('\nInitializing protocol with parameters:');
    console.log('  Mint fee:', params.mintFee.toString(), 'lamports');
    console.log('  Treasury:', params.treasury.toBase58());
    console.log('  Protocol fee rate:', params.defaultProtocolFeeRate / 100, '%');
    console.log('  Creator fee rate:', params.defaultCreatorFeeRate / 100, '%');
    console.log('  JitoSOL:', params.jitosol.toBase58());
    console.log('  Token expiration:', params.tokenExpirationSeconds.toString(), 'seconds');

    const tx = await (program.methods as any)
      ['initializeProtocol'](params)
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
    console.error('\nError:', error);
    process.exit(1);
  }
}

if (require.main === module) {
  main().catch(console.error);
}