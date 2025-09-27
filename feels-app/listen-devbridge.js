#!/usr/bin/env node
// Listen to DevBridge console logs

const WebSocket = require('ws');

const ws = new WebSocket('ws://localhost:54040');

console.log('Attempting to connect to DevBridge on port 54040...');

ws.on('open', () => {
  console.log('✓ Connected to DevBridge - Listening for console logs...\n');
  console.log('Please navigate to http://localhost:3000/token/feelsWojakMvNsD5n2R8rUPzFiHkq9JbgSstPVNkDPGb');
  console.log('and try switching between Linear and Logarithmic axis.\n');
  console.log('Console logs will appear below:\n');
  console.log('=' .repeat(80));
});

ws.on('message', (data) => {
  try {
    const msg = JSON.parse(data.toString());
    
    if (msg.t === 'log') {
      // Format console logs nicely
      const timestamp = new Date().toLocaleTimeString();
      const level = msg.level.toUpperCase().padEnd(5);
      console.log(`[${timestamp}] ${level}`, ...msg.args);
      
      // Look for specific patterns related to axis configuration
      const logStr = msg.args.join(' ');
      if (logStr.includes('axis') || logStr.includes('Axis') || logStr.includes('logarithm') || logStr.includes('log')) {
        console.log('  ^^^ AXIS-RELATED LOG ^^^');
      }
    } else if (msg.t === 'connected') {
      console.log('✓ Browser client connected');
    } else if (msg.t === 'error') {
      console.error('[DevBridge Error]:', msg.message);
    }
  } catch (e) {
    // Ignore parse errors
  }
});

ws.on('error', (error) => {
  console.error('WebSocket error:', error.message);
  console.error('\nMake sure DevBridge server is running.');
  process.exit(1);
});

ws.on('close', () => {
  console.log('\nDisconnected from DevBridge');
  process.exit(0);
});

// Keep the process alive
process.on('SIGINT', () => {
  console.log('\nClosing connection...');
  ws.close();
});
