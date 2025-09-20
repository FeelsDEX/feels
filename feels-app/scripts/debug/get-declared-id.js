// Simple script to decode the error and extract the declared vs actual IDs
console.log('Checking program declared ID mismatch...');
console.log('Actual deployed program ID: GfEnptgRs7gTk7nY9JCWff49dponYZJRMS5YzDsRproK');
console.log('\nThe error indicates the program was built with a different declare_id\!');

const fs = require('fs');
const path = require('path');

// Let's check what program ID the frontend is using
const envPath = path.join(__dirname, '.env.local');
const envContent = fs.readFileSync(envPath, 'utf-8');
const match = envContent.match(/NEXT_PUBLIC_FEELS_PROGRAM_ID=(\w+)/);
if (match) {
  console.log('\nFrontend is configured to use:', match[1]);
}

// Check the IDL
const idlPath = path.join(__dirname, '../target/idl/feels.json');
const idl = JSON.parse(fs.readFileSync(idlPath, 'utf-8'));
console.log('IDL program address:', idl.address);
