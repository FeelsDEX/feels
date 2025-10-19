# Vanity Address Miner WASM

High-performance vanity address miner for Solana, compiled to WebAssembly for use in web browsers. Generates cryptographically secure keypairs where the public address ends with a specific suffix (e.g., "FEEL").

## Overview

This crate generates Solana Ed25519 keypairs where the public address (base58 encoded) ends with a user-specified suffix. It uses cryptographically secure random number generation and is optimized for mining in web browsers.

## Features

- **Cryptographically Secure**: Uses ChaCha20 RNG seeded from browser's crypto.getRandomValues()
- **Web Worker Support**: Designed to work with Web Workers for parallel mining
- **Progress Tracking**: Reports attempts and elapsed time for UI feedback
- **Batch Processing**: Mines in configurable batches for efficient browser execution
- **Suffix Match**: Canonicalizes to uppercase `FEEL`; only exact uppercase matches are accepted
- **Suffix Fast Path**: Uses modular arithmetic to filter candidates before base58 encoding (~98% rejection)
- **RNG Batching**: Reuses an 8 KB ChaCha20 entropy buffer to reduce RNG overhead
- **Multi-Batch API**: `mine_multi_batch32` processes multiple batches per WASM call
- **Rayon Parallelism**: Optional `parallel` feature (enabled by default in builds) fans out the hot loop across WebAssembly threads

## Architecture

### Components

1. **Rust Core (`src/lib.rs`)**
   - `VanityMiner` struct - manages mining state and RNG
   - `mine_sync()` - mines with max attempts limit (delegates to `mine_with_limit`)
   - `mine_until_found()` - mines until match found (delegates to `mine_with_limit`)
   - `mine_batch32()` - mines a single batch, returns match or null
   - `mine_multi_batch32()` - mines multiple batches, returns stats with match
   - `benchmark_single_thread()` - measures single-thread mining rate
   - Uses ChaCha20Rng for cryptographically secure random generation
   - Uses ed25519-dalek for Ed25519 key derivation
   - Uses bs58 for base58 encoding

2. **Optimizations**
   - Entropy buffer (8 KB) amortizes ChaCha20 RNG overhead across 256 keypairs
   - Modular arithmetic prefilter rejects ~98% of candidates before base58 encoding
   - Case-insensitive comparison widens matches while still targeting canonical `FEEL`
   - Rayon-enabled batches split verification work across WebAssembly threads when available
   - Multi-batch execution reduces WASM/JS boundary crossings
   - Reusable buffers for secret keys, public keys, and base58 encoding
   - Aggressive compiler optimizations (LTO, codegen-units=1, opt-level=3)

3. **Web Worker Architecture**
   - Single-threaded workers coordinated by vanity-coordinator.js
   - Each worker runs independently for true parallelism
   - Coordinator manages worker pool and aggregates results
   - Automatic worker count based on CPU cores

## Building

### Prerequisites

- Rust toolchain (stable)
- wasm-pack (installed automatically by build commands if needed)
- Node.js and npm (for frontend integration)

### Build Commands

```bash
# Build for production (optimized)
just build

# Build with development features
just build-dev

# Clean build artifacts
just clean

# Run tests
just test

# Check integration
just test-integration
```

### Manual Build Commands

```bash
# Build for production (optimized, with rayon threads)
RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals -C link-arg=--shared-memory -C link-arg=--import-memory -C link-arg=--export-table" \
  rustup run nightly \
  wasm-pack build --target web --out-dir ../feels-app/public/wasm \
    --no-default-features --features parallel -- -Z build-std=panic_abort,std

# Build with development features
RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals -C link-arg=--shared-memory -C link-arg=--import-memory -C link-arg=--export-table" \
  rustup run nightly \
  wasm-pack build --dev --target web --out-dir ../feels-app/public/wasm \
    --no-default-features --features "console_error_panic_hook parallel" \
    -- -Z build-std=panic_abort,std

# Run tests
wasm-pack test --headless --chrome
```

## Frontend Integration

### 1. Required Headers for Multi-threading

To enable multi-threaded mining, your web server must send these headers:
```
Cross-Origin-Embedder-Policy: credentialless
Cross-Origin-Opener-Policy: same-origin
```

For Next.js, add to `next.config.js`:
```javascript
headers: async () => [{
  source: '/:path*',
  headers: [
    { key: 'Cross-Origin-Embedder-Policy', value: 'credentialless' },
    { key: 'Cross-Origin-Opener-Policy', value: 'same-origin' },
  ],
}]
```

### 2. Worker Architecture

The system uses two worker types:

**vanity-worker.js** - Single-threaded worker for mining
- Handles mining in a single thread
- Sends progress updates periodically
- Uses large batch sizes (100,000) for efficiency

