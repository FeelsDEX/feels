// WASM Module Caching with IndexedDB for Instant Initialization
const DB_NAME = 'FeelsWasmCache';
const DB_VERSION = 1;
const STORE_NAME = 'wasmModules';
const CACHE_DURATION = 7 * 24 * 60 * 60 * 1000; // 7 days

interface CachedWasmModule {
  url: string;
  module: WebAssembly.Module;
  timestamp: number;
  version: string;
}

export class WasmModuleCache {
  private db: IDBDatabase | null = null;
  private initPromise: Promise<void> | null = null;

  constructor() {
    this.initPromise = this.initialize();
  }

  private async initialize(): Promise<void> {
    return new Promise((resolve, reject) => {
      const request = indexedDB.open(DB_NAME, DB_VERSION);

      request.onerror = () => {
        console.error('[WasmCache] Failed to open IndexedDB');
        reject(new Error('Failed to open IndexedDB'));
      };

      request.onsuccess = () => {
        this.db = request.result;
        console.log('[WasmCache] IndexedDB initialized');
        resolve();
      };

      request.onupgradeneeded = () => {
        const db = request.result;
        
        if (!db.objectStoreNames.contains(STORE_NAME)) {
          const store = db.createObjectStore(STORE_NAME, { keyPath: 'url' });
          store.createIndex('timestamp', 'timestamp');
          console.log('[WasmCache] Created WASM module store');
        }
      };
    });
  }

  // Get cached WASM module
  async getCachedModule(url: string, currentVersion?: string): Promise<WebAssembly.Module | null> {
    try {
      await this.initPromise;
      
      if (!this.db) {
        console.warn('[WasmCache] Database not available');
        return null;
      }

      return new Promise((resolve, _reject) => {
        const transaction = this.db!.transaction([STORE_NAME], 'readonly');
        const store = transaction.objectStore(STORE_NAME);
        const request = store.get(url);

        request.onsuccess = () => {
          const cached = request.result as CachedWasmModule | undefined;
          
          if (!cached) {
            console.log('[WasmCache] No cached module found for:', url);
            resolve(null);
            return;
          }

          // Check if cache is expired
          const now = Date.now();
          if (now - cached.timestamp > CACHE_DURATION) {
            console.log('[WasmCache] Cached module expired for:', url);
            this.deleteCachedModule(url); // Clean up expired cache
            resolve(null);
            return;
          }

          // Check version if provided
          if (currentVersion && cached.version !== currentVersion) {
            console.log('[WasmCache] Cached module version mismatch for:', url);
            this.deleteCachedModule(url); // Clean up old version
            resolve(null);
            return;
          }

          console.log('[WasmCache] Using cached module for:', url);
          resolve(cached.module);
        };

        request.onerror = () => {
          console.error('[WasmCache] Error retrieving cached module:', request.error);
          resolve(null); // Don't fail, just return null
        };
      });

    } catch (error) {
      console.error('[WasmCache] Error in getCachedModule:', error);
      return null;
    }
  }

  // Cache WASM module
  async cacheModule(url: string, module: WebAssembly.Module, version?: string): Promise<void> {
    try {
      await this.initPromise;
      
      if (!this.db) {
        console.warn('[WasmCache] Database not available for caching');
        return;
      }

      const cachedModule: CachedWasmModule = {
        url,
        module,
        timestamp: Date.now(),
        version: version || '1.0.0'
      };

      return new Promise((resolve, reject) => {
        const transaction = this.db!.transaction([STORE_NAME], 'readwrite');
        const store = transaction.objectStore(STORE_NAME);
        const request = store.put(cachedModule);

        request.onsuccess = () => {
          console.log('[WasmCache] Module cached successfully:', url);
          resolve();
        };

        request.onerror = () => {
          console.error('[WasmCache] Error caching module:', request.error);
          reject(new Error('Failed to cache WASM module'));
        };
      });

    } catch (error) {
      console.error('[WasmCache] Error in cacheModule:', error);
      throw error;
    }
  }

  // Delete cached module
  async deleteCachedModule(url: string): Promise<void> {
    try {
      await this.initPromise;
      
      if (!this.db) {
        return;
      }

      return new Promise((resolve, reject) => {
        const transaction = this.db!.transaction([STORE_NAME], 'readwrite');
        const store = transaction.objectStore(STORE_NAME);
        const request = store.delete(url);

        request.onsuccess = () => {
          console.log('[WasmCache] Cached module deleted:', url);
          resolve();
        };

        request.onerror = () => {
          console.error('[WasmCache] Error deleting cached module:', request.error);
          reject(new Error('Failed to delete cached module'));
        };
      });

    } catch (error) {
      console.error('[WasmCache] Error in deleteCachedModule:', error);
      throw error;
    }
  }

  // Clear all cached modules
  async clearCache(): Promise<void> {
    try {
      await this.initPromise;
      
      if (!this.db) {
        return;
      }

      return new Promise((resolve, reject) => {
        const transaction = this.db!.transaction([STORE_NAME], 'readwrite');
        const store = transaction.objectStore(STORE_NAME);
        const request = store.clear();

        request.onsuccess = () => {
          console.log('[WasmCache] All cached modules cleared');
          resolve();
        };

        request.onerror = () => {
          console.error('[WasmCache] Error clearing cache:', request.error);
          reject(new Error('Failed to clear cache'));
        };
      });

    } catch (error) {
      console.error('[WasmCache] Error in clearCache:', error);
      throw error;
    }
  }

