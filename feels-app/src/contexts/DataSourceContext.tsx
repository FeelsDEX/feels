'use client';

import React, { createContext, useContext, useState, useEffect, ReactNode } from 'react';

export type DataSource = 'test' | 'indexer';

interface DataSourceContextType {
  dataSource: DataSource;
  setDataSource: (source: DataSource) => void;
  isIndexerAvailable: boolean;
  isUsingFallback: boolean;
}

const DataSourceContext = createContext<DataSourceContextType | undefined>(undefined);

// Check if indexer should be used based on environment
const shouldUseIndexer = (): boolean => {
  // Check if we're in a local development environment with indexer enabled
  const useIndexer = process.env['NEXT_PUBLIC_USE_INDEXER'] === 'true';
  const indexerUrl = process.env['NEXT_PUBLIC_INDEXER_URL'];
  const isLocalDev = indexerUrl?.includes('localhost') ?? false;
  
  return useIndexer && isLocalDev;
};

export function DataSourceProvider({ children }: { children: ReactNode }) {
  const [dataSource, setDataSource] = useState<DataSource>(() => 
    shouldUseIndexer() ? 'indexer' : 'test'
  );
  const [isIndexerAvailable, setIsIndexerAvailable] = useState(false);
  const [isUsingFallback, setIsUsingFallback] = useState(false);

  // Check indexer availability on mount
  useEffect(() => {
    const checkIndexerHealth = async () => {
      if (!shouldUseIndexer()) {
        setIsIndexerAvailable(false);
        return;
      }

      try {
        // Use Next.js proxy to avoid CORS issues in development
        const indexerUrl = '/api/indexer';
        const response = await fetch(`${indexerUrl}/health`, {
          signal: AbortSignal.timeout(5000),
        });
        
        if (response.ok) {
          const health = await response.json();
          setIsIndexerAvailable(health.status === 'ok' || health.status === 'healthy');
          
          // If indexer is available and we should use it, switch to indexer mode
          if (health.status === 'ok' || health.status === 'healthy') {
            setDataSource('indexer');
            setIsUsingFallback(false);
          }
        } else {
          setIsIndexerAvailable(false);
          setDataSource('test');
          setIsUsingFallback(true);
        }
      } catch (error) {
        console.log('Indexer not available, using test data:', error instanceof Error ? error.message : 'Unknown error');
        setIsIndexerAvailable(false);
        setDataSource('test');
        setIsUsingFallback(true);
      }
    };

    checkIndexerHealth();
  }, []);

  // Expose context to window for debugging
  useEffect(() => {
    if (typeof window !== 'undefined') {
      (window as any).__dataSourceContext = { dataSource, setDataSource, isIndexerAvailable, isUsingFallback };
    }
  }, [dataSource, isIndexerAvailable, isUsingFallback]);

  return (
    <DataSourceContext.Provider value={{ dataSource, setDataSource, isIndexerAvailable, isUsingFallback }}>
      {children}
    </DataSourceContext.Provider>
  );
}

export function useDataSource() {
  const context = useContext(DataSourceContext);
  if (context === undefined) {
    throw new Error('useDataSource must be used within a DataSourceProvider');
  }
  return context;
}