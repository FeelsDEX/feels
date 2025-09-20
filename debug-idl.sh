#!/usr/bin/env bash
set -euo pipefail

echo "=== Debug IDL Comparison ==="

# Test IDL parsing
echo "1. Testing IDL parsing..."
IDL_TEMP=$(mktemp)
jq -r '.instructions[0:2][] | "\(.name):\(.discriminator | map(tostring) | join(","))"' target/idl/feels.json > "$IDL_TEMP"
echo "IDL temp file: $IDL_TEMP"
echo "IDL contents:"
cat "$IDL_TEMP"

# Test SDK parsing
echo -e "\n2. Testing SDK parsing..."
SDK_TEMP=$(mktemp)
echo "SDK temp file: $SDK_TEMP"

for file in sdk/src/instructions/*.rs; do
    if [[ ! -f "$file" ]] || [[ "$(basename "$file")" == "mod.rs" ]]; then
        continue
    fi
    
    echo "  Processing $(basename "$file")..."
    grep "_DISCRIMINATOR: \[u8; 8\] = \[" "$file" 2>/dev/null | head -1 || echo "    No discriminators found"
done

echo -e "\n3. Testing simple comparison..."
echo "cleanup_bonding_curve" > test_input.txt
while IFS= read -r line; do
    echo "  Processing line: '$line'"
    # Test basic grep
    result=$(grep "^${line}:" "$SDK_TEMP" 2>/dev/null || echo "NOT_FOUND")
    echo "  Grep result: '$result'"
done < test_input.txt

# Cleanup
rm -f "$IDL_TEMP" "$SDK_TEMP" test_input.txt
echo "Debug complete!"