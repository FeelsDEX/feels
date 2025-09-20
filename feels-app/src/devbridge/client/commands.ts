'use client';

import { useRouter } from 'next/navigation';

type Handler = (args?: any) => Promise<any> | any;

// Feature flags store (example)
const featureFlags = new Map<string, boolean>();

// Built-in command handlers
export function setupBuiltinCommands(
  router: ReturnType<typeof useRouter>,
  registerCommand: (name: string, handler: Handler) => void
) {
  // Ping command for testing
  registerCommand('ping', async () => {
    return { pong: true, timestamp: Date.now() };
  });

  // Toggle feature flag
  registerCommand('toggleFlag', ({ name }: { name: string }) => {
    if (!name) {
      throw new Error('Flag name required');
    }
    const current = featureFlags.get(name) || false;
    featureFlags.set(name, !current);
    return { flag: name, enabled: !current };
  });

  // Get all feature flags
  registerCommand('getFlags', () => {
    const flags: Record<string, boolean> = {};
    featureFlags.forEach((value, key) => {
      flags[key] = value;
    });
    return flags;
  });

  // Navigate to route
  registerCommand('navigate', ({ path }: { path: string }) => {
    if (!path) {
      throw new Error('Path required');
    }
    router.push(path);
    return { navigated: path };
  });

  // Refresh current route
  registerCommand('refresh', () => {
    router.refresh();
    return { refreshed: true };
  });

  // Get current pathname
  registerCommand('getPath', () => {
    return { path: window.location.pathname };
  });

  // Get app info
  registerCommand('appInfo', () => {
    return {
      name: 'Feels App',
      version: process.env.NEXT_PUBLIC_APP_VERSION || '1.0.0',
      env: process.env.NODE_ENV,
      timestamp: Date.now()
    };
  });

  // Clear local storage
  registerCommand('clearStorage', () => {
    localStorage.clear();
    sessionStorage.clear();
    return { cleared: true };
  });

  // Get storage info
  registerCommand('storageInfo', () => {
    return {
      localStorage: {
        keys: Object.keys(localStorage),
        size: Object.keys(localStorage).length
      },
      sessionStorage: {
        keys: Object.keys(sessionStorage),
        size: Object.keys(sessionStorage).length
      }
    };
  });

  // Trigger a test event
  registerCommand('testEvent', ({ message }: { message?: string }) => {
    window.dispatchEvent(new CustomEvent('devbridge:test', {
      detail: { message: message || 'Test event triggered' }
    }));
    return { eventTriggered: true, message };
  });

  // Get window dimensions
  registerCommand('windowInfo', () => {
    return {
      innerWidth: window.innerWidth,
      innerHeight: window.innerHeight,
      outerWidth: window.outerWidth,
      outerHeight: window.outerHeight,
      devicePixelRatio: window.devicePixelRatio,
      screenWidth: window.screen.width,
      screenHeight: window.screen.height
    };
  });

  // Simulate wallet connection (for testing)
  registerCommand('simulateWallet', ({ connected }: { connected: boolean }) => {
    window.dispatchEvent(new CustomEvent('devbridge:wallet', {
      detail: { connected }
    }));
    return { walletSimulation: connected };
  });

  // Get performance metrics
  registerCommand('perfMetrics', () => {
    const nav = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming;
    return {
      domContentLoaded: nav?.domContentLoadedEventEnd - nav?.domContentLoadedEventStart,
      loadComplete: nav?.loadEventEnd - nav?.loadEventStart,
      responseTime: nav?.responseEnd - nav?.fetchStart,
      renderTime: nav?.domComplete - nav?.domInteractive
    };
  });

  // Console log levels control
  registerCommand('setLogLevel', ({ level }: { level: 'all' | 'warn' | 'error' | 'none' }) => {
    // This would integrate with your logging system
    return { logLevel: level };
  });
}

// Export feature flags for app use
export function getFeatureFlag(name: string): boolean {
  return featureFlags.get(name) || false;
}

export function getAllFeatureFlags(): Record<string, boolean> {
  const flags: Record<string, boolean> = {};
  featureFlags.forEach((value, key) => {
    flags[key] = value;
  });
  return flags;
}