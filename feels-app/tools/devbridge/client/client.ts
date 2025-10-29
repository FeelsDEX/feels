'use client';

import { useEffect, useRef, useState } from 'react';
import { BridgeMsg } from '../types';

interface DevBridgeAPI {
  sendEvent: (name: string, data?: unknown) => void;
  registerCommand: (name: string, handler: (args?: any) => Promise<any> | any) => void;
  unregisterCommand: (name: string) => void;
  connected: boolean;
}

const DEVBRIDGE_URL = process.env['NEXT_PUBLIC_DEVBRIDGE_URL'] || 'ws://127.0.0.1:54040';

// Command registry
const commandRegistry = new Map<string, (args?: any) => Promise<any> | any>();

export function useDevBridge(): DevBridgeAPI {
  const [connected, setConnected] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const originalConsoleRef = useRef<{
    log: typeof console.log;
    warn: typeof console.warn;
    error: typeof console.error;
  } | null>(null);

  const sendMessage = (msg: BridgeMsg) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(msg));
    }
  };

  const sendEvent = (name: string, data?: unknown) => {
    sendMessage({
      t: 'event',
      name,
      ts: Date.now(),
      data
    });
  };

  const registerCommand = (name: string, handler: (args?: any) => Promise<any> | any) => {
    commandRegistry.set(name, handler);
  };

  const unregisterCommand = (name: string) => {
    commandRegistry.delete(name);
  };

  useEffect(() => {
    // Only connect if explicitly enabled and in development
    const enabled = process.env['NEXT_PUBLIC_DEVBRIDGE_ENABLED'] === 'true' && 
                   process.env.NODE_ENV !== 'production';
    
    if (!enabled) {
      return; // Silently skip if not enabled
    }
    
    // Wrap connection attempt to handle failures gracefully
    const connectToDevBridge = async () => {
      try {
        // Directly attempt WebSocket connection without HTTP pre-check
        // The WebSocket server will return 426 Upgrade Required for HTTP requests
        const ws = new WebSocket(DEVBRIDGE_URL);
        wsRef.current = ws;

        // Add connection timeout
        const connectionTimeout = setTimeout(() => {
          if (ws.readyState === WebSocket.CONNECTING) {
            console.debug('[DevBridge] Connection timeout - server may not be running');
            ws.close();
          }
        }, 2000);

        ws.onopen = () => {
          clearTimeout(connectionTimeout);
          setConnected(true);
          console.info('[DevBridge] Connected to DevBridge server');
          ws.send(JSON.stringify({ t: 'hello', role: 'app', version: 1 }));
        
        // Mirror console methods
        if (!originalConsoleRef.current) {
          originalConsoleRef.current = {
            log: console.log,
            warn: console.warn,
            error: console.error
          };

          const createMirror = (level: 'log' | 'warn' | 'error') => {
            return (...args: unknown[]) => {
              // Call original
              originalConsoleRef.current![level](...args);
              
              // Send to bridge
              sendMessage({
                t: 'log',
                level,
                ts: Date.now(),
                origin: 'browser',
                msg: args
              });
            };
          };

          console.log = createMirror('log');
          console.warn = createMirror('warn');
          console.error = createMirror('error');
        }
      };

      ws.onmessage = async (event) => {
        try {
          const msg = JSON.parse(event.data) as BridgeMsg;
          
          if (msg.t === 'command') {
            const handler = commandRegistry.get(msg.name);
            
            if (!handler) {
              sendMessage({
                t: 'result',
                id: msg.id,
                ok: false,
                error: `Unknown command: ${msg.name}`
              });
              return;
            }

            try {
              const result = await Promise.resolve(handler(msg.args));
              sendMessage({
                t: 'result',
                id: msg.id,
                ok: true,
                data: result
              });
            } catch (error) {
              sendMessage({
                t: 'result',
                id: msg.id,
                ok: false,
                error: String(error)
              });
            }
          }
        } catch (error) {
          console.error('[devbridge] Error handling message:', error);
        }
      };

      ws.onclose = (event) => {
        clearTimeout(connectionTimeout);
        setConnected(false);
        wsRef.current = null;
        
        // Only log if we were previously connected
        if (event.code !== 1000 && event.code !== 1006) {
          console.info('[DevBridge] Disconnected from server');
        }
        
        // Restore original console methods
        if (originalConsoleRef.current) {
          console.log = originalConsoleRef.current.log;
          console.warn = originalConsoleRef.current.warn;
          console.error = originalConsoleRef.current.error;
          originalConsoleRef.current = null;
        }
      };

        ws.onerror = (_event) => {
          // Suppress connection errors - they're expected when DevBridge isn't running
          // The close event will handle cleanup
        };
      } catch (error) {
        // Silently fail - DevBridge is optional
        // Only log in debug mode to avoid console clutter
        if (process.env.NODE_ENV === 'development') {
          console.debug('[DevBridge] Connection attempt failed (this is normal if DevBridge server is not running)');
        }
      }
    };

    // Attempt connection
    connectToDevBridge();

    // Cleanup function
    return () => {
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, []);

  return {
    sendEvent,
    registerCommand,
    unregisterCommand,
    connected
  };
}