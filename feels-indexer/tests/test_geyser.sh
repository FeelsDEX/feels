#!/bin/bash
# Test script for Geyser-enabled devnet

set -euo pipefail

echo "Testing Geyser-enabled devnet..."

# Function to check if a port is open
check_port() {
    local port=$1
    local name=$2
    if nc -z localhost $port 2>/dev/null; then
        echo "✓ $name is running on port $port"
        return 0
    else
        echo "✗ $name is not available on port $port"
        return 1
    fi
}

# Check services
echo ""
echo "Checking services..."
check_port 8899 "Solana RPC"
check_port 10000 "Geyser gRPC"
check_port 8999 "Geyser Prometheus metrics"

# Test Geyser gRPC connection
echo ""
echo "Testing Geyser gRPC..."
if command -v grpcurl >/dev/null 2>&1; then
    echo "Listing available gRPC services..."
    grpcurl -plaintext localhost:10000 list || echo "Failed to list services"
    
    echo ""
    echo "Getting server info..."
    grpcurl -plaintext localhost:10000 geyser.Geyser/GetVersion || echo "Failed to get version"
else
    echo "grpcurl not found, skipping gRPC tests"
fi

# Test Solana RPC
echo ""
echo "Testing Solana RPC..."
curl -s -X POST -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"getVersion"}' \
    http://localhost:8899 | jq . || echo "Failed to get Solana version"

# Check Prometheus metrics
echo ""
echo "Checking Prometheus metrics..."
curl -s http://localhost:8999/metrics | head -10 || echo "Failed to get metrics"

echo ""
echo "Geyser devnet test complete!"