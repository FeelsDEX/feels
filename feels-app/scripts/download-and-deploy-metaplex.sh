#!/usr/bin/env bash
set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "Checking for Metaplex Token Metadata program..."

METAPLEX_PROGRAM_PATH="../target/external-programs/mpl_token_metadata.so"
METAPLEX_PROGRAM_ID="HsYigmf9uvLS4QQkc7q9BkyvSf6mEVPxYyVf6rwwa8Bw"
MAINNET_METAPLEX_ID="metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
CONFIG_FILE="scripts/metaplex-localnet.json"

# Check if already downloaded
if [[ -f "$METAPLEX_PROGRAM_PATH" ]]; then
    echo "✓ Metaplex program already downloaded"
else
    echo "Downloading Metaplex Token Metadata program..."
    mkdir -p ../target/external-programs
    
    # Try to download from mainnet
    echo "Using solana CLI to dump the program from mainnet..."
    if solana program dump $MAINNET_METAPLEX_ID $METAPLEX_PROGRAM_PATH --url mainnet-beta; then
        echo "✓ Downloaded from mainnet"
    else
        echo -e "${RED}Failed to download Metaplex program${NC}"
        exit 1
    fi
fi

# Check file size
if [[ -f "$METAPLEX_PROGRAM_PATH" ]]; then
    SIZE=$(ls -la "$METAPLEX_PROGRAM_PATH" | awk '{print $5}')
    echo "✓ Metaplex program ready ($SIZE bytes)"
else
    echo -e "${RED}Metaplex program file not found${NC}"
    exit 1
fi

# Deploy to localnet
echo ""
echo "Deploying Metaplex to localnet..."

# Check if validator is running
if ! curl -s http://localhost:8899 >/dev/null 2>&1; then
    echo -e "${RED}Error: Local validator is not running${NC}"
    echo "Please start the validator with: just validator"
    exit 1
fi

# Generate or use existing keypair
METAPLEX_KEYPAIR="../target/deploy/metaplex-keypair.json"
if [[ ! -f "$METAPLEX_KEYPAIR" ]]; then
    echo "Generating Metaplex keypair..."
    solana-keygen new -o "$METAPLEX_KEYPAIR" --no-bip39-passphrase --force
fi

# Get the program ID from keypair
DEPLOYED_PROGRAM_ID=$(solana-keygen pubkey "$METAPLEX_KEYPAIR")
echo "Metaplex will be deployed to: $DEPLOYED_PROGRAM_ID"

# Deploy the program
echo "Deploying Metaplex Token Metadata to localnet..."
if solana program deploy "$METAPLEX_PROGRAM_PATH" --program-id "$METAPLEX_KEYPAIR" --url http://localhost:8899; then
    echo -e "${GREEN}✓ Metaplex deployed successfully!${NC}"
    
    # Save the config
    mkdir -p scripts
    echo "{
  \"programId\": \"$DEPLOYED_PROGRAM_ID\",
  \"deployedAt\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\",
  \"network\": \"localnet\"
}" > "$CONFIG_FILE"
    
    echo "✓ Configuration saved to $CONFIG_FILE"
    echo ""
    echo "Metaplex Token Metadata deployed to: $DEPLOYED_PROGRAM_ID"
else
    echo -e "${RED}Failed to deploy Metaplex Token Metadata${NC}"
    echo "Make sure your local validator is running"
    exit 1
fi