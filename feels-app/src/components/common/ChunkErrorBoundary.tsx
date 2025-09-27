'use client';

import React from 'react';
import { useRouter } from 'next/navigation';

interface ChunkErrorBoundaryState {
  hasError: boolean;
  error?: Error;
}

export class ChunkErrorBoundary extends React.Component<
  { children: React.ReactNode },
  ChunkErrorBoundaryState
> {
  constructor(props: { children: React.ReactNode }) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(error: Error): ChunkErrorBoundaryState {
    // Check if this is a chunk loading error
    const isChunkError = 
      error.message?.includes('Loading chunk') || 
      error.message?.includes('Failed to fetch dynamically imported module') ||
      error.message?.includes('vendor-chunks');
    
    if (isChunkError) {
      // Try to recover by reloading once
      const reloadKey = `chunk_error_reload_${window.location.pathname}`;
      const hasReloaded = sessionStorage.getItem(reloadKey);
      
      if (!hasReloaded) {
        sessionStorage.setItem(reloadKey, 'true');
        // Clear after 10 seconds to allow future reloads if needed
        setTimeout(() => sessionStorage.removeItem(reloadKey), 10000);
        window.location.reload();
      }
    }
    
    return { hasError: true, error };
  }

  override componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error('ChunkErrorBoundary caught error:', error, errorInfo);
  }

  override render() {
    if (this.state.hasError && this.state.error) {
      // Show a fallback UI for chunk errors
      return (
        <div className="min-h-screen flex items-center justify-center">
          <div className="text-center p-8">
            <h2 className="text-xl font-medium mb-4">Loading Error</h2>
            <p className="text-muted-foreground mb-6">
              There was an issue loading some resources. 
            </p>
            <button
              onClick={() => window.location.reload()}
              className="px-4 py-2 bg-primary text-primary-foreground rounded hover:bg-primary/90"
            >
              Reload Page
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}

// Hook to reset error boundary from child components
export function useResetChunkError() {
  const router = useRouter();
  
  return React.useCallback(() => {
    // Clear any reload flags
    const keys = Object.keys(sessionStorage);
    keys.forEach(key => {
      if (key.startsWith('chunk_error_reload_')) {
        sessionStorage.removeItem(key);
      }
    });
    
    // Navigate to force a fresh load
    router.refresh();
  }, [router]);
}