// Hook for managing vanity address mining in the background
import { useEffect, useRef, useState, useCallback } from 'react';
import { Keypair } from '@solana/web3.js';

// Message types for WASM miner
export interface MinerMessage {
  type: 'init' | 'start' | 'stop' | 'check';
  suffix?: string;
  data?: any;
}

export interface MinerResult {
  type: 'found' | 'progress' | 'error' | 'ready';
  result?: {
    public_key: string;
    secret_key: number[];
    attempts: number;
    elapsed_ms: number;
  };
  keypair?: {
    publicKey: string;
    secretKey: number[];
  };
  attempts?: number;
  elapsedMs?: number;
  error?: string;
  rate?: number;
  isMultiThreaded?: boolean;
  workers?: number;
}

export interface VanityKeypair {
  publicKey: string;
  secretKey: Uint8Array;
}

export interface MinerStatus {
  isReady: boolean;
  isRunning: boolean;
  attempts: number;
  elapsedMs: number;
  keypair: VanityKeypair | null;
  error: string | null;
}

export function useVanityAddressMiner() {
  const workerRef = useRef<Worker | null>(null);
  const performanceRate = useRef<number>(0);
  const [status, setStatus] = useState<MinerStatus>({
    isReady: false,
    isRunning: false,
    attempts: 0,
    elapsedMs: 0,
    keypair: null,
    error: null,
  });

  // Initialize worker
  useEffect(() => {
    let mounted = true;
    const canUseMultiThreading = typeof SharedArrayBuffer !== 'undefined' && self.crossOriginIsolated;
    
    console.log(`Initializing vanity miner...`);
    console.log(`Environment: SharedArrayBuffer=${typeof SharedArrayBuffer !== 'undefined'}, crossOriginIsolated=${self.crossOriginIsolated}`);
    
    const initializeWorker = async () => {
      try {
        // Use coordinator for multi-threading if available, otherwise single worker
        const workerPath = canUseMultiThreading ? '/wasm/vanity-coordinator.js' : '/wasm/vanity-worker.js';
        const workerType = canUseMultiThreading ? 'multi-threaded coordinator' : 'single worker';
        
        // Add cache-busting parameter to force reload
        const cacheBuster = `?v=${Date.now()}`;
        const workerPathWithCache = workerPath + cacheBuster;
        
        console.log(`Creating ${workerType}...`);
        console.log(`Worker URL: ${workerPathWithCache}`);
        const worker = new Worker(workerPathWithCache, { type: 'module' });
        
        worker.addEventListener('message', (event: MessageEvent<MinerResult>) => {
          if (!mounted) return;
          const result = event.data;

          switch (result.type) {
            case 'found':
              const keypairData = result.result || result.keypair;
              if (keypairData) {
                const publicKey = keypairData.public_key || keypairData.publicKey;
                const secretKey = keypairData.secret_key || keypairData.secretKey;
                
                setStatus(prev => ({
                  ...prev,
                  isRunning: false,
                  keypair: {
                    publicKey: publicKey,
                    secretKey: new Uint8Array(secretKey),
                  },
                  attempts: result.attempts || keypairData.attempts || 0,
                  elapsedMs: result.elapsedMs || keypairData.elapsed_ms || 0,
                }));
                
                localStorage.setItem('vanityKeypair', JSON.stringify({
                  publicKey: publicKey,
                  secretKey: Array.from(secretKey),
                }));
              }
              break;

            case 'progress':
              setStatus(prev => ({
                ...prev,
                attempts: result.attempts || 0,
                elapsedMs: result.elapsedMs || 0,
              }));
              break;

            case 'error':
              setStatus(prev => ({
                ...prev,
                error: result.error || 'Unknown error',
                isRunning: false,
              }));
              break;

            case 'ready':
              setStatus(prev => ({ ...prev, isReady: true }));
              if (result.rate) {
                performanceRate.current = result.rate;
                const mode = result.isMultiThreaded ? 
                  `multi-threaded (${result.workers || 'unknown'} workers)` : 
                  'single-threaded';
                console.log(`WASM miner ready (${mode}). Performance: ${result.rate.toLocaleString()} attempts/sec`);
              }
              break;
          }
        });

        worker.addEventListener('error', (event: ErrorEvent) => {
          if (!mounted) return;
          const errorMessage = event.error?.message || event.message || 'Worker error';
          console.error('Vanity address miner worker error:');
          console.error('  Message:', errorMessage);
          console.error('  Filename:', event.filename || 'unknown');
          console.error('  Line:', event.lineno, 'Column:', event.colno);
          if (event.error) {
            console.error('  Error stack:', event.error.stack);
          }
          setStatus(prev => ({
            ...prev,
            error: errorMessage,
            isRunning: false,
          }));
        });

        worker.addEventListener('messageerror', (event) => {
          if (!mounted) return;
          console.error('Worker message deserialization error:', event);
          setStatus(prev => ({
            ...prev,
            error: 'Worker message error',
            isRunning: false,
          }));
        });

        workerRef.current = worker;
        console.log(`Worker created successfully: ${workerType}`);
        
        // Send init message to coordinator
        if (canUseMultiThreading) {
          console.log('Sending init message to coordinator...');
          worker.postMessage({ type: 'init' });
        }
        
      } catch (error) {
        console.error('Failed to create worker:', error);
        if (!mounted) return;
        const errorMsg = error instanceof Error ? error.message : 'Failed to initialize miner';
        const helpText = errorMsg.includes('Failed to fetch') || errorMsg.includes('import') 
          ? 'WASM files missing. Run: cd vanity-miner-wasm && just build'
          : errorMsg;
        setStatus(prev => ({
          ...prev,
          error: helpText,
          isRunning: false,
        }));
      }
    };
    
    // Start initialization
    initializeWorker();
    
    // Expose worker to window for debugging
    if (typeof window !== 'undefined') {
      (window as any).__vanityMinerWorker = workerRef.current;
    }

    // Check if we already have a vanity keypair stored
    const stored = localStorage.getItem('vanityKeypair');
    if (stored) {
      try {
        const parsed = JSON.parse(stored);
        setStatus(prev => ({
          ...prev,
          keypair: {
            publicKey: parsed.publicKey,
            secretKey: new Uint8Array(parsed.secretKey),
          },
        }));
      } catch (e) {
        console.error('Failed to parse stored vanity keypair:', e);
        localStorage.removeItem('vanityKeypair');
      }
    }

    // Cleanup
    return () => {
      mounted = false;
      if (workerRef.current) {
        workerRef.current.terminate();
      }
    };
  }, []);

  // Start mining
  const startMining = useCallback(() => {
    if (workerRef.current && !status.isRunning && !status.keypair) {
      // Check if WASM is ready
      if (!status.isReady) {
        console.log('WASM not ready yet, waiting...');
        return;
      }
      
      setStatus(prev => ({
        ...prev,
        isRunning: true,
        error: null,
      }));
      
      console.log('Starting mining with suffix: FEEL');
      const message: MinerMessage = { type: 'start', suffix: 'FEEL' };
      workerRef.current.postMessage(message);
    }
  }, [status.isRunning, status.keypair, status.isReady]);

  // Stop mining
  const stopMining = useCallback(() => {
    if (workerRef.current && status.isRunning) {
      const message: MinerMessage = { type: 'stop' };
      workerRef.current.postMessage(message);
      setStatus(prev => ({
        ...prev,
        isRunning: false,
      }));
    }
  }, [status.isRunning]);

  // Clear stored keypair and restart mining
  const resetAndMine = useCallback(() => {
    localStorage.removeItem('vanityKeypair');
    setStatus(prev => ({
      ...prev,
      isRunning: false,
      attempts: 0,
      elapsedMs: 0,
      keypair: null,
      error: null,
    }));
    // Start mining after a short delay
    setTimeout(startMining, 100);
  }, [startMining]);

  // Convert keypair to Solana Keypair object
  const getSolanaKeypair = useCallback((): Keypair | null => {
    if (!status.keypair) return null;
    try {
      return Keypair.fromSecretKey(status.keypair.secretKey);
    } catch (e) {
      console.error('Failed to create Solana keypair:', e);
      return null;
    }
  }, [status.keypair]);

  return {
    status,
    startMining,
    stopMining,
    resetAndMine,
    getSolanaKeypair,
  };
}