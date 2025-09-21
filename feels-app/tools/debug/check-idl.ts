#!/usr/bin/env tsx
import * as fs from 'fs';
import * as path from 'path';

interface IDL {
  instructions: Array<{
    name: string;
    args: any[];
  }>;
  types?: Array<{
    name: string;
    [key: string]: any;
  }>;
}

function main() {
  const idlPath = path.resolve(__dirname, '../../src/idl/feels.json');
  
  if (!fs.existsSync(idlPath)) {
    console.error(`IDL file not found at: ${idlPath}`);
    process.exit(1);
  }

  const idl: IDL = JSON.parse(fs.readFileSync(idlPath, 'utf8'));

  console.log('Total instructions:', idl.instructions.length);
  const mintToken = idl.instructions.find(i => i.name === 'mint_token');
  console.log('Found mint_token:', !!mintToken);

  if (mintToken) {
    console.log('mint_token args:', JSON.stringify(mintToken.args, null, 2));
  }

  // Check MintTokenParams type
  const mintTokenParams = idl.types?.find(t => t.name === 'MintTokenParams');
  console.log('\nFound MintTokenParams type:', !!mintTokenParams);
  if (mintTokenParams) {
    console.log('MintTokenParams:', JSON.stringify(mintTokenParams, null, 2));
  }
}

if (require.main === module) {
  main();
}