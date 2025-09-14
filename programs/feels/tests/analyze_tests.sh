#!/bin/bash

# Test Analysis Script for Feels Protocol
# This script analyzes the current test structure and generates a report
# Usage: ./analyze_tests.sh

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Check we're in the right directory
if [[ ! -f "mod.rs" ]] || [[ ! -d "common" ]]; then
    echo -e "${RED}Error: This script must be run from the tests/ directory${NC}"
    exit 1
fi

echo -e "${CYAN}Feels Protocol Test Structure Analysis${NC}"
echo "======================================"
echo

# Function to count files in directory
count_files() {
    local dir="$1"
    local pattern="${2:-*.rs}"
    if [[ -d "$dir" ]]; then
        find "$dir" -maxdepth 1 -name "$pattern" -type f 2>/dev/null | wc -l | tr -d ' '
    else
        echo "0"
    fi
}

# Current structure analysis
echo -e "${BLUE}Current Structure:${NC}"
echo

# Root level tests
echo "Root level test files:"
for file in test_*.rs idl_gen.rs; do
    [[ -f "$file" ]] && echo "  - $file"
done
echo

# Unit tests
echo "Unit tests:"
echo "  Total files: $(count_files unit 'test_*.rs')"
echo "  Categories:"
echo "    - Buffer tests: $(count_files unit 'test_buffer*.rs')"
echo "    - Fee tests: $(count_files unit 'test_fee*.rs')"
echo "    - Security tests: $(ls unit/test_*security*.rs unit/test_*guard*.rs unit/test_*griefing*.rs 2>/dev/null | wc -l | tr -d ' ')"
echo "    - POMM tests: $(count_files unit 'test_pomm*.rs')"
echo "    - Other tests: $(ls unit/test_*.rs 2>/dev/null | grep -v -E 'buffer|fee|security|guard|griefing|pomm' | wc -l | tr -d ' ')"
echo "  Empty directories:"
[[ -d "unit/state" ]] && [[ -z "$(ls -A unit/state 2>/dev/null)" ]] && echo "    - unit/state/"
[[ -d "unit/instructions" ]] && echo "    - unit/instructions/ (1 disabled test)"
echo

# Integration tests
echo "Integration tests:"
echo "  Total files: $(find integration -name '*.rs' -not -name 'mod.rs' 2>/dev/null | wc -l | tr -d ' ')"
echo "  Market initialization tests: $(find integration -name '*initialize_market*.rs' 2>/dev/null | wc -l | tr -d ' ')"
echo "  Other tests: $(find integration -name '*.rs' -not -name 'mod.rs' -not -name '*initialize_market*' 2>/dev/null | wc -l | tr -d ' ')"
echo

# E2E tests
echo "E2E tests:"
echo "  Total files: $(count_files e2e 'test_*.rs')"
echo

# Property tests
echo "Property tests:"
echo "  Total files: $(count_files property '*.rs' | grep -v mod.rs | wc -l | tr -d ' ')"
echo

# Other directories
echo "Other directories:"
[[ -d "fuzzing" ]] && echo "  - fuzzing/ (empty)"
[[ -d "helpers" ]] && echo "  - helpers/ ($(count_files helpers) files)"
echo

# Files that will be moved/renamed
echo -e "${YELLOW}Proposed Changes:${NC}"
echo

echo "Files to be moved from root:"
[[ -f "test_debug.rs" ]] && echo "  • test_debug.rs → integration/debug/test_basic_debug.rs"
[[ -f "test_pomm_isolated.rs" ]] && echo "  • test_pomm_isolated.rs → unit/pomm/test_pomm_isolated.rs"
[[ -f "test_stair_pattern_liquidity.rs" ]] && echo "  • test_stair_pattern_liquidity.rs → integration/liquidity/test_stair_pattern.rs"
[[ -f "idl_gen.rs" ]] && echo "  • idl_gen.rs → test_idl_generation.rs"
echo

echo "Unit tests to be reorganized into subdirectories:"
echo "  Math category:"
[[ -f "unit/test_fee_growth.rs" ]] && echo "    • test_fee_growth.rs → math/test_fee_growth.rs"
[[ -f "unit/test_fee_rounding.rs" ]] && echo "    • test_fee_rounding.rs → math/test_fee_rounding.rs"
[[ -f "unit/test_dust_control.rs" ]] && echo "    • test_dust_control.rs → math/test_dust_control.rs"

echo "  Security category:"
[[ -f "unit/test_reentrancy_guard.rs" ]] && echo "    • test_reentrancy_guard.rs → security/test_reentrancy_guard.rs"
[[ -f "unit/test_initialization_race_condition_fix.rs" ]] && echo "    • test_initialization_race_condition_fix.rs → security/test_initialization_race_condition.rs"
[[ -f "unit/test_launch_security.rs" ]] && echo "    • test_launch_security.rs → security/test_launch_security.rs"
echo

echo "Integration tests to be renamed:"
[[ -f "integration/basic_test.rs" ]] && echo "  • basic_test.rs → test_basic_operations.rs"
[[ -f "integration/example_new_infrastructure.rs" ]] && echo "  • example_new_infrastructure.rs → test_new_infrastructure_example.rs"
[[ -f "integration/mint_token_test.rs" ]] && echo "  • mint_token_test.rs → token/test_token_minting.rs"
echo

echo "Directories to be removed:"
[[ -d "fuzzing" ]] && echo "  • fuzzing/ (empty)"
[[ -d "unit/state" ]] && [[ -z "$(ls -A unit/state 2>/dev/null)" ]] && echo "  • unit/state/ (empty)"
[[ -d "helpers" ]] && echo "  • helpers/ (after moving contents to common/helpers.rs)"
echo

# Summary statistics
echo -e "${GREEN}Summary:${NC}"
total_tests=$(find . -name '*.rs' -not -path './common/*' -not -name 'mod.rs' | wc -l | tr -d ' ')
echo "  Total test files: $total_tests"
echo "  Files to be moved: ~15-20"
echo "  Files to be renamed: ~8-10"
echo "  New directories to create: 10"
echo "  Directories to remove: 3"
echo

echo -e "${CYAN}To see what changes would be made, run:${NC}"
echo "  ./reorganize_tests.sh --dry-run"
echo
echo -e "${CYAN}To apply the changes, run:${NC}"
echo "  ./reorganize_tests.sh"
echo