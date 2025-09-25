import { useEffect } from 'react';

export function usePeriodDebug() {
  useEffect(() => {
    // Expose debug function globally for DevBridge
    if (typeof window !== 'undefined') {
      (window as any).getPeriodDebugLogs = () => {
        const logs = localStorage.getItem('period-debug-logs');
        return logs ? JSON.parse(logs) : [];
      };
      
      (window as any).clearPeriodDebugLogs = () => {
        localStorage.removeItem('period-debug-logs');
        console.log('[PriceChart] Period debug logs cleared');
      };
    }
  }, []);
}