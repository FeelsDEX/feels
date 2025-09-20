const { Connection, Keypair, PublicKey, SystemProgram } = require('@solana/web3.js');
const { Program, AnchorProvider, BN } = require('@coral-xyz/anchor');
const fs = require('fs');
const path = require('path');

async function initializeHub() {
  const connection = new Connection('http://localhost:8899', 'confirmed');
  const walletPath = path.join(process.env.HOME, '.config/solana/id.json');
  const wallet = Keypair.fromSecretKey(new Uint8Array(JSON.parse(fs.readFileSync(walletPath, 'utf-8'))));
  
  const provider = new AnchorProvider(connection, wallet, { commitment: 'confirmed' });
  
  const idlPath = path.join(__dirname, '../target/idl/feels.json');
  const idl = JSON.parse(fs.readFileSync(idlPath, 'utf-8'));
  const programId = new PublicKey('GfEnptgRs7gTk7nY9JCWff49dponYZJRMS5YzDsRproK');
  const program = new Program(idl, programId, provider);
  
  // FeelsSOL mint from the config
  const feelssolMint = new PublicKey('AVipLVdojBn8E2mogQ6pcBJLw7WQ1pZfbWM9TRF8xHGF');
  
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
    console.log('FeelsSOL mint:', feelssolMint.toBase58());
    console.log('Hub PDA:', feelssolHub.toBase58());
    console.log('Hub authority:', hubAuthority.toBase58());
    
    const tx = await program.methods
      .initializeHub()
      .accounts({
        deployer: wallet.publicKey,
        protocolConfig,
        feelssolHub,
        hubAuthority,
        feelssolMint,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
      
    console.log('Hub initialized\! Transaction:', tx);
    
    // Read the hub state
    const hub = await program.account.feelssolHub.fetch(feelssolHub);
    console.log('Hub state:', {
      mint: hub.mint.toBase58(),
      totalEntered: hub.totalEntered.toString(),
      totalExited: hub.totalExited.toString(),
      netFlow: hub.netFlow.toString(),
    });
  } catch (error) {
    if (error.toString().includes('already in use')) {
      console.log('Hub already initialized');
      const hub = await program.account.feelssolHub.fetch(feelssolHub);
      console.log('Existing hub state:', {
        mint: hub.mint.toBase58(),
        totalEntered: hub.totalEntered.toString(),
      });
    } else {
      console.error('Failed to initialize hub:', error);
    }
  }
}

initializeHub().catch(console.error);
