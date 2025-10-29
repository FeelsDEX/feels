// Hook for managing vanity address mining in the background
import { useEffect, useRef, useState, useCallback } from 'react';
import { Keypair } from '@solana/web3.js';
import { getDevKeypair } from '@/constants/devKeypairs';

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

export function useVanityAddressMiner(isTestDataMode: boolean = false) {
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
    
    // In test data mode, use predefined development keypair instead of mining
    if (isTestDataMode) {
      console.log('Test data mode detected - using predefined development keypair');
      const devKeypair = getDevKeypair();
      
      setStatus(prev => ({
        ...prev,
        isReady: true,
        keypair: {
          publicKey: devKeypair.publicKey,
          secretKey: new Uint8Array(devKeypair.secretKey),
        },
        attempts: 0, // No mining attempts needed
        elapsedMs: 0, // Instant
      }));
      
      // Store in localStorage for consistency
      localStorage.setItem('vanityKeypair', JSON.stringify({
        publicKey: devKeypair.publicKey,
        secretKey: devKeypair.secretKey,
      }));
      
      console.log(`Using development keypair: ${devKeypair.publicKey}`);
      return;
    }
    
    const canUseMultiThreading = typeof SharedArrayBuffer !== 'undefined' && self.crossOriginIsolated;
    
    console.log(`Initializing vanity miner...`);
    console.log(`Environment: SharedArrayBuffer=${typeof SharedArrayBuffer !== 'undefined'}, crossOriginIsolated=${self.crossOriginIsolated}`);
    
    const initializeWorker = async () => {
      try {
        // Use pre-compiled WASM if available for instant initialization
        let wasmModule = null;
        if (typeof window !== 'undefined' && (window as any).__wasmPromise) {
          try {
            wasmModule = await (window as any).__wasmPromise;
            console.log('Using pre-compiled WASM module for instant initialization');
          } catch (e) {
            console.log('Pre-compiled WASM not available, falling back to normal initialization');
          }
        }
        
        // Use coordinator for multi-threading if available, otherwise single worker
        const workerPath = canUseMultiThreading ? '/wasm/vanity-coordinator.js' : '/wasm/vanity-worker.js';
        const workerType = canUseMultiThreading ? 'multi-threaded coordinator' : 'single worker';
        
        console.log(`Creating ${workerType}...`);
        console.log(`Worker URL: ${workerPath}`);
        const worker = new Worker(workerPath, { type: 'module' });
        
        // Pass pre-compiled WASM module to worker if available
        if (wasmModule) {
          worker.postMessage({ 
            type: 'set-wasm-module', 
            module: wasmModule 
          });
        }
        
        worker.addEventListener('message', (event: MessageEvent<MinerResult>) => {
          if (!mounted) return;
          const result = event.data;

          switch (result.type) {
            case 'found':
              const keypairData = result.result || result.keypair;
              if (keypairData) {
                const publicKey = ('public_key' in keypairData) ? keypairData.public_key : keypairData.publicKey;
                const secretKey = ('secret_key' in keypairData) ? keypairData.secret_key : keypairData.secretKey;
                
                setStatus(prev => ({
                  ...prev,
                  isRunning: false,
                  keypair: {
                    publicKey: publicKey,
                    secretKey: new Uint8Array(secretKey),
                  },
                  attempts: result.attempts || (('attempts' in keypairData) ? keypairData.attempts : 0) || 0,
                  elapsedMs: result.elapsedMs || (('elapsed_ms' in keypairData) ? keypairData.elapsed_ms : 0) || 0,
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
  }, [isTestDataMode]);

  // Start mining
  const startMining = useCallback(() => {
    // In test data mode, keypair is already provided - no need to mine
    if (isTestDataMode) {
      console.log('Test data mode - keypair already available, no mining needed');
      return;
    }
    
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
  }, [isTestDataMode, status.isRunning, status.keypair, status.isReady]);

  // Stop mining
  const stopMining = useCallback(() => {
    // In test data mode, no mining is happening - nothing to stop
    if (isTestDataMode) {
      console.log('Test data mode - no mining to stop');
      return;
    }
    
    if (workerRef.current && status.isRunning) {
      const message: MinerMessage = { type: 'stop' };
      workerRef.current.postMessage(message);
      setStatus(prev => ({
        ...prev,
        isRunning: false,
      }));
    }
  }, [isTestDataMode, status.isRunning]);

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
    
    // In test data mode, immediately restore the dev keypair
    if (isTestDataMode) {
      console.log('Test data mode - restoring development keypair');
      const devKeypair = getDevKeypair();
      setStatus(prev => ({
        ...prev,
        isReady: true,
        keypair: {
          publicKey: devKeypair.publicKey,
          secretKey: new Uint8Array(devKeypair.secretKey),
        },
        attempts: 0,
        elapsedMs: 0,
      }));
      localStorage.setItem('vanityKeypair', JSON.stringify({
        publicKey: devKeypair.publicKey,
        secretKey: devKeypair.secretKey,
      }));
    } else {
      // Start mining after a short delay in normal mode
      setTimeout(startMining, 100);
    }
  }, [isTestDataMode, startMining]);

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