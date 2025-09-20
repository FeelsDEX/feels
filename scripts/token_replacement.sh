#!/bin/bash
# Token naming replacement script for macOS

echo "Starting token naming replacement..."

# Function to replace tokens in a file
replace_in_file() {
    local file=$1
    echo "Processing: $file"
    
    # Create backup
    cp "$file" "$file.bak"
    
    # Perform replacements using perl (more reliable on macOS)
    perl -i -pe '
        s/\btoken_a\b/token_0/g;
        s/\btoken_b\b/token_1/g;
        s/\bToken A\b/Token 0/g;
        s/\bToken B\b/Token 1/g;
        s/\bTOKEN_A\b/TOKEN_0/g;
        s/\bTOKEN_B\b/TOKEN_1/g;
        s/\bomega_a\b/omega_0/g;
        s/\bomega_b\b/omega_1/g;
        s/\btwap_a\b/twap_0/g;
        s/\btwap_b\b/twap_1/g;
        s/\btwap_b_per_a\b/twap_1_per_0/g;
        s/\bamount_a\b/amount_0/g;
        s/\bamount_b\b/amount_1/g;
        s/\bvault_a\b/vault_0/g;
        s/\bvault_b\b/vault_1/g;
        s/\btoken_a_vault\b/token_0_vault/g;
        s/\btoken_b_vault\b/token_1_vault/g;
        s/\btoken_a_mint\b/token_0_mint/g;
        s/\btoken_b_mint\b/token_1_mint/g;
        s/\btoken_a_decimals\b/token_0_decimals/g;
        s/\btoken_b_decimals\b/token_1_decimals/g;
        s/\brate_a\b/rate_0/g;
        s/\brate_b\b/rate_1/g;
        s/\bindex_a\b/index_0/g;
        s/\bindex_b\b/index_1/g;
        s/\bfee_growth_global_a\b/fee_growth_global_0/g;
        s/\bfee_growth_global_b\b/fee_growth_global_1/g;
        s/\bfee_growth_inside_last_a\b/fee_growth_inside_last_0/g;
        s/\bfee_growth_inside_last_b\b/fee_growth_inside_last_1/g;
        s/\btokens_owed_a\b/tokens_owed_0/g;
        s/\btokens_owed_b\b/tokens_owed_1/g;
        s/\bbase_value_a\b/base_value_0/g;
        s/\bbase_value_b\b/base_value_1/g;
        s/\bprice_cumulative_a\b/price_cumulative_0/g;
        s/\bprice_cumulative_b\b/price_cumulative_1/g;
        s/\bgrowth_factor_a\b/growth_factor_0/g;
        s/\bgrowth_factor_b\b/growth_factor_1/g;
        s/\bprotocol_fees_a\b/protocol_fees_0/g;
        s/\bprotocol_fees_b\b/protocol_fees_1/g;
        s/\bfee_share_a\b/fee_share_0/g;
        s/\bfee_share_b\b/fee_share_1/g;
        s/\ba_to_b\b/zero_to_one/g;
        s/\bb_to_a\b/one_to_zero/g;
        s/\buser_token_a\b/user_token_0/g;
        s/\buser_token_b\b/user_token_1/g;
        s/\bpool_token_a\b/pool_token_0/g;
        s/\bpool_token_b\b/pool_token_1/g;
        s/\bamount_a_max\b/amount_0_max/g;
        s/\bamount_a_min\b/amount_0_min/g;
        s/\bamount_b_max\b/amount_1_max/g;
        s/\bamount_b_min\b/amount_1_min/g;
        s/\bvalue_a\b/value_0/g;
        s/\bvalue_b\b/value_1/g;
    ' "$file"
}

# Find and process all Rust files
find programs/feels/src -name "*.rs" | while read -r file; do
    replace_in_file "$file"
done

find crates -name "*.rs" 2>/dev/null | while read -r file; do
    replace_in_file "$file"
done

find programs/feels/tests -name "*.rs" | while read -r file; do
    replace_in_file "$file"
done

echo "Token replacement complete!"
echo "Run 'git diff' to review changes"