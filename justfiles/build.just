# Build and compilation recipes

# Import dependencies
import 'common.just'

# ===== Main Build Commands =====

# Build the protocol using Anchor (internal - use 'just build' from main justfile)
_build:
    @just show-building
    @just run-preflight-checks "anchor anchor.toml"
    @if [[ "{{USE_NIX_BUILD}}" == "true" ]] && command -v nix >/dev/null 2>&1; then \
        if anchor build --no-idl --program-name feels; then \
            just show-success "Programs built and available in {{DEPLOY_PATH}}/"; \
        else \
            just exit-with-error {{EXIT_BUILD_FAILURE}} "Anchor build failed"; \
        fi; \
    else \
        just show-warning "Not in Nix environment, using system Anchor"; \
        if anchor build --no-idl --program-name feels; then \
            just show-success "Programs built and available in {{DEPLOY_PATH}}/"; \
        else \
            just exit-with-error {{EXIT_BUILD_FAILURE}} "Anchor build failed"; \
        fi; \
    fi

# Build with Nix BPF builder (hermetic build)
_nix-build:
    @just show-progress "Building with Nix BPF builder"
    @if command -v nix >/dev/null 2>&1; then \
        nix run .#bpf-build; \
    else \
        just show-error "Nix is not installed. Install Nix to use hermetic builds."; \
        exit {{EXIT_MISSING_DEPS}}; \
    fi

# Build IDL files
_idl-build program="":
    #!/usr/bin/env bash
    # Always use Nix for IDL generation due to nightly Rust requirements
    if ! command -v nix >/dev/null 2>&1; then
        just show-error "Nix is required for IDL generation (needs nightly Rust)"
        echo "Please install Nix or run from within 'nix develop'"
        exit {{EXIT_MISSING_DEPS}}
    fi
    
    if [[ -z "{{program}}" ]]; then
        just show-progress "Generating IDL files for all on-chain programs"
        nix run .#idl-build -- feels
        echo "Note: feels-jupiter-adapter is a library, not an on-chain program, so it doesn't have an IDL"
        echo ""
        just show-progress "Generating TypeScript and Rust clients from IDL"
        just _generate-clients
    else
        just show-progress "Generating IDL for {{program}}"
        nix run .#idl-build -- {{program}}
        if [[ "{{program}}" = "feels" ]]; then
            echo ""
            just show-progress "Generating TypeScript and Rust clients from IDL"
            just _generate-clients
        fi
    fi

# Generate TypeScript and Rust clients from IDL
_generate-clients:
    @just show-progress "Generating TypeScript and Rust clients from IDL"
    @just validate-idl-exists
    @just fix-idl-anchor-bug
    @just _generate-typescript-client
    @just generate-rust-client
    @just show-client-locations
    @just show-success "Client generation complete"

# ===== IDL Management =====

# Fix Anchor 0.31.1 IDL generation bug
[private]
fix-idl-anchor-bug:
    #!/usr/bin/env bash
    if [[ ! -f "{{IDL_PATH}}" ]]; then
        just show-error "IDL file not found at {{IDL_PATH}}"
        exit {{EXIT_CONFIG_ERROR}}
    fi
    
    just show-progress "Fixing Anchor IDL generation bug"
    
    # Create backup
    cp "{{IDL_PATH}}" "{{IDL_PATH}}.backup"
    
    # Use the existing fix-idl.js script
    if [[ -f "{{FEELS_APP_PATH}}/src/lib/fix-idl.js" ]]; then
        node "{{FEELS_APP_PATH}}/src/lib/fix-idl.js" "{{IDL_PATH}}" "{{IDL_PATH}}"
        just show-success "IDL fixed successfully"
    else
        just show-warning "fix-idl.js not found, skipping IDL fix"
    fi

# Generate TypeScript client
[private]
_generate-typescript-client:
    #!/usr/bin/env bash
    mkdir -p "{{GENERATED_SDK_PATH}}"
    
    if [[ ! -f "{{IDL_PATH}}" ]]; then
        just show-error "IDL not found at {{IDL_PATH}}"
        exit {{EXIT_CONFIG_ERROR}}
    fi
    
    just show-progress "Converting IDL to TypeScript"
    
    # Get the program ID
    PROGRAM_ID=$(just get-program-id)
    
    # Generate TypeScript SDK
    node -e "
        const fs = require('fs');
        const idl = JSON.parse(fs.readFileSync('{{IDL_PATH}}', 'utf8'));
        const ts = 'export type Feels = ' + JSON.stringify(idl, null, 2) + ';\\n\\nexport const IDL: Feels = ' + JSON.stringify(idl, null, 2) + ';\\n\\nexport const PROGRAM_ID = \"${PROGRAM_ID}\";';
        fs.writeFileSync('{{GENERATED_SDK_PATH}}/feels.ts', ts);
        console.log('TypeScript IDL generated successfully with program ID: ${PROGRAM_ID}');
    "
    
    # Generate TypeScript types
    if anchor idl type -o "{{GENERATED_SDK_PATH}}/feels_types.ts" "{{IDL_PATH}}" 2>/dev/null; then
        echo "  TypeScript types generated"
    else
        just show-warning "Type generation failed (this is expected with Anchor 0.31.1)"
    fi

