# Patches Directory

This directory contains patched versions of upstream dependencies applied via Cargo's `[patch.crates-io]` mechanism.

## solana-net-utils

**Version**: 2.2.1 (patched)

**Issue**: The original `solana-net-utils` crate contains const functions incompatible with Rust 1.82, causing compilation failures in test dependencies.

**Fix**: Modified const function declarations for compatibility with Rust 1.82 toolchain.

**Applied in**: `Cargo.toml` line 48-50

```toml
[patch.crates-io]
solana-net-utils = { path = "patches/solana-net-utils" }
```

### Dependency Chain

The patch is required by the following transitive dependency chain:

```
programs/feels/Cargo.toml
  └─ solana-program-test (dev-dependency)
      └─ solana-banks-server
          └─ solana-client
              ├─ solana-connection-cache
              │   └─ solana-net-utils ← patched here
              └─ solana-quic-client
                  └─ solana-net-utils ← patched here
```

**Direct dependents of solana-net-utils**:
- `solana-connection-cache` - Connection pooling and caching
- `solana-quic-client` - QUIC protocol client for Solana RPC
- `solana-streamer` - Network streaming utilities
- `solana-udp-client` - UDP client implementation

**Why tests only**: The patch is only needed when running tests that use `solana-program-test`, which brings in the full Solana client stack. The main program build does not depend on these networking components.

### Technical Details

**Original code** (incompatible with Rust 1.82):
```rust
pub const MINIMUM_IP_ECHO_SERVER_THREADS: NonZeroUsize = ...;
```

**Patched code**:
```rust
pub const MINIMUM_IP_ECHO_SERVER_THREADS_VALUE: usize = 2;

pub fn minimum_ip_echo_server_threads() -> NonZeroUsize {
    NonZeroUsize::new(MINIMUM_IP_ECHO_SERVER_THREADS_VALUE).unwrap()
}

// LazyLock static for backward compatibility
pub static MINIMUM_IP_ECHO_SERVER_THREADS: LazyLock<NonZeroUsize> = ...;
```

**Root cause**: Rust 1.82 doesn't support `const NonZeroUsize` initialization. The patch splits the constant into a plain `usize` and provides accessor functions for type safety.

### Files Modified

- `src/ip_echo_server.rs` - Const NonZeroUsize compatibility fix

## Maintenance

These patches should be removed once:
1. Upstream Solana releases a fix, or
2. The project upgrades to a Rust version compatible with the original implementation

Check compatibility when upgrading Solana dependencies or Rust toolchain versions.