  // Get cache statistics
  async getCacheStats(): Promise<{ count: number; totalSize: number; oldestTimestamp: number }> {
    try {
      await this.initPromise;
      
      if (!this.db) {
        return { count: 0, totalSize: 0, oldestTimestamp: 0 };
      }

      return new Promise((resolve, reject) => {
        const transaction = this.db!.transaction([STORE_NAME], 'readonly');
        const store = transaction.objectStore(STORE_NAME);
        const request = store.getAll();

        request.onsuccess = () => {
          const modules = request.result as CachedWasmModule[];
          
          const stats = {
            count: modules.length,
            totalSize: modules.reduce((total, _module) => {
              // Estimate size (WebAssembly.Module doesn't expose size directly)
              return total + 250000; // Rough estimate: 250KB per module
            }, 0),
            oldestTimestamp: modules.length > 0 
              ? Math.min(...modules.map(m => m.timestamp))
              : 0
          };

          resolve(stats);
        };

        request.onerror = () => {
          console.error('[WasmCache] Error getting cache stats:', request.error);
          reject(new Error('Failed to get cache stats'));
        };
      });

    } catch (error) {
      console.error('[WasmCache] Error in getCacheStats:', error);
      return { count: 0, totalSize: 0, oldestTimestamp: 0 };
    }
  }
}

// Advanced WASM loading with caching
export class OptimizedWasmLoader {
  private cache: WasmModuleCache;
  private loadingPromises = new Map<string, Promise<WebAssembly.Module>>();

  constructor() {
    this.cache = new WasmModuleCache();
  }

  // Load WASM module with intelligent caching
  async loadModule(url: string, version?: string): Promise<WebAssembly.Module> {
    // Check if already loading this URL
    if (this.loadingPromises.has(url)) {
      console.log('[WasmLoader] Reusing existing load promise for:', url);
      return this.loadingPromises.get(url)!;
    }

    const loadPromise = this._loadModuleInternal(url, version);
    this.loadingPromises.set(url, loadPromise);

    try {
      const wasmModule = await loadPromise;
      this.loadingPromises.delete(url);
      return wasmModule;
    } catch (error) {
      this.loadingPromises.delete(url);
      throw error;
    }
  }

  private async _loadModuleInternal(url: string, version?: string): Promise<WebAssembly.Module> {
    console.log('[WasmLoader] Loading WASM module:', url);

    try {
      // Try cache first
      const cachedModule = await this.cache.getCachedModule(url, version);
      if (cachedModule) {
        console.log('[WasmLoader] Using cached WASM module for instant initialization');
        return cachedModule;
      }

      // Load from network with streaming compilation
      console.log('[WasmLoader] Compiling WASM module from network...');
      const startTime = performance.now();
      
      const wasmModule = await WebAssembly.compileStreaming(fetch(url));
      
      const loadTime = performance.now() - startTime;
      console.log(`[WasmLoader] WASM module compiled in ${loadTime.toFixed(2)}ms`);

      // Cache for next time (don't block on this)
      this.cache.cacheModule(url, wasmModule, version).catch(error => {
        console.warn('[WasmLoader] Failed to cache module:', error);
      });

      return wasmModule;

    } catch (error) {
      console.error('[WasmLoader] Failed to load WASM module:', error);
      const message = error instanceof Error ? error.message : String(error);
      throw new Error(`Failed to load WASM module from ${url}: ${message}`);
    }
  }

  // Preload and cache WASM modules
  async preloadModules(urls: string[], version?: string): Promise<void> {
    console.log('[WasmLoader] Preloading WASM modules:', urls);

    const loadPromises = urls.map(url => 
      this.loadModule(url, version).catch(error => {
        console.warn(`[WasmLoader] Failed to preload ${url}:`, error);
        return null;
      })
    );

    await Promise.all(loadPromises);
    console.log('[WasmLoader] WASM modules preloaded');
  }

  // Clear all cached modules
  async clearCache(): Promise<void> {
    await this.cache.clearCache();
  }

  // Get cache statistics
  async getCacheStats() {
    return this.cache.getCacheStats();
  }
}

// Global WASM loader instance
let globalWasmLoader: OptimizedWasmLoader | null = null;

// Initialize global WASM loader
export function initializeGlobalWasmLoader(): OptimizedWasmLoader {
  if (!globalWasmLoader) {
    globalWasmLoader = new OptimizedWasmLoader();
    
    // Preload critical WASM modules
    globalWasmLoader.preloadModules([
      '/wasm/vanity_miner_wasm_bg.wasm'
    ]).catch(error => {
      console.warn('[WasmLoader] Failed to preload modules:', error);
    });
  }
  
  return globalWasmLoader;
}

// Get global WASM loader
export function getGlobalWasmLoader(): OptimizedWasmLoader | null {
  return globalWasmLoader;
}