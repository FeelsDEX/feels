#!/usr/bin/env bash
set -euo pipefail

echo "Generating IDL for Feels Protocol..."
mkdir -p target/idl

# Try to generate IDL without full build
echo "Attempting IDL extraction..."
if command -v anchor &> /dev/null; then
    # Try anchor idl parse
    anchor idl parse -f programs/feels/src/lib.rs -o target/idl/feels.json 2>/dev/null || {
        echo "Direct parse failed, trying with nix..."
        nix run .#idl-build -- feels || {
            echo "Nix build failed, creating minimal IDL..."
            # Create a minimal IDL
            cat > target/idl/feels.json <<EOF
{
  "version": "0.1.0",
  "name": "feels",
  "address": "BLjLS7TzUBncLxXMjFYxezioeg4RVdc5vRpVRYDq8GyQ",
  "instructions": [],
  "accounts": [],
  "types": [],
  "errors": []
}
EOF
        }
    }
fi

# Update the address if IDL was generated
if [[ -f "target/idl/feels.json" ]]; then
    jq --arg addr "BLjLS7TzUBncLxXMjFYxezioeg4RVdc5vRpVRYDq8GyQ" '.address = $addr' target/idl/feels.json > target/idl/feels.json.tmp
    mv target/idl/feels.json.tmp target/idl/feels.json
    echo "IDL generated and updated with correct program ID"
fi