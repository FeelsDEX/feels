// Suppress specific console warnings
// This runs immediately when loaded, before any React code
(function() {
  if (typeof window !== 'undefined' && window.console && window.console.warn) {
    const originalWarn = window.console.warn;
    
    window.console.warn = function() {
      const args = Array.prototype.slice.call(arguments);
      const warningString = args.join(' ');
      
      // Check if this is the Solflare StreamMiddleware warning
      if (
        warningString.includes('StreamMiddleware') &&
        warningString.includes('Unknown response id') &&
        warningString.includes('solflare-detect-metamask')
      ) {
        // Suppress this specific warning
        return;
      }
      
      // Check if this is the Lit dev mode warning
      if (
        warningString.includes('Lit is in dev mode') &&
        warningString.includes('Not recommended for production')
      ) {
        // Suppress Lit dev mode warning
        return;
      }

      // Pass through all other warnings
      return originalWarn.apply(console, args);
    };
  }
})();