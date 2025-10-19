'use client';

import { useEffect, useRef } from 'react';
import { useRouter, usePathname } from 'next/navigation';

export function useGlobalSearchHotkey() {
  const router = useRouter();
  const pathname = usePathname();
  const searchInputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      // Check if the pressed key is "/"
      if (event.key === '/') {
        // Get the currently focused element
        const activeElement = document.activeElement;
        
        // Check if the active element is an input, textarea, or contenteditable
        const isInputFocused = activeElement && (
          activeElement.tagName === 'INPUT' ||
          activeElement.tagName === 'TEXTAREA' ||
          activeElement.getAttribute('contenteditable') === 'true' ||
          // Check for specific input types that should block the hotkey
          (activeElement as HTMLInputElement).type === 'text' ||
          (activeElement as HTMLInputElement).type === 'search' ||
          (activeElement as HTMLInputElement).type === 'number' ||
          (activeElement as HTMLInputElement).type === 'email' ||
          (activeElement as HTMLInputElement).type === 'password' ||
          (activeElement as HTMLInputElement).type === 'url'
        );

        // If no input is focused, prevent default and focus the search
        if (!isInputFocused) {
          event.preventDefault();
          
          // Check if we're on the search page
          if (pathname === '/search') {
            // Focus the page search input
            const pageSearchInput = document.getElementById('page-search-search-input') as HTMLInputElement;
            if (pageSearchInput) {
              pageSearchInput.focus();
              pageSearchInput.select();
            }
          } else {
            // Try to focus the existing search bar in the navigation
            // The search input has id="nav-search-input"
            const navSearchInput = document.getElementById('nav-search-input') as HTMLInputElement;
            if (navSearchInput) {
              navSearchInput.focus();
              // Clear any existing search text when focusing via hotkey
              navSearchInput.select();
            }
          }
        }
      }
    };

    // Add event listener
    window.addEventListener('keydown', handleKeyDown);

    // Cleanup
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [router, pathname]);

  return searchInputRef;
}