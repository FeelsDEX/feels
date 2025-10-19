// Custom webpack plugin to fix chunk loading issues with Solana dependencies
class ChunkLoadErrorFixPlugin {
  apply(compiler) {
    const webpack = compiler.webpack;
    
    // Ensure stable chunk IDs and names
    compiler.hooks.compilation.tap('ChunkLoadErrorFixPlugin', (compilation) => {
      // Normalize chunk names to prevent filesystem issues
      compilation.hooks.beforeChunkIds.tap('ChunkLoadErrorFixPlugin', () => {
        for (const chunk of compilation.chunks) {
          // Ensure chunk names don't contain problematic characters
          if (chunk.name) {
            // Replace any non-alphanumeric characters (except hyphen and underscore)
            const normalizedName = chunk.name.replace(/[^a-zA-Z0-9-_]/g, '-');
            if (normalizedName !== chunk.name) {
              chunk.name = normalizedName;
            }
            
            // For vendor chunks, use the name as ID for stability
            if (chunk.name.startsWith('vendor-')) {
              chunk.id = chunk.name;
            }
          }
        }
      });

      // Optimize module IDs for better caching using ChunkGraph API
      compilation.hooks.moduleIds.tap('ChunkLoadErrorFixPlugin', (modules) => {
        const chunkGraph = compilation.chunkGraph;
        if (!chunkGraph || !chunkGraph.setModuleId) {
          // ChunkGraph API not available, skip module ID optimization
          return;
        }
        
        const modulesSortedByPath = Array.from(modules)
          .filter(m => m.resource)
          .sort((a, b) => {
            if (a.resource < b.resource) return -1;
            if (a.resource > b.resource) return 1;
            return 0;
          });

        modulesSortedByPath.forEach((module, index) => {
          // Only set IDs for Solana-related modules
          if (module.resource && 
              (module.resource.includes('@solana') || 
               module.resource.includes('@coral-xyz') ||
               module.resource.includes('wallet-adapter'))) {
            // Use ChunkGraph API to set module ID
            try {
              chunkGraph.setModuleId(module, `solana-${index}`);
            } catch (err) {
              // Fall back silently if setModuleId fails
              console.debug('[ChunkLoadErrorFixPlugin] Could not set module ID:', err.message);
            }
          }
        });
      });
    });
  }
}

module.exports = ChunkLoadErrorFixPlugin;