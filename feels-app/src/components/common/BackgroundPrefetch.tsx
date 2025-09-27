'use client';

import { useEffect, useState } from 'react';
import { usePathname } from 'next/navigation';
import { useRouter } from 'next/navigation';
import { useTokenSearch } from '@/hooks/useTokenSearch';

// List of routes to prefetch
const PREFETCH_ROUTES = [
  '/',
  '/search',
  '/launch',
  '/info',
  '/control'
];

export function BackgroundPrefetch() {
  const pathname = usePathname();
  const router = useRouter();
  const [hasPrefetched, setHasPrefetched] = useState(false);
  
  // Pre-load token search data with empty query to get popular tokens
  useTokenSearch('');

  useEffect(() => {
    // Only prefetch once after initial page load
    if (hasPrefetched) return;

    // Wait a bit for the initial page to fully load
    const prefetchTimer = setTimeout(() => {
      // Prefetch all routes except the current one
      PREFETCH_ROUTES.forEach(route => {
        if (route !== pathname) {
          router.prefetch(route);
        }
      });

      // Also prefetch some common token pages if we have them
      // You can expand this based on your needs
      const commonTokens = [
        '/token/feelsWojakMvNsD5n2R8rUPzFiHkq9JbgSstPVNkDPGb',
        '/token/feelsPepewJ9nJKy3sLKCqczaTrd2TRnhjxNLPqZB8nu'
      ];
      
      commonTokens.forEach(tokenRoute => {
        if (tokenRoute !== pathname) {
          router.prefetch(tokenRoute);
        }
      });

      setHasPrefetched(true);
    }, 1000); // Wait 1 second after mount

    return () => clearTimeout(prefetchTimer);
  }, [pathname, router, hasPrefetched]);

  // This component doesn't render anything
  return null;
}