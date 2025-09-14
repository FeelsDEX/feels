#!/bin/bash

# Test Reorganization Script for Feels Protocol
# This script reorganizes the test directory structure according to the cleanup proposal
# Usage: ./reorganize_tests.sh [--dry-run]

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if dry run mode
DRY_RUN=false
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=true
    echo -e "${YELLOW}Running in DRY RUN mode - no files will be modified${NC}"
    echo
fi

# Base directory (script should be run from tests directory)
if [[ ! -f "mod.rs" ]] || [[ ! -d "common" ]]; then
    echo -e "${RED}Error: This script must be run from the tests/ directory${NC}"
    exit 1
fi

# Function to create directory
create_dir() {
    local dir="$1"
    if [[ ! -d "$dir" ]]; then
        echo -e "${BLUE}Creating directory:${NC} $dir"
        if [[ "$DRY_RUN" == false ]]; then
            mkdir -p "$dir"
        fi
    fi
}

# Function to move file
move_file() {
    local src="$1"
    local dst="$2"
    
    if [[ -f "$src" ]]; then
        echo -e "${GREEN}Moving:${NC} $src → $dst"
        if [[ "$DRY_RUN" == false ]]; then
            mv "$src" "$dst"
        fi
    else
        echo -e "${YELLOW}Warning:${NC} Source file not found: $src"
    fi
}

# Function to remove directory
remove_dir() {
    local dir="$1"
    if [[ -d "$dir" ]]; then
        echo -e "${RED}Removing empty directory:${NC} $dir"
        if [[ "$DRY_RUN" == false ]]; then
            rmdir "$dir" 2>/dev/null || echo -e "${YELLOW}Warning:${NC} Directory not empty: $dir"
        fi
    fi
}

echo "Starting test reorganization..."
echo "================================"
echo

# Step 1: Create new directory structure
echo "Step 1: Creating new directories..."
create_dir "unit/math"
create_dir "unit/security"
create_dir "unit/pomm"
create_dir "unit/buffer"
create_dir "unit/position"
create_dir "unit/oracle"
create_dir "integration/initialization"
create_dir "integration/liquidity"
create_dir "integration/debug"
create_dir "integration/token"
echo

# Step 2: Move root-level tests
echo "Step 2: Moving root-level tests..."
move_file "test_debug.rs" "integration/debug/test_basic_debug.rs"
move_file "test_pomm_isolated.rs" "unit/pomm/test_pomm_isolated.rs"
move_file "test_stair_pattern_liquidity.rs" "integration/liquidity/test_stair_pattern.rs"
echo

# Step 3: Rename IDL generation file
echo "Step 3: Renaming IDL generation file..."
move_file "idl_gen.rs" "test_idl_generation.rs"
echo

# Step 4: Reorganize unit tests
echo "Step 4: Reorganizing unit tests..."

# Math tests
move_file "unit/test_fee_growth.rs" "unit/math/test_fee_growth.rs"
move_file "unit/test_fee_rounding.rs" "unit/math/test_fee_rounding.rs"
move_file "unit/test_dust_control.rs" "unit/math/test_dust_control.rs"

# Security tests
move_file "unit/test_reentrancy_guard.rs" "unit/security/test_reentrancy_guard.rs"
move_file "unit/test_initialization_race_condition_fix.rs" "unit/security/test_initialization_race_condition.rs"
move_file "unit/test_launch_security.rs" "unit/security/test_launch_security.rs"
move_file "unit/test_oracle_timestamp_security.rs" "unit/security/test_oracle_timestamp_security.rs"
move_file "unit/test_tick_array_griefing.rs" "unit/security/test_tick_array_griefing.rs"

# POMM tests
move_file "unit/test_pomm_saturation.rs" "unit/pomm/test_pomm_saturation.rs"
move_file "unit/test_pomm_security.rs" "unit/pomm/test_pomm_security.rs"

# Buffer tests
move_file "unit/test_buffer.rs" "unit/buffer/test_buffer.rs"
move_file "unit/test_buffer_overflow.rs" "unit/buffer/test_buffer_overflow.rs"