# Validate IDL exists
[private]
validate-idl-exists:
    #!/usr/bin/env bash
    if [[ ! -f "{{IDL_PATH}}" ]]; then
        just show-error "IDL not found at {{IDL_PATH}}"
        echo "Run 'just build' first to generate the IDL"
        exit {{EXIT_CONFIG_ERROR}}
    fi

# Generate Rust client
[private]
generate-rust-client:
    #!/usr/bin/env bash
    just show-progress "Generating Rust client bindings"
    
    mkdir -p {{GENERATED_SDK_PATH}}/rust/src
    
    # Create Cargo.toml for Rust client
    cat > {{GENERATED_SDK_PATH}}/rust/Cargo.toml << 'EOF'
    [package]
    name = "feels-client"
    version = "0.1.0"
    edition = "2021"
    
    [dependencies]
    anchor-client = "0.31.1"
    anchor-lang = "0.31.1"
    solana-sdk = "2.3.0"
    serde = { version = "1.0", features = ["derive"] }
    serde_json = "1.0"
    
    [features]
    cpi = ["anchor-lang/cpi"]
    EOF
    
    # Get program ID
    PROGRAM_ID=$(just get-program-id 2>/dev/null || echo "")
    
    # Create Rust client lib.rs
    cat > {{GENERATED_SDK_PATH}}/rust/src/lib.rs << 'RUST_EOF'
    //! Auto-generated Rust client for Feels Protocol
    #![allow(dead_code)]
    
    use anchor_client::{
        solana_sdk::{
            instruction::Instruction,
            pubkey::Pubkey,
            signature::{Keypair, Signer},
            system_program,
        },
        Client, ClientError, Cluster, Program,
    };
    use std::rc::Rc;
    use std::str::FromStr;
    
    pub mod types {
        use super::*;
        use anchor_lang::prelude::*;
        
        // Re-export instruction parameter types
        // These would be generated from the IDL in a full implementation
        
        #[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
        pub struct SwapParams {
            pub amount_in: u64,
            pub min_amount_out: u64,
            pub sqrt_price_limit: Option<u128>,
            pub is_token_0_in: bool,
            pub is_exact_in: bool,
        }
        
        #[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
        pub struct InitializeMarketParams {
            pub fee_tier: u16,
            pub tick_spacing: u16,
            pub initial_sqrt_price: u128,
            pub initial_buy_feelssol_amount: u64,
        }
    }
    
    RUST_EOF
    
    # Add program ID
    echo "pub const PROGRAM_ID: &str = \"${PROGRAM_ID}\";" >> {{GENERATED_SDK_PATH}}/rust/src/lib.rs
    
    # Add client struct and basic implementation
    cat >> {{GENERATED_SDK_PATH}}/rust/src/lib.rs << 'RUST_EOF'
    
    pub struct FeelsClient {
        program: Program<Rc<Keypair>>,
    }
    
    type ClientResult<T> = Result<T, ClientError>;
    
    impl FeelsClient {
        pub fn new(
            cluster: Cluster,
            payer: Rc<Keypair>,
        ) -> ClientResult<Self> {
            let client = Client::new(cluster, payer.clone());
            let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();
            let program = client.program(program_id)?;
            
            Ok(Self { program })
        }
        
        pub fn new_with_program_id(
            cluster: Cluster,
            payer: Rc<Keypair>,
            program_id: Pubkey,
        ) -> ClientResult<Self> {
            let client = Client::new(cluster, payer.clone());
            let program = client.program(program_id)?;
            
            Ok(Self { program })
        }
        
        pub fn program(&self) -> &Program<Rc<Keypair>> {
            &self.program
        }
    
        // Example instruction builders
        pub fn initialize_market(
            &self,
            deployer: Pubkey,
            token_0: Pubkey,
            token_1: Pubkey,
            feelssol_mint: Pubkey,
            params: types::InitializeMarketParams,
        ) -> ClientResult<Instruction> {
            // In a full implementation, this would use the IDL to build the instruction
            todo!("Implement based on IDL")
        }
        
        pub fn swap(
            &self,
            user: Pubkey,
            market: Pubkey,
            user_token_in: Pubkey,
            user_token_out: Pubkey,
            params: types::SwapParams,
        ) -> ClientResult<Instruction> {
            // In a full implementation, this would use the IDL to build the instruction
            todo!("Implement based on IDL")
        }
    }
    
    // Include the IDL as a constant
    pub const IDL_JSON: &str = include_str!("../../../target/idl/feels.json");
    
    #[cfg(test)]
    mod tests {
        use super::*;
        
        #[test]
        fn test_parse_idl() {
            let idl: serde_json::Value = serde_json::from_str(IDL_JSON).unwrap();
            assert!(idl.is_object());
        }
        
        #[test]
        fn test_program_id() {
            let program_id = Pubkey::from_str(PROGRAM_ID);
            assert!(program_id.is_ok());
        }
    }
    RUST_EOF
    
    echo "  Rust client generated"