**vanity-coordinator.js** - Multi-threaded coordinator
- Manages multiple vanity-worker.js instances
- Aggregates results and calculates combined mining rate
- Automatically uses (CPU cores - 2) workers
- Tracks individual worker performance

### 3. React Hook Usage

```typescript
import { useVanityAddressMiner } from '@/hooks/useVanityAddressMiner';

function MyComponent() {
  const vanityMiner = useVanityAddressMiner();

  // Start mining
  vanityMiner.startMining();

  // Access status
  console.log(vanityMiner.status.attempts);
  console.log(vanityMiner.status.elapsedMs);
  
  // Check if found
  if (vanityMiner.status.keypair) {
    console.log('Found:', vanityMiner.status.keypair.publicKey);
  }
}
```

### 4. Initializing the Rayon Thread Pool

When the `parallel` feature is enabled (default via the Just recipes), initialize the WebAssembly thread pool before mining:

```typescript
import init, { init_threads } from '@/public/wasm/vanity_miner_wasm.js';

await init();
if (typeof init_threads === 'function' && self.crossOriginIsolated) {
  const threadTarget = Math.max(1, (navigator.hardwareConcurrency ?? 4) - 1);
  await init_threads(threadTarget);
}
```

The worker scripts in `feels-app/public/wasm/` perform this initialization automatically when the environment supports `SharedArrayBuffer`.

## Performance

### Current Performance
- ~35,000 - 40,000 attempts/second per thread
- Modular arithmetic filter rejects most candidates before base58 encoding
- Multi-batch API reduces WASM/JS boundary crossing overhead
- Performance bottleneck is Ed25519 key derivation (ed25519-dalek)

### Multi-threaded Performance
- Scales linearly with CPU cores
- 4-core machine: ~120,000 - 160,000 attempts/second
- 8-core machine: ~240,000 - 320,000 attempts/second
- 16-core machine: ~480,000 - 640,000 attempts/second

### Expected Mining Times

For suffix "FEEL" (4 characters, 1 in 11,316,496 probability):

| Mining Rate | Average Time | 90% Probability |
|-------------|---------------|------------------|
| 40K/sec     | 4.7 minutes   | 10.9 minutes    |
| 160K/sec    | 1.2 minutes   | 2.7 minutes     |
| 320K/sec    | 35 seconds    | 82 seconds      |

## API Reference

### VanityMiner Class

```typescript
class VanityMiner {
  constructor(suffix: string);
  mine_sync(max_attempts: number): FoundKeypair | null;
  mine_until_found(max_attempts: number): FoundKeypair | null;
  mine_batch32(batch_size: number): FoundKeypair | null;
  mine_multi_batch32(batch_size: number, batch_count: number): MiningStats;
  stop(): void;
  is_running(): boolean;
  get_suffix(): string;
}
```

### Types

```typescript
interface FoundKeypair {
  public_key: string;      // Base58 encoded public key
  secret_key: Uint8Array;  // 32-byte secret key
  attempts: number;        // Number of attempts to find
  elapsed_ms: number;      // Time taken in milliseconds
}

interface MiningStats {
  attempts: number;              // Total attempts in this run
  elapsed_ms: number;            // Total time elapsed
  found: FoundKeypair | null;    // Match if found
}
```

### Utility Functions

```typescript
// Benchmark single-thread performance (returns keys/second)
function benchmark_single_thread(duration_ms: number): number;

// Generate a single random keypair
function generate_random_keypair(): FoundKeypair;
```

## Security Considerations

- Each user generates their own cryptographically secure seed using browser's crypto.getRandomValues()
- Seeds cannot be predicted or guessed by other users
- ChaCha20 RNG provides cryptographic security with high performance
- Secret keys are never logged or exposed unnecessarily
- Results are stored in browser localStorage for persistence

## Troubleshooting

### Mining seems slow
1. Current performance is ~35-40K attempts/second per thread (bottleneck is Ed25519)
2. Check if multi-threading is enabled (requires CORS headers)
3. Verify SharedArrayBuffer is available in browser console
4. Check CPU usage - should be high when mining

### No matches found after long time
1. Verify the suffix is correct (case-sensitive)
2. Check browser console for errors
3. Expected time for 4-character suffix at 40K/sec is ~5 minutes average
4. Longer suffixes take exponentially longer (each character multiplies time by ~58)

### Worker errors
1. Ensure WASM files are served with correct MIME type
2. Check browser compatibility (requires WebAssembly support)
3. Verify worker files are accessible at correct paths

## Testing

Test pages are available at:
- `/test-coordinator.html` - Multi-threaded mining test with statistics
- `/test-benchmark.html` - Performance benchmark
- `/test-match-logic.html` - Match logic verification
