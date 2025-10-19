#!/usr/bin/env node

const WebSocket = require('ws');

const DEVBRIDGE_WS_URL = 'ws://localhost:54040';

console.log('[DevBridge Vanity Miner Monitor]');
console.log('Connecting to DevBridge...');

const ws = new WebSocket(DEVBRIDGE_WS_URL);

// Track mining state
let miningStartTime = null;
let totalAttempts = 0;
let lastRate = 0;
let workersActive = 0;
let matchFound = false;

ws.on('open', () => {
  console.log('[OK] Connected to DevBridge');
  console.log('Monitoring for vanity miner activity...\n');
  
  // Subscribe to console messages
  ws.send(JSON.stringify({
    type: 'subscribe',
    channel: 'console'
  }));
});

ws.on('message', (data) => {
  try {
    const message = JSON.parse(data.toString());
    
    if (message.type === 'console') {
      const text = message.data.message;
      
      // Monitor key events
      if (text.includes('[Coordinator]')) {
        console.log('[COORD]', text);
        
        if (text.includes('Worker') && text.includes('found match!')) {
          console.log('\n*** MATCH FOUND! ***');
          console.log('Details:', text);
          matchFound = true;
        }
        
        if (text.includes('workers')) {
          const match = text.match(/(\d+) workers/);
          if (match) workersActive = parseInt(match[1]);
        }
      }
      
      if (text.includes('Worker progress:')) {
        const attemptsMatch = text.match(/([\d,]+) attempts/);
        const rateMatch = text.match(/([\d,]+) attempts\/sec/);
        
        if (attemptsMatch && rateMatch) {
          const attempts = parseInt(attemptsMatch[1].replace(/,/g, ''));
          const rate = parseInt(rateMatch[1].replace(/,/g, ''));
          
          totalAttempts = Math.max(totalAttempts, attempts);
          lastRate = rate;
          
          // Display progress every 10 updates
          if (Math.random() < 0.1) {
            console.log(`[Progress] ${attempts.toLocaleString()} attempts | ${rate.toLocaleString()} att/sec | Workers: ${workersActive}`);
          }
        }
      }
      
      if (text.includes('FOUND MATCH!')) {
        console.log('\n\n*** VANITY ADDRESS FOUND! ***');
        console.log('Full message:', text);
        matchFound = true;
        
        // Check if it really ends with 'FEEL'
        const addressMatch = text.match(/([1-9A-HJ-NP-Za-km-z]{32,44})/);
        if (addressMatch) {
          const address = addressMatch[1];
          console.log('Address:', address);
          if (address.endsWith('FEEL')) {
            console.log('[OK] VERIFIED: Address ends with "FEEL"');
          } else {
            console.log('[ERROR] Address does NOT end with "FEEL"');
            console.log(`Actual ending: "${address.slice(-4)}"`);
          }
        }
      }
      
      if (text.includes('Mining stopped:')) {
        console.log('\n[WARN] Mining stopped:', text);
      }
      
      if (text.includes('Mining error:')) {
        console.log('\n[ERROR] Mining error:', text);
      }
      
      if (text.includes('mine_batch') || text.includes('mine_sync')) {
        console.log('[Mining method]', text);
      }
      
      // Monitor for suffix verification
      if (text.includes('Verification:')) {
        console.log('[VERIFY]', text);
        if (text.includes('CORRECT')) {
          console.log('[OK] ADDRESS CORRECTLY ENDS WITH "FEEL"');
          matchFound = true;
        } else if (text.includes('ERROR')) {
          console.log('[ERROR] ADDRESS DOES NOT END WITH "FEEL"');
        }
      }
      
      // Check matching logic
      if (text.includes('check_suffix_match') || text.includes('ends_with')) {
        console.log('[MATCH LOGIC]', text);
      }
    }
  } catch (error) {
    console.error('Error parsing message:', error.message);
  }
});

ws.on('close', () => {
  console.log('\n\nDevBridge connection closed');
  if (matchFound) {
    console.log('[OK] SUCCESS: Match was found during this session');
  } else {
    console.log('[INFO] No match found during this session');
  }
});

ws.on('error', (error) => {
  console.error('DevBridge connection error:', error.message);
  console.log('Make sure DevBridge is running with: just frontend dev');
  process.exit(1);
});

// Status report every 5 seconds
setInterval(() => {
  if (totalAttempts > 0 && !matchFound) {
    console.log(`\n[Status] Total: ${totalAttempts.toLocaleString()} | Rate: ${lastRate.toLocaleString()} att/sec | Expected time: ${(11316496 / lastRate / 60).toFixed(1)} min`);
  }
}, 5000);

// Handle exit
process.on('SIGINT', () => {
  console.log('\n\nClosing monitor...');
  ws.close();
  process.exit(0);
});

console.log('\nPress Ctrl+C to exit\n');
