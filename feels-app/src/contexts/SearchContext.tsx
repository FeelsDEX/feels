'use client';

import { createContext, useContext, useState, ReactNode, useEffect } from 'react';
import { usePathname } from 'next/navigation';

export enum SearchContextArea {
  TOKENS = 'tokens',
  DOCS = 'docs',
  ALL = 'all'
}

interface SearchContextType {
  isTokenSearchModalOpen: boolean;
  setIsTokenSearchModalOpen: (open: boolean) => void;
  searchArea: SearchContextArea;
  setSearchArea: (area: SearchContextArea) => void;
}

const SearchContext = createContext<SearchContextType | undefined>(undefined);

export function SearchProvider({ children }: { children: ReactNode }) {
  const pathname = usePathname();
  const [isTokenSearchModalOpen, setIsTokenSearchModalOpen] = useState(false);
  const [searchArea, setSearchArea] = useState<SearchContextArea>(SearchContextArea.TOKENS);

  // Auto-detect search area based on route
  useEffect(() => {
    if (pathname.startsWith('/docs')) {
      setSearchArea(SearchContextArea.DOCS);
    } else {
      setSearchArea(SearchContextArea.TOKENS);
    }
  }, [pathname]);

  return (
    <SearchContext.Provider value={{ 
      isTokenSearchModalOpen, 
      setIsTokenSearchModalOpen,
      searchArea,
      setSearchArea
    }}>
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