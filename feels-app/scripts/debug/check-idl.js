const fs = require('fs');
const idl = JSON.parse(fs.readFileSync('./src/idl/feels.json', 'utf8'));

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