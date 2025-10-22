use core::convert::TryInto;
use ed25519_dalek::SigningKey;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "parallel")]
use std::sync::atomic::AtomicUsize;
use wasm_bindgen::prelude::*;

#[cfg(feature = "parallel")]
use rayon::prelude::*;
#[cfg(feature = "parallel")]
use std::sync::{Arc, OnceLock};

// Ed25519 key length in bytes
const SECRET_LEN: usize = 32;
// Number of secrets to generate per RNG fill (reduces overhead)
const ENTROPY_CHUNKS: usize = 256;
// Total entropy buffer size (32 bytes × 256 = 8KB)
const ENTROPY_BUFFER_LEN: usize = SECRET_LEN * ENTROPY_CHUNKS;
// Maximum encoded base58 public key length
const BASE58_BUFFER_LEN: usize = 64;
// Threshold for modulus optimization (prevents overflow in suffix matching)
const BASE58_THRESHOLD_U64: u64 = u64::MAX / 256;
// Base58 alphabet used for Solana public keys
const BASE58_ALPHABET: &[u8; 58] =
    b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
// Default suffix if user provides empty string
const DEFAULT_SUFFIX: &str = "FEEL";

// Global flag indicating if Rayon thread pool is initialized
#[cfg(feature = "parallel")]
static PARALLEL_READY: AtomicBool = AtomicBool::new(false);

// Re-export init_thread_pool for wasm-bindgen-rayon
// This is REQUIRED for wasm-bindgen to generate the threading support correctly
#[cfg(feature = "parallel")]
pub use wasm_bindgen_rayon::init_thread_pool;

