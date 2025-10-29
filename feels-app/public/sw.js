// Service Worker for aggressive WASM caching and optimization
const CACHE_NAME = 'feels-wasm-cache-v1';
const WASM_CACHE_NAME = 'wasm-modules-v1';

// Files to cache aggressively for vanity mining
const WASM_FILES = [
  '/wasm/vanity_miner_wasm_bg.wasm',
  '/wasm/vanity_miner_wasm.js',
  '/wasm/vanity-worker.js',
  '/wasm/vanity-coordinator.js'
];

// Install event - cache WASM files immediately
self.addEventListener('install', (event) => {
  console.log('[SW] Installing service worker for WASM optimization');
  
  event.waitUntil(
    Promise.all([
      // Cache WASM files with high priority
      caches.open(WASM_CACHE_NAME).then((cache) => {
        console.log('[SW] Caching WASM files for instant access');
        return cache.addAll(WASM_FILES);
      }),
      
      // Precompile WASM for instant initialization
      precompileWasm()
    ])
  );
  
  // Skip waiting to activate immediately
  self.skipWaiting();
});

// Activate event - clean up old caches
self.addEventListener('activate', (event) => {
  console.log('[SW] Activating service worker');
  
  event.waitUntil(
    Promise.all([
      // Clean up old caches
      caches.keys().then((cacheNames) => {
        return Promise.all(
          cacheNames.map((cacheName) => {
            if (cacheName !== CACHE_NAME && cacheName !== WASM_CACHE_NAME) {
              console.log('[SW] Deleting old cache:', cacheName);
              return caches.delete(cacheName);
            }
          })
        );
      }),
      
      // Take control of all clients immediately
      self.clients.claim()
    ])
  );
});

// Fetch event - serve WASM files from cache with optimization
self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url);
  
  // Handle WASM files with special optimization
  if (WASM_FILES.some(file => url.pathname === file)) {
    event.respondWith(handleWasmRequest(event.request));
    return;
  }
  
  // For other requests, use network-first strategy
  event.respondWith(
    fetch(event.request)
      .catch(() => caches.match(event.request))
  );
});

// Optimized WASM file serving
async function handleWasmRequest(request) {
  const url = new URL(request.url);
  
  try {
    // Try cache first for instant serving
    const cache = await caches.open(WASM_CACHE_NAME);
    const cachedResponse = await cache.match(request);
    
    if (cachedResponse) {
      console.log('[SW] Serving WASM from cache:', url.pathname);
      
      // Clone response and add optimization headers
      const response = cachedResponse.clone();
      const headers = new Headers(response.headers);
      headers.set('Cache-Control', 'public, max-age=31536000, immutable');
      headers.set('Cross-Origin-Embedder-Policy', 'credentialless');
      headers.set('Cross-Origin-Opener-Policy', 'same-origin');
      
      return new Response(response.body, {
        status: response.status,
        statusText: response.statusText,
        headers: headers
      });
    }
    
    // Fallback to network if not in cache
    console.log('[SW] WASM not in cache, fetching from network:', url.pathname);
    const networkResponse = await fetch(request);
    
    // Cache the response for next time
    cache.put(request, networkResponse.clone());
    
    return networkResponse;
    
  } catch (error) {
    console.error('[SW] Error serving WASM file:', error);
    // Return network response as fallback
    return fetch(request);
  }
}

// Precompile WASM modules for instant initialization
async function precompileWasm() {
  try {
    console.log('[SW] Pre-compiling WASM modules...');
    
    // Fetch and compile the main WASM module
    const wasmUrl = '/wasm/vanity_miner_wasm_bg.wasm';
    const wasmResponse = await fetch(wasmUrl);
    
    if (!wasmResponse.ok) {
      throw new Error(`Failed to fetch WASM: ${wasmResponse.status}`);
    }
    
    const wasmModule = await WebAssembly.compileStreaming(wasmResponse);
    
    // Store compiled module in a specialized cache
    const compiledCache = await caches.open('compiled-wasm-v1');
    const compiledResponse = new Response(wasmModule, {
      headers: {
        'Content-Type': 'application/wasm',
        'Cache-Control': 'public, max-age=31536000, immutable'
      }
    });
    
    await compiledCache.put(wasmUrl + '?compiled=true', compiledResponse);
    
    console.log('[SW] WASM module pre-compiled and cached');
    
    // Broadcast to all clients that compiled WASM is ready
    const clients = await self.clients.matchAll();
    clients.forEach(client => {
      client.postMessage({
        type: 'wasm-precompiled',
        module: wasmModule
      });
    });
    
  } catch (error) {
    console.warn('[SW] WASM pre-compilation failed:', error);
  }
}

// Handle messages from main thread
self.addEventListener('message', async (event) => {
  const { action, url } = event.data;
  
  switch (action) {
    case 'precompile-wasm':
      if (url) {
        try {
          const response = await fetch(url);
          const module = await WebAssembly.compileStreaming(response);
          
          // Send compiled module back to client
          event.ports[0].postMessage({
            type: 'compiled-module',
            module: module
          });
        } catch (error) {
          event.ports[0].postMessage({
            type: 'compilation-error',
            error: error.message
          });
        }
      }
      break;
      
    case 'cache-wasm':
      // Force cache WASM files
      const cache = await caches.open(WASM_CACHE_NAME);
      await cache.addAll(WASM_FILES);
      
      event.ports[0].postMessage({
        type: 'cache-complete'
      });
      break;
  }
});

// Background sync for WASM updates
self.addEventListener('sync', (event) => {
  if (event.tag === 'wasm-update') {
    event.waitUntil(updateWasmCache());
  }
});

// Update WASM cache in background
async function updateWasmCache() {
  try {
    const cache = await caches.open(WASM_CACHE_NAME);
    
    // Update all WASM files
    await Promise.all(
      WASM_FILES.map(async (file) => {
        const response = await fetch(file);
        if (response.ok) {
          await cache.put(file, response);
        }
      })
    );
    
    console.log('[SW] WASM cache updated in background');
  } catch (error) {
    console.error('[SW] Background WASM update failed:', error);
  }
}

console.log('[SW] Feels Protocol WASM Service Worker loaded');