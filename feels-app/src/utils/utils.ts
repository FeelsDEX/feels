import { type ClassValue, clsx } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

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