// Initialize WebAssembly thread pool for parallel mining (requires SharedArrayBuffer)
#[cfg(feature = "parallel")]
#[wasm_bindgen]
pub fn init_threads(worker_count: usize) -> js_sys::Promise {
    let threads = worker_count.clamp(1, 64);
    let promise = wasm_bindgen_rayon::init_thread_pool(threads);
    
    // Create a new promise that sets the ready flag when initialization completes
    let future = async move {
        wasm_bindgen_futures::JsFuture::from(promise).await?;
        PARALLEL_READY.store(true, Ordering::SeqCst);
        Ok(JsValue::UNDEFINED)
    };
    
    wasm_bindgen_futures::future_to_promise(future)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FoundKeypair {
    pub public_key: String,
    pub secret_key: Vec<u8>,
    pub attempts: u64,
    pub elapsed_ms: f64,
}

// Precomputed parameters for fast suffix matching using modular arithmetic
#[derive(Clone, Copy)]
struct SuffixParams {
    modulus: u64, // Base58 suffix converted to modulus
    value: u64,   // Target remainder for matching
}

// Statistics returned by multi-batch mining operations
#[derive(Serialize)]
struct MiningStats {
    attempts: u64,               // Total attempts in this run
    elapsed_ms: f64,             // Total time elapsed
    found: Option<FoundKeypair>, // Match if found
}

// Result of a mining run (internal use)
struct RunOutcome {
    attempts: u32,               // Number of keys tried
    found: Option<FoundKeypair>, // Match if found
}

// Candidate match from parallel worker (used to pass result back to main thread)
#[cfg(feature = "parallel")]
struct ParallelCandidate {
    index: usize,                   // Index in batch where match was found
    secret: [u8; SECRET_LEN],       // Secret key that produced match
    public_key: [u8; SECRET_LEN],   // Public key that matches suffix
}

#[wasm_bindgen]
pub struct VanityMiner {
    suffix: String,                                      // Original suffix (canonical uppercase)
    suffix_bytes: Vec<u8>,                               // Suffix as bytes (must be uppercase)
    suffix_params: Option<SuffixParams>,                 // Precomputed params for fast filtering
    is_running: AtomicBool,                              // Mining state flag (atomic for thread safety)
    rng: ChaCha20Rng,                                    // Fast CSPRNG
    entropy_buffer: Box<[u8; ENTROPY_BUFFER_LEN]>,       // Bulk entropy buffer (reduces RNG calls)
    entropy_offset: usize,                               // Current position in entropy buffer
    secret_buffer: Box<[u8; SECRET_LEN]>,                // Reusable secret key buffer
    public_key_buffer: Box<[u8; SECRET_LEN]>,            // Reusable public key buffer
    encoding_buffer: Box<[u8; BASE58_BUFFER_LEN]>,       // Reusable base58 encoding buffer
}

#[wasm_bindgen]
impl VanityMiner {
    #[wasm_bindgen(constructor)]
    pub fn new(mut suffix: String) -> Self {
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();

        // Use default suffix if empty
        if suffix.trim().is_empty() {
            suffix = DEFAULT_SUFFIX.to_string();
        }

        // Canonicalize to uppercase so comparisons require exact "FEEL"
        let canonical_suffix = canonicalize_suffix(&suffix);
        let suffix_bytes = canonical_suffix.as_bytes().to_vec();
        // Precompute modular arithmetic parameters for fast filtering
        let suffix_params = compute_suffix_params(&suffix_bytes);

        // Initialize RNG with crypto-secure entropy
        let mut seed = [0u8; SECRET_LEN];
        getrandom::getrandom(&mut seed).unwrap();
        let rng = ChaCha20Rng::from_seed(seed);

        VanityMiner {
            suffix: canonical_suffix,
            suffix_bytes,
            suffix_params,
            is_running: AtomicBool::new(false),
            rng,
            entropy_buffer: Box::new([0u8; ENTROPY_BUFFER_LEN]),
            entropy_offset: ENTROPY_BUFFER_LEN, // Force initial fill
            secret_buffer: Box::new([0u8; SECRET_LEN]),
            public_key_buffer: Box::new([0u8; SECRET_LEN]),
            encoding_buffer: Box::new([0u8; BASE58_BUFFER_LEN]),
        }
    }

    pub fn get_suffix(&self) -> String {
        self.suffix.clone()
    }

    // Mine synchronously up to max_attempts (returns found keypair or NULL)
    pub fn mine_sync(&mut self, max_attempts: u64) -> JsValue {
        self.mine_with_limit(max_attempts)
    }

    // Mine until keypair found or max_attempts reached (alias for mine_sync)
    pub fn mine_until_found(&mut self, max_attempts: u64) -> JsValue {
        self.mine_with_limit(max_attempts)
    }

    // Mine a single batch (64-bit wrapper for mine_batch32)
    pub fn mine_batch(&mut self, batch_size: u64) -> JsValue {
        let size = batch_size.min(u32::MAX as u64) as u32;
        self.mine_batch32(size)
    }

    // Mine a single batch of attempts (returns FoundKeypair or NULL)
    pub fn mine_batch32(&mut self, batch_size: u32) -> JsValue {
        if batch_size == 0 {
            return JsValue::NULL;
        }

        self.is_running.store(true, Ordering::SeqCst);
        let start = js_sys::Date::now();
        let outcome = self.run_attempts(batch_size, start, 0);
        self.is_running.store(false, Ordering::SeqCst);

        if let Some(found) = outcome.found {
            serde_wasm_bindgen::to_value(&found).unwrap()
        } else {
            JsValue::NULL
        }
    }

    // Mine multiple batches in a single WASM call (reduces JS/WASM boundary crossings)
    // Returns MiningStats with total attempts, elapsed time, and found keypair if any
    pub fn mine_multi_batch32(&mut self, batch_size: u32, batch_count: u32) -> JsValue {
        if batch_size == 0 || batch_count == 0 {
            let stats = MiningStats {
                attempts: 0,
                elapsed_ms: 0.0,
                found: None,
            };
            return serde_wasm_bindgen::to_value(&stats).unwrap();
        }

        self.is_running.store(true, Ordering::SeqCst);
        let start = js_sys::Date::now();
        let mut attempts_run = 0u64;
        let mut found: Option<FoundKeypair> = None;

        // Run batches until found, stopped, or batch_count exhausted
        for _ in 0..batch_count {
            if !self.is_running.load(Ordering::Relaxed) {
                break;
            }

            let outcome = self.run_attempts(batch_size, start, attempts_run);
            attempts_run += outcome.attempts as u64;

            if let Some(result) = outcome.found {
                found = Some(result);
                break;
            }

            // Early exit if run was interrupted
            if outcome.attempts < batch_size {
                break;
            }
        }

        self.is_running.store(false, Ordering::SeqCst);
        let elapsed = js_sys::Date::now() - start;
        let stats = MiningStats {
            attempts: attempts_run,
            elapsed_ms: found
                .as_ref()
                .map(|f| f.elapsed_ms)
                .unwrap_or(elapsed),
            found,
        };

        serde_wasm_bindgen::to_value(&stats).unwrap()
    }

    // Stop mining (sets atomic flag checked by worker loops)
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}

impl VanityMiner {
    // Internal: mine with a maximum attempt limit (handles >u32::MAX by chunking)
    fn mine_with_limit(&mut self, max_attempts: u64) -> JsValue {
        if max_attempts == 0 {
            return JsValue::NULL;
        }

        self.is_running.store(true, Ordering::SeqCst);
        let start = js_sys::Date::now();
        let mut attempts_offset = 0u64;
        let mut remaining = max_attempts;

        // Process in u32 chunks (required by run_attempts signature)
        while remaining > 0 && self.is_running.load(Ordering::Relaxed) {
            let chunk = remaining.min(u32::MAX as u64) as u32;
            let outcome = self.run_attempts(chunk, start, attempts_offset);
            attempts_offset += outcome.attempts as u64;

            if let Some(found) = outcome.found {
                self.is_running.store(false, Ordering::SeqCst);
                return serde_wasm_bindgen::to_value(&found).unwrap();
            }

            // Early exit if interrupted
            if outcome.attempts < chunk {
                break;
            }

            remaining -= chunk as u64;
        }

        self.is_running.store(false, Ordering::SeqCst);
        JsValue::NULL
    }

    // Dispatch to parallel or sequential implementation based on feature flag
    fn run_attempts(&mut self, attempts: u32, start: f64, attempt_offset: u64) -> RunOutcome {
        if parallel_enabled() {
            return self.run_attempts_parallel(attempts, start, attempt_offset);
        }
        self.run_attempts_sequential(attempts, start, attempt_offset)
    }

    // Sequential hot loop: generate keypair, check suffix match, repeat
    fn run_attempts_sequential(
        &mut self,
        attempts: u32,
        start: f64,
        attempt_offset: u64,
    ) -> RunOutcome {
        let mut completed = 0u32;

        for attempt_index in 0..attempts {
            // Check if stop() was called
            if !self.is_running.load(Ordering::Relaxed) {
                return RunOutcome {
                    attempts: completed,
                    found: None,
                };
            }

            // Generate next secret key from entropy buffer
            self.next_secret();

            // Derive public key from secret
            let signing_key = SigningKey::from_bytes(self.secret_buffer.as_ref());
            let verifying_key = signing_key.verifying_key();
            let public_key_bytes = verifying_key.to_bytes();
            self.public_key_buffer.copy_from_slice(&public_key_bytes);

            // Check if suffix matches (modular arithmetic + base58 encoding)
            if self.try_match_suffix(&public_key_bytes).is_some() {
                let attempts_total = attempt_offset + attempt_index as u64 + 1;
                let found =
                    self.build_found_keypair_from_public_key(&public_key_bytes, attempts_total, start);
                return RunOutcome {
                    attempts: attempt_index + 1,
                    found: Some(found),
                };
            }

            completed += 1;
        }

        RunOutcome {
            attempts: completed,
            found: None,
        }
    }

    // Parallel hot loop: fan out verification across Rayon thread pool
    #[cfg(feature = "parallel")]
    fn run_attempts_parallel(
        &mut self,
        attempts: u32,
        start: f64,
        attempt_offset: u64,
    ) -> RunOutcome {
        // Fallback if threads not initialized
        if !parallel_enabled() {
            return self.run_attempts_sequential(attempts, start, attempt_offset);
        }

        // Pre-generate all secrets on main thread (RNG is not thread-safe)
        let target = attempts as usize;
        let mut secrets = Vec::with_capacity(target);

        for _ in 0..target {
            if !self.is_running.load(Ordering::Relaxed) {
                break;
            }
            self.next_secret();
            secrets.push(*self.secret_buffer);
        }

        if secrets.is_empty() {
            return RunOutcome {
                attempts: 0,
                found: None,
            };
        }

        // Clone data needed by parallel workers
        let suffix_bytes = self.suffix_bytes.clone();
        let suffix_params = self.suffix_params;
        let found_index = AtomicUsize::new(usize::MAX);
        let candidate_slot: Arc<OnceLock<ParallelCandidate>> = Arc::new(OnceLock::new());
        let running_flag = &self.is_running;

        // Parallel verification: each worker checks its own secret
        secrets
            .par_iter()
            .enumerate()
            .for_each(|(idx, secret)| {
                // Early exit if stopped or already found
                if !running_flag.load(Ordering::Relaxed) {
                    return;
                }
                if found_index.load(Ordering::Relaxed) != usize::MAX {
                    return;
                }

                // Derive public key
                let signing_key = SigningKey::from_bytes(secret);
                let public_key = signing_key.verifying_key().to_bytes();

                // Fast path: modular arithmetic prefilter (~98% rejection)
                if let Some(params) = suffix_params {
                    if !matches_suffix_mod_bytes(&public_key, &params) {
                        return;
                    }
                }

                // Full check: base58 encode and compare suffix
                let mut buffer = [0u8; BASE58_BUFFER_LEN];
                let Ok(encoded_len) = bs58::encode(public_key).onto(&mut buffer[..]) else {
                    return;
                };
                if !suffix_matches_exact(&buffer[..encoded_len], &suffix_bytes) {
                    return;
                }

                // Atomically claim first match (race condition handled by compare_exchange)
                if found_index
                    .compare_exchange(
                        usize::MAX,
                        idx,
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    )
                    .is_ok()
                {
                    let _ = candidate_slot.set(ParallelCandidate {
                        index: idx,
                        secret: *secret,
                        public_key,
                    });
                    running_flag.store(false, Ordering::SeqCst);
                }
            });

        // If match found, build result and update internal state
        if let Some(candidate) = candidate_slot.get() {
            self.secret_buffer.copy_from_slice(&candidate.secret);
            self.public_key_buffer
                .copy_from_slice(&candidate.public_key);

            let attempts_total = attempt_offset + candidate.index as u64 + 1;
            let found = self.build_found_keypair_from_public_key(
                &candidate.public_key,
                attempts_total,
                start,
            );
            return RunOutcome {
                attempts: (candidate.index + 1) as u32,
                found: Some(found),
            };
        }

        RunOutcome {
            attempts: secrets.len() as u32,
            found: None,
        }
    }

    // Stub for parallel when feature is disabled
    #[cfg(not(feature = "parallel"))]
    fn run_attempts_parallel(
        &mut self,
        attempts: u32,
        start: f64,
        attempt_offset: u64,
    ) -> RunOutcome {
        self.run_attempts_sequential(attempts, start, attempt_offset)
    }

    // Fill secret_buffer with next 32 bytes from entropy buffer
    // Refills entropy buffer when exhausted (amortizes RNG cost)
    fn next_secret(&mut self) {
        if self.entropy_offset >= ENTROPY_BUFFER_LEN {
            self.rng.fill_bytes(self.entropy_buffer.as_mut());
            self.entropy_offset = 0;
        }

        let end = self.entropy_offset + SECRET_LEN;
        self.secret_buffer
            .copy_from_slice(&self.entropy_buffer[self.entropy_offset..end]);
        self.entropy_offset = end;
    }

    // Construct FoundKeypair result from matched public key
    fn build_found_keypair_from_public_key(
        &mut self,
        public_key: &[u8; SECRET_LEN],
        attempts: u64,
        start: f64,
    ) -> FoundKeypair {
        let elapsed_ms = js_sys::Date::now() - start;
        let encoded_len = self.encode_into_buffer(public_key);
        let public_key_str =
            String::from_utf8(self.encoding_buffer[..encoded_len].to_vec()).unwrap();
        let secret_key = self.secret_buffer.to_vec();

        FoundKeypair {
            public_key: public_key_str,
            secret_key,
            attempts,
            elapsed_ms,
        }
    }

    // Check if public key matches suffix (fast modular path + full base58 check)
    fn try_match_suffix(&mut self, public_key: &[u8; SECRET_LEN]) -> Option<usize> {
        if self.suffix_bytes.is_empty() {
            return Some(self.encode_into_buffer(public_key));
        }

        // Fast path: modular arithmetic prefilter
        if let Some(params) = &self.suffix_params {
            if !matches_suffix_mod_bytes(public_key, params) {
                return None;
            }

            // Slow path: base58 encode and compare
            let encoded_len = self.encode_into_buffer(public_key);
            if suffix_matches_exact(&self.encoding_buffer[..encoded_len], &self.suffix_bytes) {
                return Some(encoded_len);
            }
            return None;
        }

        // No modular params: just base58 encode and compare
        let encoded_len = self.encode_into_buffer(public_key);
        if suffix_matches_exact(
            &self.encoding_buffer[..encoded_len],
            &self.suffix_bytes,
        ) {
            Some(encoded_len)
        } else {
            None
        }
    }

    // Base58 encode public key into reusable buffer
    fn encode_into_buffer(&mut self, public_key: &[u8; SECRET_LEN]) -> usize {
        bs58::encode(public_key)
            .onto(&mut self.encoding_buffer[..])
            .expect("encoding buffer too small")
    }
}

// Convert suffix to canonical uppercase form
fn canonicalize_suffix(input: &str) -> String {
    let mut canonical = input.to_ascii_uppercase();
    if canonical.is_empty() {
        canonical = DEFAULT_SUFFIX.to_string();
    }
    canonical
}

// Strict suffix comparison (expects uppercase suffix bytes)
fn suffix_matches_exact(haystack: &[u8], suffix: &[u8]) -> bool {
    if suffix.is_empty() {
        return true;
    }
    if haystack.len() < suffix.len() {
        return false;
    }
    let start = haystack.len() - suffix.len();
    &haystack[start..] == suffix
}

// Fast prefilter: check if public key's modulus matches suffix params (~98% rejection)
fn matches_suffix_mod_bytes(public_key: &[u8; SECRET_LEN], params: &SuffixParams) -> bool {
    if params.modulus == 1 {
        return true;
    }

    // Compute remainder of public key when divided by suffix modulus
    let modulus = params.modulus as u128;
    let mut remainder = 0u128;

    for chunk in public_key.chunks_exact(8) {
        let value = u64::from_be_bytes(chunk.try_into().unwrap()) as u128;
        remainder = ((remainder << 64) | value) % modulus;
    }

    remainder == params.value as u128
}

// Precompute modular arithmetic parameters for suffix (returns None if suffix too long)
fn compute_suffix_params(suffix_bytes: &[u8]) -> Option<SuffixParams> {
    if suffix_bytes.is_empty() {
        return Some(SuffixParams {
            modulus: 1,
            value: 0,
        });
    }

    let mut modulus = 1u64;
    let mut value = 0u64;

    // Convert base58 suffix to numeric modulus and value
    for &byte in suffix_bytes {
        let digit = decode_base58_byte(byte)?;
        modulus = modulus.checked_mul(58)?;
        // Abort if modulus would overflow (suffix too long for optimization)
        if modulus > BASE58_THRESHOLD_U64 {
            return None;
        }
        value = value.checked_mul(58)?.checked_add(digit as u64)?;
    }

    Some(SuffixParams { modulus, value })
}

// Decode single base58 character to digit value (0-57)
fn decode_base58_byte(byte: u8) -> Option<u8> {
    BASE58_ALPHABET
        .iter()
        .position(|&ch| ch == byte)
        .map(|idx| idx as u8)
}

// Check if Rayon thread pool is initialized and ready
fn parallel_enabled() -> bool {
    #[cfg(feature = "parallel")]
    {
        PARALLEL_READY.load(Ordering::SeqCst)
    }
    #[cfg(not(feature = "parallel"))]
    {
        false
    }
}

// Utility: generate a random Solana keypair (no vanity matching)
#[wasm_bindgen]
pub fn generate_random_keypair() -> JsValue {
    let mut seed = [0u8; SECRET_LEN];
    getrandom::getrandom(&mut seed).unwrap();
    let mut rng = ChaCha20Rng::from_seed(seed);

    let mut secret = [0u8; SECRET_LEN];
    rng.fill_bytes(&mut secret);

    let signing_key = SigningKey::from_bytes(&secret);
    let public_key = signing_key.verifying_key();

    let public_key_string = bs58::encode(public_key.to_bytes()).into_string();

    let result = FoundKeypair {
        public_key: public_key_string,
        secret_key: secret.to_vec(),
        attempts: 1,
        elapsed_ms: 0.0,
    };

    serde_wasm_bindgen::to_value(&result).unwrap()
}

// Benchmark: measure single-thread keypair generation rate (attempts per second)
#[wasm_bindgen]
pub fn benchmark_single_thread(duration_ms: f64) -> u64 {
    let start = js_sys::Date::now();
    let mut attempts = 0u64;

    // Initialize RNG and buffers
    let mut seed = [0u8; SECRET_LEN];
    getrandom::getrandom(&mut seed).unwrap();
    let mut rng = ChaCha20Rng::from_seed(seed);
    let mut secret_buffer = [0u8; SECRET_LEN];
    let mut public_key_buffer = [0u8; SECRET_LEN];
    let mut encoding_buffer = [0u8; BASE58_BUFFER_LEN];

    // Run full keypair generation + encoding loop for duration
    while js_sys::Date::now() - start < duration_ms {
        rng.fill_bytes(&mut secret_buffer);
        let signing_key = SigningKey::from_bytes(&secret_buffer);
        let public_key = signing_key.verifying_key();
        public_key_buffer.copy_from_slice(&public_key.to_bytes());
        let _ = bs58::encode(public_key_buffer)
            .onto(&mut encoding_buffer[..])
            .expect("encoding buffer too small");
        attempts += 1;
    }

    // Return attempts per second
    let elapsed_secs = duration_ms / 1000.0;
    (attempts as f64 / elapsed_secs) as u64
}

#[cfg(test)]
mod test;
