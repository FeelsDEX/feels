import { WebSocketServer, WebSocket } from 'ws';
import { IncomingMessage } from 'http';
import net from 'net';
import { BridgeMsg, ClientSocket } from '../types';

const DEVBRIDGE_PORT_START = 54040;
const DEVBRIDGE_HOST = '127.0.0.1';

// Only start if enabled
if (process.env['DEVBRIDGE_ENABLED'] !== 'true') {
  console.log('[devbridge] Server disabled (DEVBRIDGE_ENABLED != true)');
  process.exit(0);
}

// Function to check if a port is available
function isPortAvailable(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const server = net.createServer();
    server.listen(port, DEVBRIDGE_HOST, () => {
      server.close(() => resolve(true));
    });
    server.on('error', () => resolve(false));
  });
}

// Function to find an available port starting from the default
async function findAvailablePort(startPort: number, maxAttempts: number = 10): Promise<number> {
  for (let i = 0; i < maxAttempts; i++) {
    const port = startPort + i;
    if (await isPortAvailable(port)) {
      return port;
    }
  }
  throw new Error(`No available ports found in range ${startPort}-${startPort + maxAttempts - 1}`);
}

// Find available port and start server
async function startServer() {
  try {
    const port = await findAvailablePort(DEVBRIDGE_PORT_START);
    
    const wss = new WebSocketServer({
      port,
      host: DEVBRIDGE_HOST,
    });

    const clients = new Map<string, ClientSocket>();
    let clientCounter = 0;

    console.log(`[devbridge] Server running on ws://${DEVBRIDGE_HOST}:${port}`);

    setupWebSocketServer(wss, clients, clientCounter);
  } catch (error) {
    console.error('[devbridge] Failed to start server:', error);
    process.exit(1);
  }
}

function setupWebSocketServer(wss: WebSocketServer, clients: Map<string, ClientSocket>, clientCounter: number) {
  wss.on('connection', (ws: WebSocket, req: IncomingMessage) => {
    // Only accept loopback connections for security
    const remoteAddress = req.socket.remoteAddress;
    if (remoteAddress !== '127.0.0.1' && remoteAddress !== '::1') {
      console.log(`[devbridge] Rejected non-loopback connection from ${remoteAddress}`);
      ws.close(1008, 'Only loopback connections allowed');
      return;
    }

    const clientId = `client-${++clientCounter}`;
    const client: ClientSocket = { id: clientId, role: 'app', socket: ws };
    clients.set(clientId, client);
    
    console.log(`[devbridge] Client ${clientId} connected`);

    ws.on('message', (data: Buffer) => {
      try {
        const msg = JSON.parse(data.toString()) as BridgeMsg;
        
        // Handle hello messages to set client role
        if (msg.t === 'hello') {
          client.role = msg.role;
          console.log(`[devbridge] Client ${clientId} identified as ${msg.role}`);
          return;
        }

        // Route messages based on type
        switch (msg.t) {
          case 'log':
          case 'event':
            // Forward logs/events from app to all CLI clients
            if (client.role === 'app') {
              broadcastToRole('cli', msg, clients);
            }
            break;
            
          case 'command':
            // Forward commands from CLI to all app clients
            if (client.role === 'cli') {
              broadcastToRole('app', msg, clients);
              // Store command originator for result routing
              (client.socket as any).pendingCommands = (client.socket as any).pendingCommands || new Set();
              (client.socket as any).pendingCommands.add((msg as any).id);
            }
            break;
            
          case 'result':
            // Route results back to originating CLI
            routeResultToOriginator(msg, clients);
            break;
        }
      } catch (error) {
        console.error(`[devbridge] Error processing message from ${clientId}:`, error);
      }
    });

    ws.on('close', () => {
      console.log(`[devbridge] Client ${clientId} disconnected`);
      clients.delete(clientId);
    });

    ws.on('error', (error: Error) => {
      console.error(`[devbridge] Client ${clientId} error:`, error);
    });
  });

  // Handle graceful shutdown
  process.on('SIGINT', () => {
    console.log('\n[devbridge] Shutting down server...');
    wss.close(() => {
      process.exit(0);
    });
  });

  process.on('SIGTERM', () => {
    console.log('\n[devbridge] Shutting down server...');
    wss.close(() => {
      process.exit(0);
    });
  });
}

function broadcastToRole(role: 'cli' | 'app', msg: BridgeMsg, clients: Map<string, ClientSocket>) {
  const message = JSON.stringify(msg);
  for (const [, client] of clients) {
    if (client.role === role && client.socket.readyState === WebSocket.OPEN) {
      client.socket.send(message);
    }
  }
}

function routeResultToOriginator(msg: BridgeMsg & { t: 'result' }, clients: Map<string, ClientSocket>) {
  // Find CLI client that sent the command
  for (const [, client] of clients) {
    if (client.role === 'cli' && (client.socket as any).pendingCommands?.has(msg.id)) {
      if (client.socket.readyState === WebSocket.OPEN) {
        client.socket.send(JSON.stringify(msg));
        (client.socket as any).pendingCommands.delete(msg.id);
      }
      break;
    }
  }
}

// Start the server
startServer();