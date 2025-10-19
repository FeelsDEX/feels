import { NextResponse } from 'next/server';
import fs from 'fs';
import path from 'path';

export async function GET() {
  try {
    const configPath = path.join(process.cwd(), 'config', 'localnet-metaplex.json');
    
    if (!fs.existsSync(configPath)) {
      return NextResponse.json({});
    }
    
    const config = JSON.parse(fs.readFileSync(configPath, 'utf-8'));
    return NextResponse.json(config);
  } catch (error) {
    console.error('Failed to load Metaplex config:', error);
    return NextResponse.json({}, { status: 200 });
  }
}