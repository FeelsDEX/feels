import { NextResponse } from 'next/server';
import fs from 'fs';
import path from 'path';

export async function GET() {
  try {
    // Try to read the localnet tokens configuration
    const configPath = path.join(process.cwd(), 'scripts', 'localnet-tokens.json');
    
    if (fs.existsSync(configPath)) {
      const config = JSON.parse(fs.readFileSync(configPath, 'utf-8'));
      
      // Don't expose secret keys
      const safeConfig = {
        jitosol: {
          mint: config.jitosol?.mint,
          authority: config.jitosol?.authority,
        },
        feelssol: {
          mint: config.feelssol?.mint,
          authority: config.feelssol?.authority,
        },
        hub: config.hub,
        jitosolVault: config.jitosolVault,
        vaultAuthority: config.vaultAuthority,
      };
      
      return NextResponse.json(safeConfig);
    }
    
    // Return empty config if file doesn't exist
    return NextResponse.json({});
  } catch (error) {
    console.error('Failed to load localnet tokens:', error);
    return NextResponse.json({}, { status: 500 });
  }
}