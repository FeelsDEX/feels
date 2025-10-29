'use client';

import React, { createContext, useContext, useState, useEffect, ReactNode } from 'react';

interface DeveloperModeContextType {
  isDeveloperMode: boolean;
  setDeveloperMode: (enabled: boolean) => void;
}

const DeveloperModeContext = createContext<DeveloperModeContextType | undefined>(undefined);

const STORAGE_KEY = 'feels_developer_mode';

export function DeveloperModeProvider({ children }: { children: ReactNode }) {
  // Default to false in production, true in development
  const defaultMode = process.env.NODE_ENV === 'production' ? false : true;
  const [isDeveloperMode, setIsDeveloperMode] = useState<boolean>(defaultMode);

  // Load from localStorage on mount
  useEffect(() => {
    if (typeof window !== 'undefined') {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored !== null) {
        setIsDeveloperMode(stored === 'true');
      }
    }
  }, []);

  const setDeveloperMode = (enabled: boolean) => {
    setIsDeveloperMode(enabled);
    if (typeof window !== 'undefined') {
      localStorage.setItem(STORAGE_KEY, String(enabled));
    }
  };

  // Expose to window for debugging
  useEffect(() => {
    if (typeof window !== 'undefined') {
      (window as any).__developerMode = { isDeveloperMode, setDeveloperMode };
    }
  }, [isDeveloperMode]);

  return (
    <DeveloperModeContext.Provider value={{ isDeveloperMode, setDeveloperMode }}>
      {children}
    </DeveloperModeContext.Provider>
  );
}

export function useDeveloperMode() {
  const context = useContext(DeveloperModeContext);
  if (context === undefined) {
    throw new Error('useDeveloperMode must be used within a DeveloperModeProvider');
  }
  return context;
}

