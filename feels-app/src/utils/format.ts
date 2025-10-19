// Consolidated formatting utilities for the Feels Protocol frontend
// Combines number, date, and metrics formatting in one place

/**
 * Format numbers with smart precision based on magnitude
 */
export function formatNumber(value: number, decimals?: number): string {
  if (decimals !== undefined) {
    return value.toFixed(decimals);
  }

  // Smart formatting based on value magnitude
  if (value === 0) return '0';

  const absValue = Math.abs(value);

  if (absValue < 0.000001) return value.toExponential(2);
  if (absValue < 0.00001) return value.toFixed(6);
  if (absValue < 0.0001) return value.toFixed(5);
  if (absValue < 0.001) return value.toFixed(4);
  if (absValue < 0.01) return value.toFixed(4);
  if (absValue < 0.1) return value.toFixed(3);
  if (absValue < 1) return value.toFixed(3);

  if (absValue < 10) {
    const decimals = value % 1 === 0 ? 0 :
                    Math.abs(value * 10 % 1) < 0.01 ? 1 : 2;
    return value.toFixed(decimals);
  }

  if (absValue < 100) {
    const decimals = value % 1 === 0 ? 0 : 1;
    return value.toFixed(decimals);
  }

  if (absValue < 1000) {
    return value.toFixed(0);
  }

  if (absValue >= 1000000) {
    const millions = value / 1000000;
    return millions.toFixed(millions < 10 ? 1 : 0) + 'M';
  }

  if (absValue >= 1000) {
    const thousands = value / 1000;
    return thousands.toFixed(thousands < 10 ? 1 : 0) + 'K';
  }

  return value.toFixed(0);
}

/**
 * Format date in human-readable format
 */
export function formatDate(date: Date | string): string {
  const dateObj = typeof date === 'string' ? new Date(date) : date;
  
  return new Intl.DateTimeFormat('en-US', {
    year: 'numeric',
    month: 'long',
    day: 'numeric'
  }).format(dateObj);
}

/**
 * Format percentage with sign
 */
export function formatPercent(value: number, decimals: number = 2): string {
  const sign = value > 0 ? '+' : '';
  return `${sign}${value.toFixed(decimals)}%`;
}

/**
 * Format currency with appropriate precision
 */
export function formatCurrency(value: number, decimals?: number): string {
  return `$${formatNumber(value, decimals)}`;
}

/**
 * Format market cap with K/M/B suffixes
 */
export function formatMarketCap(value: number): string {
  if (value >= 1e9) {
    return `$${(value / 1e9).toFixed(2)}B`;
  }
  if (value >= 1e6) {
    return `$${(value / 1e6).toFixed(2)}M`;
  }
  if (value >= 1e3) {
    return `$${(value / 1e3).toFixed(2)}K`;
  }
  return `$${value.toFixed(2)}`;
}

/**
 * Format volume with appropriate precision
 */
export function formatVolume(value: number): string {
  return formatMarketCap(value); // Same logic as market cap
}

/**
 * Format large numbers compactly
 */
export function formatCompactNumber(value: number): string {
  if (value >= 1e9) return `${(value / 1e9).toFixed(1)}B`;
  if (value >= 1e6) return `${(value / 1e6).toFixed(1)}M`;
  if (value >= 1e3) return `${(value / 1e3).toFixed(1)}K`;
  return value.toFixed(0);
}

/**
 * Format metrics for display
 */
export function formatMetric(value: number | string | undefined, type: 'currency' | 'percent' | 'number' = 'number'): string {
  if (value === undefined || value === null) {
    return type === 'currency' ? '$0' : '0';
  }

  const numValue = typeof value === 'string' ? parseFloat(value) : value;
  
  if (isNaN(numValue)) {
    return type === 'currency' ? '$0' : '0';
  }

  switch (type) {
    case 'currency':
      return formatCurrency(numValue);
    case 'percent':
      return formatPercent(numValue);
    case 'number':
      return formatNumber(numValue);
    default:
      return String(value);
  }
}

/**
 * Format relative time (e.g., "2 hours ago")
 */
export function formatRelativeTime(date: Date | string): string {
  const dateObj = typeof date === 'string' ? new Date(date) : date;
  const now = new Date();
  const diffMs = now.getTime() - dateObj.getTime();
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHour / 24);
  const diffWeek = Math.floor(diffDay / 7);
  const diffMonth = Math.floor(diffDay / 30);

  if (diffSec < 60) return 'just now';
  if (diffMin < 60) return `${diffMin} minute${diffMin > 1 ? 's' : ''} ago`;
  if (diffHour < 24) return `${diffHour} hour${diffHour > 1 ? 's' : ''} ago`;
  if (diffDay < 7) return `${diffDay} day${diffDay > 1 ? 's' : ''} ago`;
  if (diffWeek < 4) return `${diffWeek} week${diffWeek > 1 ? 's' : ''} ago`;
  if (diffMonth < 12) return `${diffMonth} month${diffMonth > 1 ? 's' : ''} ago`;
  
  const diffYear = Math.floor(diffDay / 365);
  return `${diffYear} year${diffYear > 1 ? 's' : ''} ago`;
}

/**
 * Format token amount with decimals
 */
export function formatTokenAmount(amount: number | string, decimals: number = 9): string {
  const numAmount = typeof amount === 'string' ? parseFloat(amount) : amount;
  const adjusted = numAmount / Math.pow(10, decimals);
  return formatNumber(adjusted);
}

