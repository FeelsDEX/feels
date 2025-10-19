#!/usr/bin/env node

/**
 * Cleanup utility for DevBridge processes
 * Kills any existing DevBridge servers to prevent port conflicts
 */

const { execSync } = require('child_process');

function killDevBridgeProcesses() {
  try {
    // Find all DevBridge processes
    const processes = execSync('ps aux | grep devbridge/server/server.ts | grep -v grep', { encoding: 'utf8' });
    
    if (processes.trim()) {
      console.log('[devbridge] Found existing DevBridge processes:');
      console.log(processes);
      
      // Extract PIDs and kill them
      const lines = processes.trim().split('\n');
      for (const line of lines) {
        const parts = line.trim().split(/\s+/);
        const pid = parts[1];
        if (pid && !isNaN(parseInt(pid))) {
          try {
            execSync(`kill ${pid}`, { stdio: 'ignore' });
            console.log(`[devbridge] Killed process ${pid}`);
          } catch (error) {
            console.log(`[devbridge] Could not kill process ${pid} (may already be dead)`);
          }
        }
      }
    } else {
      console.log('[devbridge] No existing DevBridge processes found');
    }
  } catch (error) {
    // No processes found or other error - that's fine
    console.log('[devbridge] No existing DevBridge processes found');
  }
}

// Also check for processes using the default port range
function killProcessesOnPorts() {
  for (let port = 54040; port < 54050; port++) {
    try {
      const result = execSync(`lsof -ti:${port}`, { encoding: 'utf8' });
      if (result.trim()) {
        const pid = result.trim();
        try {
          execSync(`kill ${pid}`, { stdio: 'ignore' });
          console.log(`[devbridge] Killed process ${pid} using port ${port}`);
        } catch (error) {
          console.log(`[devbridge] Could not kill process ${pid} on port ${port}`);
        }
      }
    } catch (error) {
      // No process on this port - that's fine
    }
  }
}

if (require.main === module) {
  console.log('[devbridge] Cleaning up existing DevBridge processes...');
  killDevBridgeProcesses();
  killProcessesOnPorts();
  console.log('[devbridge] Cleanup complete');
}

module.exports = { killDevBridgeProcesses, killProcessesOnPorts };