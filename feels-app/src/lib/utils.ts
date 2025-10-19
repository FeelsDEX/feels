import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

/**
 * Combine Tailwind CSS classes with proper precedence
 * Part of shadcn/ui convention - keep in lib/utils.ts
 */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}