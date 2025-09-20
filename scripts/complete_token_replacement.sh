#!/bin/bash
# Complete token naming replacement script

echo "Running complete token replacement..."

# Function to replace tokens in a file
replace_in_file() {
    local file=$1
    echo "Processing: $file"
    
    # Perform replacements using perl
    perl -i -pe '
        # Token account references
        s/\bauthority_token_account\b/authority_token_account/g;
        s/\bmarket_token_a\b/market_token_0/g;
        s/\bmarket_token_b\b/market_token_1/g;
        
        # Balance function names
        s/\bestimate_token_balance_a\b/estimate_token_balance_0/g;
        s/\bestimate_token_balance_b\b/estimate_token_balance_1/g;
        s/\bbalance_a\b/balance_0/g;
        s/\bbalance_b\b/balance_1/g;
        
        # Zero for one patterns
        s/\ba_to_b\b/zero_to_one/g;
        s/\bb_to_a\b/one_to_zero/g;
        s/\bis_a_to_b\b/is_zero_to_one/g;
        
        # Comments
        s/Token A/Token 0/g;
        s/Token B/Token 1/g;
        s/token A/token 0/g;
        s/token B/token 1/g;
    ' "$file"
}

# Find and process all Rust files
find programs/feels/src -name "*.rs" | while read -r file; do
    replace_in_file "$file"
done

echo "Complete token replacement finished!"