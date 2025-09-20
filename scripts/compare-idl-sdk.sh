#!/usr/bin/env bash
set -euo pipefail

# IDL vs SDK Instruction Comparison Tool
# Compares discriminators between IDL and SDK instruction files

IDL_PATH="target/idl/feels.json"
SDK_DIR="sdk/src/instructions"

echo "=== IDL vs SDK Instruction Comparison ==="
echo

# Check if IDL file exists
if [[ ! -f "$IDL_PATH" ]]; then
    echo "Error: IDL file not found at $IDL_PATH"
    echo "Run 'just idl-build' first to generate the IDL"
    exit 1
fi

# Check if SDK directory exists
if [[ ! -d "$SDK_DIR" ]]; then
    echo "Error: SDK instructions directory not found at $SDK_DIR"
    exit 1
fi

# Check for jq
if ! command -v jq >/dev/null 2>&1; then
    echo "Error: jq is required but not installed"
    exit 1
fi

# Parse IDL instructions using jq
echo "Parsing IDL..."
IDL_TEMP=$(mktemp)
jq -r '.instructions[] | "\(.name):\(.discriminator | map(tostring) | join(","))"' "$IDL_PATH" > "$IDL_TEMP"
IDL_COUNT=$(wc -l < "$IDL_TEMP")

echo "Found $IDL_COUNT instructions in IDL"

# Find SDK discriminators
echo "Scanning SDK files..."
SDK_TEMP=$(mktemp)
SDK_COUNT=0

for file in "$SDK_DIR"/*.rs; do
    if [[ ! -f "$file" ]] || [[ "$(basename "$file")" == "mod.rs" ]]; then
        continue
    fi
    
    echo "  Scanning $(basename "$file")..."
    
    # Process discriminators more directly
    while IFS= read -r line; do
        if [[ "$line" =~ _DISCRIMINATOR:.*\[u8 ]]; then
            # Extract const name and discriminator values  
            const_name=$(echo "$line" | sed -n 's/.*const \([A-Z_]*\)_DISCRIMINATOR.*/\1/p')
            disc_values=$(echo "$line" | sed -n 's/.*= \[\([^]]*\)\].*/\1/p')
            
            if [[ -n "$const_name" && -n "$disc_values" ]]; then
                # Convert to lowercase
                instruction_name=$(echo "$const_name" | tr '[:upper:]' '[:lower:]')
                
                # Handle both hex (0x) and decimal formats
                if [[ "$disc_values" =~ 0x ]]; then
                    # Convert hex to decimal using bc
                    clean_disc=""
                    IFS=',' read -ra values <<< "$(echo "$disc_values" | sed 's/[[:space:]]//g')"
                    for val in "${values[@]}"; do
                        hex_val=$(echo "$val" | sed 's/0x//')
                        if [[ -n "$hex_val" ]]; then
                            decimal=$(echo "ibase=16; ${hex_val^^}" | bc)
                            clean_disc="${clean_disc}${decimal},"
                        fi
                    done
                    clean_disc=${clean_disc%,}  # Remove trailing comma
                else
                    # Already decimal, just clean up
                    clean_disc=$(echo "$disc_values" | sed 's/[[:space:]]//g')
                fi
                
                echo "${instruction_name}:${clean_disc}" >> "$SDK_TEMP"
            fi
        fi
    done < "$file"
done

SDK_COUNT=$(wc -l < "$SDK_TEMP")
echo "Found $SDK_COUNT discriminators in SDK"
echo

# Sort files for comparison
sort "$IDL_TEMP" -o "$IDL_TEMP"
sort "$SDK_TEMP" -o "$SDK_TEMP"

# Compare instructions
missing_count=0
mismatch_count=0
correct_count=0

missing_file=$(mktemp)
mismatch_file=$(mktemp)
correct_file=$(mktemp)

echo "Starting comparison..."

# Read the IDL temp file line by line to avoid any potential issues
exec 3< "$IDL_TEMP"
while IFS=: read -r idl_name idl_disc <&3; do
    echo "  Checking instruction: $idl_name"
    
    # Look for SDK line using grep 
    sdk_line=""
    if [[ -f "$SDK_TEMP" ]]; then
        sdk_line=$(grep "^${idl_name}:" "$SDK_TEMP" 2>/dev/null || true)
    fi
    
    if [[ -z "$sdk_line" ]]; then
        echo "$idl_name:$idl_disc" >> "$missing_file"
        ((missing_count++))
        echo "    → Missing in SDK"
    else
        sdk_disc="${sdk_line#*:}"
        
        # Normalize for comparison
        norm_idl=$(echo "$idl_disc" | sed 's/[[:space:]]//g')
        norm_sdk=$(echo "$sdk_disc" | sed 's/[[:space:]]//g')
        
        if [[ "$norm_idl" == "$norm_sdk" ]]; then
            echo "$idl_name" >> "$correct_file"
            ((correct_count++))
            echo "    → Correct"
        else
            echo "$idl_name:$idl_disc:$sdk_disc" >> "$mismatch_file"
            ((mismatch_count++))
            echo "    → Mismatch (IDL: $norm_idl, SDK: $norm_sdk)"
        fi
    fi
done
exec 3<&-

echo "Comparison complete."

# Print summary
echo "## Summary"
echo
echo "- Total IDL instructions: $IDL_COUNT"
echo "- Missing in SDK: $missing_count"
echo "- Discriminator mismatches: $mismatch_count"
echo "- Correctly implemented: $correct_count"
echo

# Print missing instructions
if [[ $missing_count -gt 0 ]]; then
    echo "## Missing Instructions"
    echo
    while IFS=: read -r name disc; do
        echo "- $name (discriminator: [$disc])"
    done < "$missing_file"
    echo
fi

# Print mismatches
if [[ $mismatch_count -gt 0 ]]; then
    echo "## Discriminator Mismatches"
    echo
    while IFS=: read -r name idl_disc sdk_disc; do
        echo "- $name"
        echo "  - IDL: [$idl_disc]"
        echo "  - SDK: [$sdk_disc]"
    done < "$mismatch_file"
    echo
fi

# Print correct implementations
if [[ $correct_count -gt 0 ]]; then
    echo "## Correctly Implemented"
    echo
    while read -r name; do
        echo "- $name ✓"
    done < "$correct_file"
    echo
fi

# Generate fix suggestions
if [[ $missing_count -gt 0 ]] || [[ $mismatch_count -gt 0 ]]; then
    echo "## Fix Suggestions"
    echo
    
    # Process missing instructions
    if [[ $missing_count -gt 0 ]]; then
        while IFS=: read -r name disc; do
            const_name=$(echo "$name" | tr '[:lower:]' '[:upper:]')
            echo "const ${const_name}_DISCRIMINATOR: [u8; 8] = [$disc];"
        done < "$missing_file"
    fi
    
    # Process mismatched instructions
    if [[ $mismatch_count -gt 0 ]]; then
        while IFS=: read -r name idl_disc sdk_disc; do
            const_name=$(echo "$name" | tr '[:lower:]' '[:upper:]')
            echo "const ${const_name}_DISCRIMINATOR: [u8; 8] = [$idl_disc]; // Was: [$sdk_disc]"
        done < "$mismatch_file"
    fi
    echo
fi

# Cleanup
rm -f "$IDL_TEMP" "$SDK_TEMP" "$missing_file" "$mismatch_file" "$correct_file"

echo "Comparison complete!"