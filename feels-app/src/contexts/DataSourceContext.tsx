'use client';

import React, { createContext, useContext, useState, ReactNode } from 'react';

export type DataSource = 'test' | 'indexer';

interface DataSourceContextType {
  dataSource: DataSource;
  setDataSource: (source: DataSource) => void;
}

const DataSourceContext = createContext<DataSourceContextType | undefined>(undefined);

export function DataSourceProvider({ children }: { children: ReactNode }) {
  const [dataSource, setDataSource] = useState<DataSource>('test');

  return (
    <DataSourceContext.Provider value={{ dataSource, setDataSource }}>
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