'use client';

import { useState, useEffect, useRef } from 'react';
import { Connection } from '@solana/web3.js';
import { getConnection } from '@/services/connection';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Label } from '@/components/ui/label';
import { useIndexer } from '@/hooks/useIndexer';
import { Database, TestTube, Wifi, WifiOff } from 'lucide-react';
import { useDataSource } from '@/contexts/DataSourceContext';

export function NetworkConnection() {
  const [networkStatus, setNetworkStatus] = useState<'local' | 'devnet' | 'mainnet' | 'unknown'>('local');
  const [isConnected, setIsConnected] = useState(false);
  const [latency, setLatency] = useState<number | null>(null);
  const [blockHeight, setBlockHeight] = useState<number | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const connection = getConnection();
  const connectionRef = useRef<Connection>(connection);
  const { dataSource, setDataSource } = useDataSource();
  
  // Indexer connection status
  const { isConnected: indexerConnected, connectionError: indexerError } = useIndexer();

  // Initialize connection and determine network
  useEffect(() => {
    let mounted = true;
    
    const initConnection = async () => {
      try {
        connectionRef.current = connection;
        
        // Determine network based on endpoint
        const endpoint = connection.rpcEndpoint;
        if (endpoint.includes('localhost') || endpoint.includes('127.0.0.1')) {
          setNetworkStatus('local');
        } else if (endpoint.includes('devnet')) {
          setNetworkStatus('devnet');
        } else if (endpoint.includes('mainnet')) {
          setNetworkStatus('mainnet');
        } else {
          setNetworkStatus('unknown');
        }
        
        // Test connection with timeout
        const startTime = Date.now();
        const versionPromise = connection.getVersion();
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
          const slotPromise = connection.getSlot();
          const slot = await Promise.race([slotPromise, timeoutPromise]);
          if (!mounted) return;
          setBlockHeight(slot as number);
          
        } catch (timeoutError) {
          if (!mounted) return;
          console.warn('Connection test timed out');
          setIsConnected(false);
          setLatency(null);
        }
        
      } catch (error) {
        if (!mounted) return;
        console.warn('RPC connection failed:', error);
        setIsConnected(false);
        setLatency(null);
        setBlockHeight(null);
      } finally {
        if (mounted) {
          setIsLoading(false);
        }
      }
    };

    // Delay initial connection to prevent immediate failures
    const timer = setTimeout(() => {
      initConnection();
    }, 500);
    
    return () => {
      mounted = false;
      clearTimeout(timer);
    };
  }, [connection]);

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
        console.warn('Failed to update status:', error);
      }
    }, 30000); // Update every 30 seconds

    return () => clearInterval(interval);
  }, [isConnected]);

  const handleDataSourceChange = (value: 'test' | 'indexer') => {
    setDataSource(value);
  };

  const getNetworkBadgeVariant = () => {
    switch (networkStatus) {
      case 'local':
        return 'default';
      case 'devnet':
        return 'secondary';
      case 'mainnet':
        return 'outline';
      default:
        return 'destructive';
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-xl">Network Connection</CardTitle>
        <CardDescription className="text-base">
          Connection settings and network status
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Data Source Selection */}
        <div className="space-y-2">
          <Label htmlFor="data-source">Data Source</Label>
          <Select value={dataSource} onValueChange={handleDataSourceChange}>
            <SelectTrigger id="data-source">
              <SelectValue placeholder="Select data source" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="test">
                <div className="flex items-center gap-2">
                  <TestTube className="h-4 w-4" />
                  <span>Test Data</span>
                </div>
              </SelectItem>
              <SelectItem value="indexer">
                <div className="flex items-center gap-2">
                  <Database className="h-4 w-4" />
                  <span>Indexer</span>
                </div>
              </SelectItem>
            </SelectContent>
          </Select>
          {dataSource === 'test' && (
            <p className="text-xs text-muted-foreground">
              Using mock data for development and testing
            </p>
          )}
          {dataSource === 'indexer' && (
            <p className="text-xs text-muted-foreground">
              {indexerConnected 
                ? 'Connected to live indexer for real-time data'
                : indexerError || 'Indexer is not available'
              }
            </p>
          )}
        </div>

        <div className="border-t pt-4 space-y-3">
          {/* RPC Connection Status */}
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium">RPC Status</span>
            <Badge 
              variant="outline"
              className="flex items-center gap-2 cursor-default hover:!bg-transparent"
            >
              {isConnected ? (
                <>
                  <Wifi className="h-3 w-3 text-primary" />
                  <span>Connected</span>
                </>
              ) : isLoading ? (
                <>
                  <div className="h-3 w-3 rounded-full bg-yellow-500 animate-pulse" />
                  <span>Connecting...</span>
                </>
              ) : (
                <>
                  <WifiOff className="h-3 w-3 text-gray-500" />
                  <span>Offline</span>
                </>
              )}
            </Badge>
          </div>

          {/* Network */}
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium">Network</span>
            <Badge 
              variant={getNetworkBadgeVariant()} 
              className={`cursor-default ${
                getNetworkBadgeVariant() === 'default' ? 'hover:!bg-primary' :
                getNetworkBadgeVariant() === 'secondary' ? 'hover:!bg-secondary' :
                getNetworkBadgeVariant() === 'destructive' ? 'hover:!bg-destructive' :
                'hover:!bg-transparent'
              }`}
            >
              {networkStatus === 'local' ? 'Local' : 
               networkStatus === 'devnet' ? 'Devnet' : 
               networkStatus === 'mainnet' ? 'Mainnet' : 
               'Unknown'}
            </Badge>
          </div>

          {/* Indexer Status (only show when indexer is selected) */}
          {dataSource === 'indexer' && (
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">Indexer Status</span>
              <Badge 
                variant={indexerConnected ? "outline" : "destructive"}
                className={`flex items-center gap-2 cursor-default ${
                  indexerConnected ? 'hover:!bg-transparent' : 'hover:!bg-destructive'
                }`}
              >
                {indexerConnected ? (
                  <>
                    <Database className="h-3 w-3 text-primary" />
                    <span>Connected</span>
                  </>
                ) : (
                  <>
                    <Database className="h-3 w-3" />
                    <span>Offline</span>
                  </>
                )}
              </Badge>
            </div>
          )}

          {/* Latency */}
          {latency !== null && isConnected && (
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Latency</span>
              <span className="text-sm font-mono">
                {latency}ms
              </span>
            </div>
          )}

          {/* Block Height */}
          {blockHeight !== null && isConnected && (
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Block Height</span>
              <span className="text-sm font-mono">
                {blockHeight.toLocaleString()}
              </span>
            </div>
          )}

          {/* RPC Endpoint */}
          {connectionRef.current && (
            <div className="space-y-1">
              <span className="text-sm text-muted-foreground">RPC Endpoint</span>
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