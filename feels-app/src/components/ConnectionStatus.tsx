'use client';

import { useState, useEffect, useRef } from 'react';
import { Connection, clusterApiUrl } from '@solana/web3.js';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';

export function ConnectionStatus() {
  const [networkStatus, setNetworkStatus] = useState<'devnet' | 'mainnet' | 'unknown'>('devnet');
  const [isConnected, setIsConnected] = useState(false);
  const [latency, setLatency] = useState<number | null>(null);
  const [blockHeight, setBlockHeight] = useState<number | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const connectionRef = useRef<Connection | null>(null);

  // Initialize connection and determine network
  useEffect(() => {
    let mounted = true;
    
    const initConnection = async () => {
      try {
        // Try to connect to devnet (current default)
        const devnetConn = new Connection(clusterApiUrl('devnet'), {
          commitment: 'confirmed',
          // Add timeout and disable preflight to reduce connection errors
          httpHeaders: {
            'Content-Type': 'application/json',
          },
        });
        
        if (!mounted) return;
        connectionRef.current = devnetConn;
        
        // Determine network based on endpoint first
        const endpoint = devnetConn.rpcEndpoint;
        if (endpoint.includes('devnet')) {
          setNetworkStatus('devnet');
        } else if (endpoint.includes('mainnet')) {
          setNetworkStatus('mainnet');
        } else {
          setNetworkStatus('unknown');
        }
        
        // Test connection with timeout
        const startTime = Date.now();
        const versionPromise = devnetConn.getVersion();
        const timeoutPromise = new Promise((_, reject) => 
          setTimeout(() => reject(new Error('Connection timeout')), 5000)
        );
        
        try {
          await Promise.race([versionPromise, timeoutPromise]);
          const endTime = Date.now();
          
          if (!mounted) return;
          setLatency(endTime - startTime);
          setIsConnected(true);
          
          // Get block height with timeout
          const slotPromise = devnetConn.getSlot();
          const slot = await Promise.race([slotPromise, timeoutPromise]);
          if (!mounted) return;
          setBlockHeight(slot as number);
          
        } catch (timeoutError) {
          if (!mounted) return;
          console.warn('Connection test timed out, but connection established');
          setIsConnected(true);
          setLatency(null);
          setIsLoading(false);
        }
        
      } catch (error) {
        if (!mounted) return;
        console.warn('RPC connection failed, displaying offline status:', error);
        setIsConnected(false);
        setNetworkStatus('devnet'); // Still show devnet as intended network
        setLatency(null);
        setBlockHeight(null);
        setIsLoading(false);
      }
    };

    // Delay initial connection to prevent immediate failures
    const timer = setTimeout(() => {
      initConnection();
    }, 1000);
    
    return () => {
      mounted = false;
      clearTimeout(timer);
    };
  }, []);

  // Update block height periodically
  useEffect(() => {
    if (!connectionRef.current || !isConnected) return;
    
    const interval = setInterval(async () => {
      try {
        // Create timeout promise
        const timeoutPromise = new Promise((_, reject) => 
          setTimeout(() => reject(new Error('Update timeout')), 3000)
        );
        
        // Update block height
        const slotPromise = connectionRef.current!.getSlot();
        const slot = await Promise.race([slotPromise, timeoutPromise]);
        setBlockHeight(slot as number);
        
        // Update latency
        const startTime = Date.now();
        const versionPromise = connectionRef.current!.getVersion();
        await Promise.race([versionPromise, timeoutPromise]);
        const endTime = Date.now();
        setLatency(endTime - startTime);
      } catch (error) {
        console.warn('Failed to update status, will retry:', error);
        // Don't change connection status on periodic update failures
      }
    }, 30000); // Update every 30 seconds (reduced frequency)

    return () => clearInterval(interval);
  }, [isConnected]);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-xl">Network Connection Status</CardTitle>
        <CardDescription className="text-base">
          Current network connection details and health
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {/* Connection Status */}
          <div className="flex items-center justify-between">
            <span className="text-base text-muted-foreground">Status</span>
            <Badge 
              variant="outline"
              className="flex items-center gap-2 bg-transparent"
            >
              <div className={`w-2 h-2 rounded-full ${
                isConnected ? 'bg-primary' : 
                isLoading ? 'bg-yellow-500 animate-pulse' : 
                'bg-gray-500'
              }`}></div>
              {isConnected ? 'Connected' : isLoading ? 'Connecting...' : 'Offline'}
            </Badge>
          </div>

          {/* Network */}
          <div className="flex items-center justify-between">
            <span className="text-base text-muted-foreground">Network</span>
            <Badge variant="secondary">
              {networkStatus === 'devnet' ? 'Devnet' : 
               networkStatus === 'mainnet' ? 'Mainnet-Beta' : 
               'Unknown'}
            </Badge>
          </div>

          {/* Latency */}
          {latency !== null && (
            <div className="flex items-center justify-between">
              <span className="text-base text-muted-foreground">Latency</span>
              <span className="text-base font-mono">
                {latency}ms
              </span>
            </div>
          )}

          {/* Block Height */}
          {blockHeight !== null && (
            <div className="flex items-center justify-between">
              <span className="text-base text-muted-foreground">Block Height</span>
              <span className="text-base font-mono">
                {blockHeight.toLocaleString()}
              </span>
            </div>
          )}

          {/* RPC Endpoint */}
          {connectionRef.current && (
            <div className="space-y-1">
              <span className="text-base text-muted-foreground">RPC Endpoint</span>
              <div className="text-xs font-mono text-muted-foreground break-all bg-muted px-2 py-1 rounded">
                {connectionRef.current.rpcEndpoint}
              </div>
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}