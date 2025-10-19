#!/usr/bin/env node

const WebSocket = require('ws');
const chalk = require('chalk').default || require('chalk');

const DEVBRIDGE_WS_URL = 'ws://localhost:8765';

console.log(chalk.cyan('[DevBridge Vanity Miner Monitor]'));
console.log(chalk.gray('Connecting to DevBridge...'));

const ws = new WebSocket(DEVBRIDGE_WS_URL);

// Track mining state
let miningStartTime = null;
let totalAttempts = 0;
let lastRate = 0;
let workersActive = 0;

ws.on('open', () => {
  console.log(chalk.green('[OK] Connected to DevBridge'));
  console.log(chalk.yellow('Monitoring for vanity miner activity...\n'));
  
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
        console.log(chalk.blue('Coordinator:'), text);
        
        if (text.includes('Worker') && text.includes('found match!')) {
          console.log(chalk.green.bold('\n*** MATCH FOUND! ***'));
          console.log(chalk.green('Details:'), text);
        }
        
        if (text.includes('workers')) {
          const match = text.match(/(\d+) workers/);
          if (match) workersActive = parseInt(match[1]);
        }
      }
      
      if (text.includes('Worker progress:')) {
        const attemptsMatch = text.match(/(\d+,?\d*) attempts/);
        const rateMatch = text.match(/(\d+,?\d*) attempts\/sec/);
        
        if (attemptsMatch && rateMatch) {
          const attempts = parseInt(attemptsMatch[1].replace(/,/g, ''));
          const rate = parseInt(rateMatch[1].replace(/,/g, ''));
          
          totalAttempts = Math.max(totalAttempts, attempts);
          lastRate = rate;
          
          // Display progress
          process.stdout.write(chalk.gray(`\r[Progress] ${attempts.toLocaleString()} attempts | ${rate.toLocaleString()} att/sec | Workers: ${workersActive}`));
        }
      }
      
      if (text.includes('FOUND MATCH!')) {
        console.log(chalk.green.bold('\n\n*** VANITY ADDRESS FOUND! ***'));
        console.log(chalk.green('Address:'), text);
        
        // Check if it really ends with 'FEEL'
        const addressMatch = text.match(/([A-Za-z0-9]{44})/);
        if (addressMatch) {
          const address = addressMatch[1];
          if (address.endsWith('FEEL')) {
            console.log(chalk.green.bold('[OK] VERIFIED: Address ends with "FEEL"'));
          } else {
            console.log(chalk.red.bold('[ERROR] Address does NOT end with "FEEL"'));
            console.log(chalk.red(`Actual ending: "${address.slice(-4)}"`));
          }
        }
      }
      
      if (text.includes('Mining stopped:')) {
        console.log(chalk.yellow('\n[WARN] Mining stopped:'), text);
      }
      
      if (text.includes('Mining error:')) {
        console.log(chalk.red('\n[ERROR] Mining error:'), text);
      }
      
      if (text.includes('mine_batch') || text.includes('mine_sync')) {
        console.log(chalk.gray('Mining method:'), text);
      }
      
      // Monitor for suffix verification
      if (text.includes('Verification:')) {
        if (text.includes('CORRECT')) {
          console.log(chalk.green('\n[OK]'), text);
        } else if (text.includes('ERROR')) {
          console.log(chalk.red('\n[ERROR]'), text);
        }
      }
    }
  } catch (error) {
    console.error(chalk.red('Error parsing message:'), error);
  }
});

ws.on('close', () => {
  console.log(chalk.yellow('\n\nDevBridge connection closed'));
});

ws.on('error', (error) => {
  console.error(chalk.red('DevBridge connection error:'), error.message);
  console.log(chalk.yellow('Make sure DevBridge is running with: just frontend dev'));
});

// Handle exit
process.on('SIGINT', () => {
  console.log(chalk.gray('\n\nClosing monitor...'));
  ws.close();
  process.exit(0);
});

console.log(chalk.gray('\nPress Ctrl+C to exit\n'));