# Show client locations
[private]
show-client-locations:
    @echo ""
    @echo "Generated clients:"
    @echo "  TypeScript: {{GENERATED_SDK_PATH}}/feels.ts"
    @echo "  TypeScript Types: {{GENERATED_SDK_PATH}}/feels_types.ts (if generated)"
    @echo "  Rust: {{GENERATED_SDK_PATH}}/rust/"
    @echo ""
    @echo "To use the TypeScript client:"
    @echo "  import { IDL, PROGRAM_ID } from './{{GENERATED_SDK_PATH}}/feels';"
    @echo "  const program = new anchor.Program(IDL, PROGRAM_ID, provider);"
    @echo ""
    @echo "To use the Rust client:"
    @echo "  Add to your Cargo.toml:"
    @echo "    feels-client = { path = \"./{{GENERATED_SDK_PATH}}/rust\" }"
    @echo ""
    @echo "  Then in your code:"
    @echo "    use feels_client::{FeelsClient, PROGRAM_ID};"
    @echo "    let client = FeelsClient::new(cluster, payer)?;"

# ===== Validation =====

# Validate IDL against expected structure
_idl-validate:
    #!/usr/bin/env bash
    just show-progress "Validating IDL consistency"
    
    if [[ ! -f "{{IDL_PATH}}" ]]; then
        just show-error "IDL not found at {{IDL_PATH}}"
        echo "   Run 'just idl-build' to generate the IDL first"
        exit {{EXIT_CONFIG_ERROR}}
    fi
    
    echo "Found in IDL:"
    echo "   - $(jq '.instructions | length' {{IDL_PATH}}) instructions"
    echo "   - $(jq '.accounts | length' {{IDL_PATH}}) accounts"
    echo "   - $(jq '.types | length' {{IDL_PATH}}) types"
    echo ""
    
    # Check critical instructions
    just show-progress "Checking critical instructions"
    MISSING=""
    for ix in initialize_market swap open_position close_position collect_fees enter_feelssol exit_feelssol; do
        if ! jq -e ".instructions[] | select(.name == \"$ix\")" {{IDL_PATH}} >/dev/null 2>&1; then
            MISSING="$MISSING $ix"
        fi
    done
    
    if [[ -z "$MISSING" ]]; then
        echo "   All critical instructions found"
    else
        just show-error "Missing instructions:$MISSING"
        exit {{EXIT_BUILD_FAILURE}}
    fi
    
    # Check critical account types
    echo ""
    just show-progress "Checking critical account types"
    MISSING=""
    for acc in Market Position Buffer TickArray; do
        if ! jq -e ".accounts[] | select(.name | endswith(\"::$acc\"))" {{IDL_PATH}} >/dev/null 2>&1; then
            MISSING="$MISSING $acc"
        fi
    done
    
    if [[ -z "$MISSING" ]]; then
        echo "   All critical account types found"
    else
        just show-error "Missing accounts:$MISSING"
        exit {{EXIT_BUILD_FAILURE}}
    fi
    
    just show-success "IDL validation passed!"

