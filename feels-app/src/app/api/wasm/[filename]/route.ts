import { NextRequest, NextResponse } from 'next/server';
import fs from 'fs/promises';
import path from 'path';

export async function GET(
  request: NextRequest,
  { params }: { params: { filename: string } }
) {
  const { filename } = params;
  
  // Security check - only allow specific WASM files
  const allowedFiles = [
    'vanity_miner_wasm.js',
    'vanity_miner_wasm_bg.wasm',
    'vanity_miner_wasm.d.ts',
    'vanity_miner_wasm_bg.wasm.d.ts',
  ];
  
  if (!allowedFiles.includes(filename)) {
    return NextResponse.json({ error: 'File not found' }, { status: 404 });
  }
  
  try {
    const filePath = path.join(process.cwd(), 'public', 'wasm', filename);
    const fileContent = await fs.readFile(filePath);
    
    // Set appropriate content type
    let contentType = 'application/octet-stream';
    if (filename.endsWith('.js')) {
      contentType = 'application/javascript';
    } else if (filename.endsWith('.wasm')) {
      contentType = 'application/wasm';
    } else if (filename.endsWith('.d.ts')) {
      contentType = 'text/plain';
    }
    
    return new NextResponse(fileContent, {
      headers: {
        'Content-Type': contentType,
        'Cache-Control': 'public, max-age=31536000, immutable',
        'Cross-Origin-Resource-Policy': 'cross-origin',
        'Cross-Origin-Embedder-Policy': 'require-corp',
      },
    });
  } catch (error) {
    console.error('Error serving WASM file:', error);
    return NextResponse.json({ error: 'Internal server error' }, { status: 500 });
  }
}