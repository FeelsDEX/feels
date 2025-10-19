import { useState, useEffect, useCallback } from 'react';
import { 
  createIndexerClient, 
  IndexedMarket, 
  IndexedSwap, 
  ProtocolStats,
  MarketStats,
  IndexedFloor
} from '@/services/indexer';
import { useDataSource } from '@/contexts/DataSourceContext';

export interface UseIndexerOptions {
  baseUrl?: string;
  refreshInterval?: number;
  enabled?: boolean;
}

export interface IndexerState<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
  lastUpdated: number | null;
}

export function useIndexer(options: UseIndexerOptions = {}) {
  const { dataSource } = useDataSource();
  const [client] = useState(() => createIndexerClient(options.baseUrl));
  const [isConnected, setIsConnected] = useState(false);
  const [connectionError, setConnectionError] = useState<string | null>(null);

  // Health check to verify indexer connection
  const checkHealth = useCallback(async () => {
    // Only check health if we're in indexer mode
    if (dataSource !== 'indexer') {
      setIsConnected(false);
      setConnectionError('Using test data mode');
      return;
    }
    
    try {
      await client.getHealth();
      setIsConnected(true);
      setConnectionError(null);
    } catch (error) {
      setIsConnected(false);
      setConnectionError(error instanceof Error ? error.message : 'Connection failed');
    }
  }, [client, dataSource]);

  useEffect(() => {
    if (options.enabled !== false && dataSource === 'indexer') {
      checkHealth();
      
      // Set up periodic health checks
      const interval = setInterval(checkHealth, 30000); // Check every 30 seconds
      return () => clearInterval(interval);
    } else if (dataSource === 'test') {
      // In test mode, don't check health
      setIsConnected(false);
      setConnectionError('Using test data mode');
    }
    // Return undefined for other cases
    return undefined;
  }, [checkHealth, options.enabled, dataSource]);

  return {
    client,
    isConnected,
    connectionError,
    checkHealth,
  };
}

export function useProtocolStats(options: UseIndexerOptions = {}) {
  const { dataSource } = useDataSource();
  const { client, isConnected } = useIndexer(options);
  const [state, setState] = useState<IndexerState<ProtocolStats>>({
    data: null,
    loading: false,
    error: null,
    lastUpdated: null,
  });

  const fetchStats = useCallback(async () => {
    // Only fetch from indexer if we're in indexer mode
    if (!isConnected || dataSource !== 'indexer') return;

    setState(prev => ({ ...prev, loading: true, error: null }));
    
    try {
      const data = await client.getProtocolStats();
      setState({
        data,
        loading: false,
        error: null,
        lastUpdated: Date.now(),
      });
    } catch (error) {
      setState(prev => ({
        ...prev,
        loading: false,
        error: error instanceof Error ? error.message : 'Failed to fetch protocol stats',
      }));
    }
  }, [client, isConnected, dataSource]);

  useEffect(() => {
    if (options.enabled !== false && isConnected && dataSource === 'indexer') {
      fetchStats();
      
      // Set up auto-refresh
      const interval = options.refreshInterval || 10000; // Default 10 seconds
      const timer = setInterval(fetchStats, interval);
      return () => clearInterval(timer);
    }
    // Return undefined for other cases
    return undefined;
  }, [fetchStats, isConnected, options.enabled, options.refreshInterval, dataSource]);

  return {
    ...state,
    refetch: fetchStats,
  };
}

export function useMarkets(options: UseIndexerOptions = {}) {
  const { dataSource, isIndexerAvailable } = useDataSource();
  const { client, isConnected } = useIndexer(options);
  const [state, setState] = useState<IndexerState<IndexedMarket[]>>({
    data: null,
    loading: false,
    error: null,
    lastUpdated: null,
  });

  const fetchMarkets = useCallback(async () => {
    // Only fetch from indexer if we're in indexer mode and it's actually available
    if (!isConnected || dataSource !== 'indexer' || !isIndexerAvailable) {
      // Set empty data immediately for unavailable indexer
      setState({
        data: [],
        loading: false,
        error: null,
        lastUpdated: Date.now(),
      });
      return;
    }

    setState(prev => ({ ...prev, loading: true, error: null }));
    
    try {
      const data = await client.getMarkets();
      setState({
        data,
        loading: false,
        error: null,
        lastUpdated: Date.now(),
      });
    } catch (error) {
      // Set empty data on error rather than showing error state
      setState({
        data: [],
        loading: false,
        error: null,
        lastUpdated: Date.now(),
      });
    }
  }, [client, isConnected, dataSource, isIndexerAvailable]);

  useEffect(() => {
    if (options.enabled !== false && dataSource === 'indexer') {
      // Only start fetching if indexer is available
      if (isIndexerAvailable && isConnected) {
        fetchMarkets();
        
        // Set up auto-refresh
        const interval = options.refreshInterval || 15000; // Default 15 seconds
        const timer = setInterval(fetchMarkets, interval);
        return () => clearInterval(timer);
      } else {
        // Set empty data immediately if indexer isn't available
        setState({
          data: [],
          loading: false,
          error: null,
          lastUpdated: Date.now(),
        });
      }
    }
    // Return undefined for other cases
    return undefined;
  }, [fetchMarkets, isConnected, options.enabled, options.refreshInterval, dataSource, isIndexerAvailable]);

  return {
    ...state,
    refetch: fetchMarkets,
  };
}

