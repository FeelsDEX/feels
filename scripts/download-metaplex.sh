#!/usr/bin/env bash
set -e

# Script to download Metaplex Token Metadata program binary for tests

# Create directory for external programs
EXTERNAL_PROGRAMS_DIR="./target/external-programs"
mkdir -p "$EXTERNAL_PROGRAMS_DIR"

echo "Downloading Metaplex Token Metadata program..."

# The program ID for Token Metadata
PROGRAM_ID="metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
PROGRAM_NAME="mpl_token_metadata"
OUTPUT_FILE="${EXTERNAL_PROGRAMS_DIR}/${PROGRAM_NAME}.so"

# Try to download from Metaplex's Solana program library
# Note: This is the mainnet deployed program
DOWNLOAD_URL="https://api.mainnet-beta.solana.com"

if [ -f "$OUTPUT_FILE" ] && [ -s "$OUTPUT_FILE" ] && [ $(stat -f%z "$OUTPUT_FILE" 2>/dev/null || stat -c%s "$OUTPUT_FILE" 2>/dev/null) -gt 1000 ]; then
    echo "Metaplex program already downloaded at ${OUTPUT_FILE}"
else
    echo "Downloading Metaplex Token Metadata program..."
    
    # Use solana CLI to dump the program if available
    if command -v solana &> /dev/null; then
        echo "Using solana CLI to dump the program..."
        solana program dump -u mainnet-beta ${PROGRAM_ID} ${OUTPUT_FILE} || {
            echo "Failed to dump program with solana CLI"
            exit 1
        }
    else
        echo "Error: solana CLI not found. Please ensure you're in the nix development shell."
        echo "Run: nix develop"
        exit 1
    fi
fi

# Verify the file exists and is not empty
if [ ! -s "$OUTPUT_FILE" ]; then
    echo "Error: Downloaded file is empty or doesn't exist"
    exit 1
fi

FILE_SIZE=$(stat -f%z "$OUTPUT_FILE" 2>/dev/null || stat -c%s "$OUTPUT_FILE" 2>/dev/null)
echo "Metaplex Token Metadata program ready for tests!"
echo "Location: ${OUTPUT_FILE}"
echo "Size: ${FILE_SIZE} bytes"