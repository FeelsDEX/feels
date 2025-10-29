// Optimized Worker Pool for Vanity Mining with Pre-warming
export class OptimizedWorkerPool {
  private workers: Worker[] = [];
  private initialized = false;
  private preCompiledModule: WebAssembly.Module | null = null;
  private initPromise: Promise<void> | null = null;

  constructor(
    private workerScript: string = '/wasm/vanity-coordinator.js',
    private maxWorkers: number = Math.max(1, (navigator.hardwareConcurrency || 4) - 4)
  ) {}

  // Pre-warm workers immediately on app start
  async preWarm(): Promise<void> {
    if (this.initPromise) {
      return this.initPromise;
    }

    this.initPromise = this._initialize();
    return this.initPromise;
  }

  private async _initialize(): Promise<void> {
    console.log('[WorkerPool] Pre-warming worker pool...');
    
    try {
      // Try to get pre-compiled WASM module from window
      if (typeof window !== 'undefined' && (window as any).__wasmPromise) {
        try {
          this.preCompiledModule = await (window as any).__wasmPromise;
          console.log('[WorkerPool] Using pre-compiled WASM module');
        } catch (e) {
          console.log('[WorkerPool] Pre-compiled WASM not available');
        }
      }

      // Create workers with optimized batch initialization
      const batchSize = 2;
      const workerBatches: Promise<Worker>[][] = [];

      for (let i = 0; i < this.maxWorkers; i += batchSize) {
        const batch: Promise<Worker>[] = [];
        
        for (let j = i; j < Math.min(i + batchSize, this.maxWorkers); j++) {
          batch.push(this.createOptimizedWorker(j));
        }
        
        workerBatches.push(batch);
      }

      // Initialize workers in batches for better performance
      for (const batch of workerBatches) {
        const batchWorkers = await Promise.all(batch);
        this.workers.push(...batchWorkers);
      }

      this.initialized = true;
      console.log(`[WorkerPool] Pre-warmed ${this.workers.length} workers successfully`);
      
    } catch (error) {
      console.error('[WorkerPool] Pre-warming failed:', error);
      throw error;
    }
  }

  private async createOptimizedWorker(index: number): Promise<Worker> {
    console.log(`[WorkerPool] Creating optimized worker ${index}...`);
    
    return new Promise((resolve, reject) => {
      const worker = new Worker(this.workerScript, { type: 'module' });
      
      // Set timeout for worker initialization
      const timeout = setTimeout(() => {
        reject(new Error(`Worker ${index} initialization timeout`));
      }, 8000); // Reduced timeout for faster failure detection

      const handleReady = (event: MessageEvent) => {
        if (event.data.type === 'ready') {
          clearTimeout(timeout);
          worker.removeEventListener('message', handleReady);
          worker.removeEventListener('error', handleError);
          
          console.log(`[WorkerPool] Worker ${index} ready`);
          resolve(worker);
        }
      };

      const handleError = (error: ErrorEvent) => {
        clearTimeout(timeout);
        worker.removeEventListener('message', handleReady);
        worker.removeEventListener('error', handleError);
        
        console.error(`[WorkerPool] Worker ${index} error:`, error);
        reject(new Error(`Worker initialization failed: ${error.message}`));
      };

      worker.addEventListener('message', handleReady);
      worker.addEventListener('error', handleError);

      // Send pre-compiled WASM module if available
      if (this.preCompiledModule) {
        worker.postMessage({
          type: 'set-wasm-module',
          module: this.preCompiledModule
        });
      }

      // Initialize the worker
      worker.postMessage({ type: 'init' });
    });
  }

  // Get a ready worker from the pool
  async getWorker(): Promise<Worker> {
    if (!this.initialized) {
      await this.preWarm();
    }

    if (this.workers.length === 0) {
      throw new Error('No workers available in pool');
    }

    // Return the first available worker (round-robin could be added later)
    const worker = this.workers[0];
    if (!worker) {
      throw new Error('Worker is unexpectedly undefined');
    }
    return worker;
  }

  // Get all workers for coordinator mode
  async getAllWorkers(): Promise<Worker[]> {
    if (!this.initialized) {
      await this.preWarm();
    }

    return this.workers;
  }

  // Terminate all workers
  terminate(): void {
    console.log('[WorkerPool] Terminating worker pool...');
    
    this.workers.forEach((worker, index) => {
      try {
        worker.terminate();
        console.log(`[WorkerPool] Worker ${index} terminated`);
      } catch (error) {
        console.warn(`[WorkerPool] Error terminating worker ${index}:`, error);
      }
    });

    this.workers = [];
    this.initialized = false;
    this.initPromise = null;
  }

  // Get pool status
  getStatus(): { initialized: boolean; workerCount: number; hasPreCompiledWasm: boolean } {
    return {
      initialized: this.initialized,
      workerCount: this.workers.length,
      hasPreCompiledWasm: this.preCompiledModule !== null
    };
  }

  // Add new worker to pool (for dynamic scaling)
  async addWorker(): Promise<void> {
    if (this.workers.length >= this.maxWorkers) {
      console.warn('[WorkerPool] Cannot add worker: pool at maximum capacity');
      return;
    }

    try {
      const worker = await this.createOptimizedWorker(this.workers.length);
      this.workers.push(worker);
      console.log(`[WorkerPool] Added worker, pool size: ${this.workers.length}`);
    } catch (error) {
      console.error('[WorkerPool] Failed to add worker:', error);
      throw error;
    }
  }

  // Remove worker from pool
  removeWorker(): void {
    if (this.workers.length <= 1) {
      console.warn('[WorkerPool] Cannot remove worker: minimum pool size reached');
      return;
    }

    const worker = this.workers.pop();
    if (worker) {
      worker.terminate();
      console.log(`[WorkerPool] Removed worker, pool size: ${this.workers.length}`);
    }
  }
}

// Global worker pool instance for pre-warming
let globalWorkerPool: OptimizedWorkerPool | null = null;

// Initialize global worker pool
export function initializeGlobalWorkerPool(): OptimizedWorkerPool {
  if (!globalWorkerPool) {
    globalWorkerPool = new OptimizedWorkerPool();
    
    // Start pre-warming immediately
    globalWorkerPool.preWarm().catch(error => {
      console.error('[WorkerPool] Global pool pre-warming failed:', error);
    });
  }
  
  return globalWorkerPool;
}

// Get global worker pool
export function getGlobalWorkerPool(): OptimizedWorkerPool | null {
  return globalWorkerPool;
}

// Cleanup global worker pool
export function cleanupGlobalWorkerPool(): void {
  if (globalWorkerPool) {
    globalWorkerPool.terminate();
    globalWorkerPool = null;
  }
}