# Compare IDL vs SDK instruction discriminators
_compare-idl-sdk:
    #!/usr/bin/env bash
    just show-progress "Comparing IDL vs SDK instruction discriminators"
    
    # Check prerequisites
    if [[ ! -f "{{IDL_PATH}}" ]]; then
        just show-error "IDL file not found at {{IDL_PATH}}"
        echo "Run 'just build' first to generate the IDL"
        exit {{EXIT_CONFIG_ERROR}}
    fi
    
    SDK_DIR="sdk/src/instructions"
    if [[ ! -d "$SDK_DIR" ]]; then
        just show-error "SDK instructions directory not found at $SDK_DIR"
        exit {{EXIT_CONFIG_ERROR}}
    fi
    
    if ! command -v jq >/dev/null 2>&1; then
        just show-error "jq is required but not installed"
        exit {{EXIT_MISSING_DEPS}}
    fi
    
    # Parse IDL instructions
    echo "Parsing IDL..."
    IDL_TEMP=$(mktemp)
    jq -r '.instructions[] | "\(.name):\(.discriminator | map(tostring) | join(","))"' "{{IDL_PATH}}" > "$IDL_TEMP"
    IDL_COUNT=$(wc -l < "$IDL_TEMP")
    echo "Found $IDL_COUNT instructions in IDL"
    
    # Find SDK discriminators
    echo "Scanning SDK files..."
    SDK_TEMP=$(mktemp)
    
    for file in "$SDK_DIR"/*.rs; do
        if [[ ! -f "$file" ]] || [[ "$(basename "$file")" == "mod.rs" ]]; then
            continue
        fi
        
        while IFS= read -r line; do
            if [[ "$line" =~ _DISCRIMINATOR:.*\[u8 ]]; then
                # Extract const name and discriminator values
                const_name=$(echo "$line" | sed -n 's/.*const \([A-Z_]*\)_DISCRIMINATOR.*/\1/p')
                disc_values=$(echo "$line" | sed -n 's/.*= \[\([^]]*\)\].*/\1/p')
                
                if [[ -n "$const_name" && -n "$disc_values" ]]; then
                    # Convert to lowercase instruction name
                    instruction_name=$(echo "$const_name" | tr '[:upper:]' '[:lower:]')
                    
                    # Handle both hex (0x) and decimal formats
                    if [[ "$disc_values" =~ 0x ]]; then
                        # Convert hex to decimal
                        clean_disc=""
                        IFS=',' read -ra values <<< "$(echo "$disc_values" | sed 's/[[:space:]]//g')"
                        for val in "${values[@]}"; do
                            hex_val=$(echo "$val" | sed 's/0x//')
                            if [[ -n "$hex_val" ]]; then
                                decimal=$(echo "ibase=16; ${hex_val^^}" | bc)
                                clean_disc="${clean_disc}${decimal},"
                            fi
                        done
                        clean_disc=${clean_disc%,}
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
    echo ""
    
    # Sort files for comparison
    sort "$IDL_TEMP" -o "$IDL_TEMP"
    sort "$SDK_TEMP" -o "$SDK_TEMP"
    
    # Compare instructions
    missing_count=0
    mismatch_count=0
    correct_count=0
    
    # Read the IDL temp file line by line
    while IFS=: read -r idl_name idl_disc; do
        # Look for SDK line
        sdk_line=$(fgrep "${idl_name}:" "$SDK_TEMP" 2>/dev/null | head -1 || true)
        
        if [[ -z "$sdk_line" ]]; then
            ((missing_count++))
            echo "  Missing in SDK: $idl_name"
        else
            sdk_disc="${sdk_line#*:}"
            
            # Normalize for comparison
            norm_idl=$(echo "$idl_disc" | sed 's/[[:space:]]//g')
            norm_sdk=$(echo "$sdk_disc" | sed 's/[[:space:]]//g')
            
            if [[ "$norm_idl" == "$norm_sdk" ]]; then
                ((correct_count++))
            else
                ((mismatch_count++))
                echo "  Mismatch: $idl_name (IDL: $norm_idl, SDK: $norm_sdk)"
            fi
        fi
    done < "$IDL_TEMP"
    
    # Print summary
    echo ""
    echo "Summary:"
    echo "- Total IDL instructions: $IDL_COUNT"
    echo "- Missing in SDK: $missing_count"
    echo "- Discriminator mismatches: $mismatch_count"
    echo "- Correctly implemented: $correct_count"
    
    # Cleanup
    rm -f "$IDL_TEMP" "$SDK_TEMP"
    
    if [[ $missing_count -gt 0 ]] || [[ $mismatch_count -gt 0 ]]; then
        just show-warning "Found discrepancies between IDL and SDK"
        exit {{EXIT_BUILD_FAILURE}}
    else
        just show-success "IDL and SDK are in sync!"
    fi

# ===== Clean Commands =====

# Clean build artifacts
_clean:
    @just show-progress "Cleaning build artifacts"
    @cargo clean || true
    @rm -rf target/ .anchor/
    @just show-success "Build artifacts cleaned"

# Reset development environment
_reset:
    @just show-progress "Resetting local development environment"
    @just _clean
    @rm -rf {{LOGS_PATH}}/ test-ledger/ {{KEYPAIRS_PATH}}/
    @just show-success "Reset complete!"