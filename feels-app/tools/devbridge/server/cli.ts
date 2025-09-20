#!/usr/bin/env node
import WebSocket from 'ws';
import { createInterface } from 'readline';
import { BridgeMsg } from '../types';
import { randomUUID } from 'crypto';

const DEVBRIDGE_URL = process.env.DEVBRIDGE_URL || 'ws://127.0.0.1:54040';

// Parse command line arguments
const args = process.argv.slice(2);
const command = args[0] || 'tail';

let ws: WebSocket;
let connected = false;
const pendingResults = new Map<string, (result: any) => void>();

function connect(): Promise<void> {
  return new Promise((resolve, reject) => {
    ws = new WebSocket(DEVBRIDGE_URL);

    ws.on('open', () => {
      connected = true;
      // Send hello message
      ws.send(JSON.stringify({ t: 'hello', role: 'cli', version: 1 }));
      resolve();
    });

    ws.on('message', (data) => {
      try {
        const msg = JSON.parse(data.toString()) as BridgeMsg;
        
        switch (msg.t) {
          case 'log':
            if (command === 'tail') {
              const prefix = `[${msg.origin}:${msg.level}]`;
              console.log(prefix, ...msg.msg);
            }
            break;
            
          case 'event':
            if (command === 'tail') {
              console.log(`[event:${msg.name}]`, msg.data || '');
            }
            break;
            
          case 'result':
            const resolver = pendingResults.get(msg.id);
            if (resolver) {
              resolver(msg);
              pendingResults.delete(msg.id);
            }
            break;
        }
      } catch (error) {
        console.error('[devbridge] Error parsing message:', error);
      }
    });

    ws.on('close', () => {
      connected = false;
      console.error('[devbridge] Connection closed');
      process.exit(1);
    });

    ws.on('error', (error) => {
      console.error('[devbridge] Connection error:', error.message);
      reject(error);
    });
  });
}

async function sendCommand(name: string, args?: any): Promise<any> {
  const id = randomUUID();
  
  return new Promise((resolve, reject) => {
    pendingResults.set(id, (result) => {
      if (result.ok) {
        resolve(result.data);
      } else {
        reject(new Error(result.error || 'Command failed'));
      }
    });

    ws.send(JSON.stringify({
      t: 'command',
      id,
      name,
      args
    }));

    // Timeout after 30 seconds
    setTimeout(() => {
      if (pendingResults.has(id)) {
        pendingResults.delete(id);
        reject(new Error('Command timeout'));
      }
    }, 30000);
  });
}

async function main() {
  try {
    await connect();
    
    switch (command) {
      case 'tail':
        console.log('[devbridge] Tailing logs and events... (Ctrl+C to exit)');
        // Keep process alive
        process.stdin.resume();
        break;
        
      case 'run':
        if (args.length < 2) {
          console.error('Usage: devbridge run <command> [args-json]');
          process.exit(1);
        }
        
        const cmdName = args[1];
        const cmdArgs = args[2] ? JSON.parse(args[2]) : undefined;
        
        try {
          const result = await sendCommand(cmdName, cmdArgs);
          console.log(JSON.stringify(result, null, 2));
          process.exit(0);
        } catch (error) {
          console.error('Command failed:', error);
          process.exit(1);
        }
        break;
        
      case 'send':
        if (args.length < 2) {
          console.error('Usage: devbridge send <command> --args <json>');
          process.exit(1);
        }
        
        const sendName = args[1];
        const argsIndex = args.indexOf('--args');
        const sendArgs = argsIndex >= 0 && args[argsIndex + 1] 
          ? JSON.parse(args[argsIndex + 1]) 
          : undefined;
        
        try {
          const result = await sendCommand(sendName, sendArgs);
          console.log('Success:', result);
          process.exit(0);
        } catch (error) {
          console.error('Command failed:', error);
          process.exit(1);
        }
        break;
        
      case 'interactive':
        console.log('[devbridge] Interactive mode. Type commands or "exit" to quit.');
        const rl = createInterface({
          input: process.stdin,
          output: process.stdout,
          prompt: '> '
        });
        
        rl.prompt();
        
        rl.on('line', async (line) => {
          const parts = line.trim().split(' ');
          if (parts[0] === 'exit') {
            process.exit(0);
          }
          
          try {
            const result = await sendCommand(parts[0], parts[1] ? JSON.parse(parts[1]) : undefined);
            console.log('Result:', JSON.stringify(result, null, 2));
          } catch (error) {
            console.error('Error:', error);
          }
          
          rl.prompt();
        });
        break;
        
      default:
        console.error(`Unknown command: ${command}`);
        console.error('Usage: devbridge [tail|run|send|interactive]');
        process.exit(1);
    }
  } catch (error) {
    console.error('Failed to connect:', error);
    process.exit(1);
  }
}

// Handle Ctrl+C gracefully
process.on('SIGINT', () => {
  console.log('\n[devbridge] Exiting...');
  if (ws && connected) {
    ws.close();
  }
  process.exit(0);
});

main();