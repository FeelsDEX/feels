'use client';

import { useEffect } from 'react';
import { useRouter, usePathname } from 'next/navigation';
import { useDevBridge } from './client';
import { setupBuiltinCommands } from './commands';

export function DevBridgeProvider({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const pathname = usePathname();
  const { sendEvent, registerCommand, connected } = useDevBridge();

  // Setup built-in commands
  useEffect(() => {
    if (connected) {
      setupBuiltinCommands(router, registerCommand);
      
      // Send initial connected event
      sendEvent('devbridge:connected', {
        timestamp: Date.now(),
        pathname
      });
    }
  }, [connected, router, registerCommand, sendEvent, pathname]);

  // Track route changes
  useEffect(() => {
    if (connected) {
      sendEvent('route:change', {
        pathname,
        timestamp: Date.now()
      });
    }
  }, [pathname, connected, sendEvent]);

  // Track page visibility
  useEffect(() => {
    if (!connected) return;

    const handleVisibilityChange = () => {
      sendEvent('visibility:change', {
        hidden: document.hidden,
        timestamp: Date.now()
      });
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);
    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
    };
  }, [connected, sendEvent]);

  // Track unhandled errors
  useEffect(() => {
    if (!connected) return;

    const handleError = (event: ErrorEvent) => {
      sendEvent('error:unhandled', {
        message: event.message,
        filename: event.filename,
        lineno: event.lineno,
        colno: event.colno,
        error: event.error?.stack || event.error?.toString(),
        timestamp: Date.now()
      });
    };

    const handleRejection = (event: PromiseRejectionEvent) => {
      sendEvent('error:rejection', {
        reason: event.reason?.stack || event.reason?.toString() || event.reason,
        timestamp: Date.now()
      });
    };

    window.addEventListener('error', handleError);
    window.addEventListener('unhandledrejection', handleRejection);

    return () => {
      window.removeEventListener('error', handleError);
      window.removeEventListener('unhandledrejection', handleRejection);
    };
  }, [connected, sendEvent]);

  return <>{children}</>;
}