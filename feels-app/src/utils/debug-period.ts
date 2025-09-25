export function debugPeriodChange(period: any) {
  // Only log in development
  if (process.env.NODE_ENV === 'production') return;
  
  const timestamp = new Date().toISOString();
  const logEntry = {
    timestamp,
    period: {
      text: period.text,
      multiplier: period.multiplier,
      timespan: period.timespan
    }
  };
  
  // Log to console with DevBridge
  console.log('[PriceChart] Period Debug:', logEntry);
  
  // Also store in localStorage for inspection
  if (typeof window !== 'undefined') {
    const logs = JSON.parse(localStorage.getItem('period-debug-logs') || '[]');
    logs.push(logEntry);
    // Keep only last 10 entries
    if (logs.length > 10) logs.shift();
    localStorage.setItem('period-debug-logs', JSON.stringify(logs));
  }
}