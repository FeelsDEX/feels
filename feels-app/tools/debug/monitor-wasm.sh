#!/bin/bash
# Monitor DevBridge for WASM multi-threading status

echo "Monitoring DevBridge for WASM multi-threading status..."
echo "Open http://localhost:3000 in your browser to trigger WASM initialization"
echo "----------------------------------------"

npx ts-node --project tsconfig.server.json tools/devbridge/server/cli.ts tail | grep -E "(WASM|thread|SharedArrayBuffer|miner|attempts/sec)" --line-buffered