export function useMarketData(marketAddress: string, options: UseIndexerOptions = {}) {
  const { dataSource } = useDataSource();
  const { client, isConnected } = useIndexer(options);
  const [marketState, setMarketState] = useState<IndexerState<IndexedMarket>>({
    data: null,
    loading: false,
    error: null,
    lastUpdated: null,
  });
  const [statsState, setStatsState] = useState<IndexerState<MarketStats>>({
    data: null,
    loading: false,
    error: null,
    lastUpdated: null,
  });
  const [floorState, setFloorState] = useState<IndexerState<IndexedFloor>>({
    data: null,
    loading: false,
    error: null,
    lastUpdated: null,
  });

  const fetchMarketData = useCallback(async () => {
    // Only fetch from indexer if we're in indexer mode
    if (!isConnected || !marketAddress || dataSource !== 'indexer') return;

    // Fetch market details
    setMarketState(prev => ({ ...prev, loading: true, error: null }));
    try {
      const market = await client.getMarket(marketAddress);
      setMarketState({
        data: market,
        loading: false,
        error: null,
        lastUpdated: Date.now(),
      });
    } catch (error) {
      setMarketState(prev => ({
        ...prev,
        loading: false,
        error: error instanceof Error ? error.message : 'Failed to fetch market',
      }));
    }

    // Fetch market stats
    setStatsState(prev => ({ ...prev, loading: true, error: null }));
    try {
      const stats = await client.getMarketStats(marketAddress);
      setStatsState({
        data: stats,
        loading: false,
        error: null,
        lastUpdated: Date.now(),
      });
    } catch (error) {
      setStatsState(prev => ({
        ...prev,
        loading: false,
        error: error instanceof Error ? error.message : 'Failed to fetch market stats',
      }));
    }

    // Fetch floor data
    setFloorState(prev => ({ ...prev, loading: true, error: null }));
    try {
      const floor = await client.getMarketFloor(marketAddress);
      setFloorState({
        data: floor,
        loading: false,
        error: null,
        lastUpdated: Date.now(),
      });
    } catch (error) {
      setFloorState(prev => ({
        ...prev,
        loading: false,
        error: error instanceof Error ? error.message : 'Failed to fetch floor data',
      }));
    }
  }, [client, isConnected, marketAddress, dataSource]);

  useEffect(() => {
    if (options.enabled !== false && isConnected && marketAddress && dataSource === 'indexer') {
      fetchMarketData();
      
      // Set up auto-refresh
      const interval = options.refreshInterval || 5000; // Default 5 seconds for market data
      const timer = setInterval(fetchMarketData, interval);
      return () => clearInterval(timer);
    }
    // Return undefined for other cases
    return undefined;
  }, [fetchMarketData, isConnected, marketAddress, options.enabled, options.refreshInterval, dataSource]);

  return {
    market: marketState,
    stats: statsState,
    floor: floorState,
    refetch: fetchMarketData,
  };
}

export function useMarketSwaps(
  marketAddress: string, 
  options: UseIndexerOptions & { limit?: number } = {}
) {
  const { dataSource } = useDataSource();
  const { client, isConnected } = useIndexer(options);
  const [state, setState] = useState<IndexerState<IndexedSwap[]>>({
    data: null,
    loading: false,
    error: null,
    lastUpdated: null,
  });

  const fetchSwaps = useCallback(async () => {
    // Only fetch from indexer if we're in indexer mode
    if (!isConnected || !marketAddress || dataSource !== 'indexer') return;

    setState(prev => ({ ...prev, loading: true, error: null }));
    
    try {
      const data = await client.getMarketSwaps(marketAddress, {
        limit: options.limit || 50,
      });
      setState({
        data,
        loading: false,
        error: null,
        lastUpdated: Date.now(),
      });
    } catch (error) {
      setState(prev => ({
        ...prev,
        loading: false,
        error: error instanceof Error ? error.message : 'Failed to fetch swaps',
      }));
    }
  }, [client, isConnected, marketAddress, options.limit, dataSource]);

  useEffect(() => {
    if (options.enabled !== false && isConnected && marketAddress && dataSource === 'indexer') {
      fetchSwaps();
      
      // Set up auto-refresh
      const interval = options.refreshInterval || 3000; // Default 3 seconds for recent swaps
      const timer = setInterval(fetchSwaps, interval);
      return () => clearInterval(timer);
    }
    // Return undefined for other cases
    return undefined;
  }, [fetchSwaps, isConnected, marketAddress, options.enabled, options.refreshInterval, dataSource]);

  return {
    ...state,
    refetch: fetchSwaps,
  };
}