# Position tests
move_file "unit/test_close_position_safety.rs" "unit/position/test_close_position_safety.rs"

# Oracle tests
move_file "unit/test_observation_offsets.rs" "unit/oracle/test_observation_offsets.rs"
echo

# Step 5: Reorganize integration tests
echo "Step 5: Reorganizing integration tests..."

# Rename files with inconsistent naming
move_file "integration/basic_test.rs" "integration/test_basic_operations.rs"
move_file "integration/example_new_infrastructure.rs" "integration/test_new_infrastructure_example.rs"
move_file "integration/mint_token_test.rs" "integration/token/test_token_minting.rs"

# Move initialization tests
move_file "integration/initialize_market_test.rs" "integration/initialization/test_basic_market.rs"
move_file "integration/test_minimal_initialize_market.rs" "integration/initialization/test_minimal_market.rs"
move_file "integration/test_raw_initialize_market.rs" "integration/initialization/test_raw_market.rs"
move_file "integration/test_simple_initialize_market.rs" "integration/initialization/test_simple_market.rs"
move_file "integration/test_debug_initialize_market.rs" "integration/initialization/test_debug_market.rs"

# Combine dummy tests into edge cases
echo -e "${BLUE}Note:${NC} The following dummy tests should be manually combined into test_edge_cases.rs:"
echo "  - test_existing_dummy_initialize_market.rs"
echo "  - test_no_dummy_initialize_market.rs"
echo "  - test_system_dummy_initialize_market.rs"
echo

# Step 6: Clean up empty directories
echo "Step 6: Cleaning up empty directories..."
remove_dir "unit/state"
remove_dir "fuzzing"
remove_dir "helpers"
echo

# Step 7: Update module files
echo "Step 7: Module files need manual updates..."
echo -e "${YELLOW}Manual steps required:${NC}"
echo "1. Update unit/mod.rs to include new submodules (math, security, pomm, buffer, position, oracle)"
echo "2. Update integration/mod.rs to include new submodules (initialization, liquidity, debug, token)"
echo "3. Update root mod.rs if needed"
echo "4. Manually combine the dummy initialization tests into integration/initialization/test_edge_cases.rs"
echo "5. Move any unique helpers from helpers/mod.rs to common/helpers.rs before deleting helpers/"
echo

# Step 8: Create mod.rs files for new directories
echo "Step 8: Creating mod.rs files for new directories..."

# Function to create a mod.rs file
create_mod_file() {
    local dir="$1"
    local mod_file="$dir/mod.rs"
    
    if [[ ! -f "$mod_file" ]]; then
        echo -e "${BLUE}Creating mod.rs:${NC} $mod_file"
        if [[ "$DRY_RUN" == false ]]; then
            # Extract test module names from the directory
            echo "// Test modules for $(basename $dir)" > "$mod_file"
            echo "" >> "$mod_file"
            
            # Find all test files and create module declarations
            for test_file in "$dir"/test_*.rs; do
                if [[ -f "$test_file" ]]; then
                    module_name=$(basename "$test_file" .rs)
                    echo "pub mod $module_name;" >> "$mod_file"
                fi
            done
        fi
    fi
}

# Create mod.rs files for new directories
create_mod_file "unit/math"
create_mod_file "unit/security"
create_mod_file "unit/pomm"
create_mod_file "unit/buffer"
create_mod_file "unit/position"
create_mod_file "unit/oracle"
create_mod_file "integration/initialization"
create_mod_file "integration/liquidity"
create_mod_file "integration/debug"
create_mod_file "integration/token"
echo

# Summary
echo "================================"
if [[ "$DRY_RUN" == true ]]; then
    echo -e "${YELLOW}DRY RUN COMPLETE${NC}"
    echo "No files were modified. Run without --dry-run to apply changes."
else
    echo -e "${GREEN}REORGANIZATION COMPLETE${NC}"
    echo "Files have been reorganized. Please complete the manual steps listed above."
fi
echo

# Final tree view (if tree command is available)
if command -v tree &> /dev/null; then
    echo "New structure preview:"
    tree -L 3 -I '__pycache__|*.pyc|target'
fi