'use client';

import { createContext, useContext, useState, ReactNode } from 'react';

interface SearchContextType {
  isTokenSearchModalOpen: boolean;
  setIsTokenSearchModalOpen: (open: boolean) => void;
}

const SearchContext = createContext<SearchContextType | undefined>(undefined);

export function SearchProvider({ children }: { children: ReactNode }) {
  const [isTokenSearchModalOpen, setIsTokenSearchModalOpen] = useState(false);

  return (
    <SearchContext.Provider value={{ isTokenSearchModalOpen, setIsTokenSearchModalOpen }}>
      {children}
    </SearchContext.Provider>
  );
}

export function useSearchContext() {
  const context = useContext(SearchContext);
  if (!context) {
    throw new Error('useSearchContext must be used within a SearchProvider');
  }
  return context;
}