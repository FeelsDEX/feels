#!/bin/bash

echo "Initializing Feels Protocol..."

# Get the local keypair
WALLET=$(solana address)
echo "Using wallet: $WALLET"

# Program ID from deployment
PROGRAM_ID="Cbv2aa2zMJdwAwzLnRZuWQ8efpr6Xb9zxpJhEzLe3v6N"

# Derive protocol config PDA
PROTOCOL_CONFIG=$(solana address -k <(echo -n "protocol_config" | xxd -r -p) --program-id $PROGRAM_ID)
echo "Protocol Config PDA: $PROTOCOL_CONFIG"

# Check if already initialized
if solana account $PROTOCOL_CONFIG --url localhost >/dev/null 2>&1; then
    echo "Protocol already initialized!"
    exit 0
fi

# We need to create a transaction that calls initialize_protocol
# Since we don't have a simple CLI for this, we'll use the test suite

echo "Protocol not initialized. Please run the protocol initialization test:"
echo "cargo test test_protocol_init -- --nocapture"