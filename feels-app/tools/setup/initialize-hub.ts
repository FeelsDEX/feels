#!/usr/bin/env tsx
/**
 * Initialize FeelsSOL Hub
 * Must be run after protocol initialization
 */

import { Connection, Keypair, PublicKey, SystemProgram } from '@solana/web3.js';
import { Program, AnchorProvider, Wallet } from '@coral-xyz/anchor';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

function loadWallet(): Keypair {
  const walletPath = process.env['SOLANA_WALLET'] || path.join(os.homedir(), '.config/solana/id.json');
  if (!fs.existsSync(walletPath)) {
    throw new Error(`Wallet not found at: ${walletPath}`);
  }
  const walletData = fs.readFileSync(walletPath, 'utf-8');
  return Keypair.fromSecretKey(new Uint8Array(JSON.parse(walletData)));
}

function loadIDL(programId: string) {
  const idlPath = path.resolve(__dirname, '../../../target/idl/feels.json');
  if (!fs.existsSync(idlPath)) {
    throw new Error(`IDL not found at: ${idlPath}`);
  }
  const idl = JSON.parse(fs.readFileSync(idlPath, 'utf-8'));
  idl.address = programId;
  return idl;
}

async function initializeHub() {
  // Parse command line args
  const args = process.argv.slice(2);
  const programIdStr = args[0] || process.env['FEELS_PROGRAM_ID'];
  const feelssolMintStr = args[1];
  const rpcUrl = args[2] || 'http://localhost:8899';

  if (!programIdStr || !feelssolMintStr) {
    console.error('Usage: initialize-hub.ts <program-id> <feelssol-mint> [rpc-url]');
    console.error('Example: initialize-hub.ts GfEnp... AVipL... http://localhost:8899');
    process.exit(1);
  }

  const connection = new Connection(rpcUrl, 'confirmed');
  const keypair = loadWallet();
  
  // Create wallet adapter from keypair
  const wallet: Wallet = {
    publicKey: keypair.publicKey,
    payer: keypair,
    signTransaction: async (tx) => {
      if ('partialSign' in tx) {
        tx.partialSign(keypair);
      }
      return tx;
    },
    signAllTransactions: async (txs) => {
      return txs.map(tx => {
        if ('partialSign' in tx) {
          tx.partialSign(keypair);
        }
        return tx;
      });
    }
  };
  
  const provider = new AnchorProvider(connection, wallet, { commitment: 'confirmed' });
  
  const programId = new PublicKey(programIdStr);
  const idl = loadIDL(programId.toBase58());
  const program = new Program(idl, provider);
  
  const feelssolMint = new PublicKey(feelssolMintStr);
  
  // PDAs
  const [protocolConfig] = PublicKey.findProgramAddressSync(
    [Buffer.from('protocol_config')],
    programId
  );
  
  const [feelssolHub] = PublicKey.findProgramAddressSync(
    [Buffer.from('feelssol_hub')],
    programId
  );
  
  const [hubAuthority] = PublicKey.findProgramAddressSync(
    [Buffer.from('hub_authority'), feelssolHub.toBuffer()],
    programId
  );
  
  try {
    console.log('Initializing FeelsSOL hub...');
    console.log('Program ID:', programId.toBase58());
    console.log('FeelsSOL mint:', feelssolMint.toBase58());
    console.log('Hub PDA:', feelssolHub.toBase58());
    console.log('Hub authority:', hubAuthority.toBase58());
    
    // Check if already initialized
    const hubAccount = await connection.getAccountInfo(feelssolHub);
    if (hubAccount) {
      console.log('Success: Hub already initialized');
      const hub = await (program.account as any)['feelssolHub'].fetch(feelssolHub);
      console.log('Hub state:', {
        mint: hub.mint.toBase58(),
        authority: hub.authority.toBase58(),
      });
      return;
    }
    
    const tx = await (program.methods as any)
      ['initializeHub']()
      .accounts({
        deployer: wallet.publicKey,
        protocolConfig,
        feelssolHub,
        hubAuthority,
        feelssolMint,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
      
    console.log('\nSuccess: Hub initialized successfully!');
    console.log('Transaction:', tx);
    
    // Read the hub state
    const hub = await (program.account as any)['feelssolHub'].fetch(feelssolHub);
    console.log('\nHub state:', {
      mint: hub.mint.toBase58(),
      authority: hub.authority.toBase58(),
    });
  } catch (error) {
    console.error('Error initializing hub:', error);
    process.exit(1);
  }
}

if (require.main === module) {
  initializeHub().catch(console.error);
}