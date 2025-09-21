#!/usr/bin/env tsx
import { Connection, PublicKey } from '@solana/web3.js';

async function checkProtocol() {
  const connection = new Connection('http://localhost:8899', 'confirmed');
  const programId = new PublicKey('GfEnptgRs7gTk7nY9JCWff49dponYZJRMS5YzDsRproK');
  
  // Check if protocol config PDA exists
  const [protocolConfig] = PublicKey.findProgramAddressSync(
    [Buffer.from('protocol_config')],
    programId
  );
  
  try {
    const accountInfo = await connection.getAccountInfo(protocolConfig);
    if (accountInfo) {
      console.log('Protocol config exists at:', protocolConfig.toBase58());
      console.log('Data length:', accountInfo.data.length);
      console.log('Owner:', accountInfo.owner.toBase58());
    } else {
      console.log('Protocol config not found');
    }
  } catch (error) {
    console.error('Error checking protocol:', error);
  }
}

if (require.main === module) {
  checkProtocol().catch(console.error);
}