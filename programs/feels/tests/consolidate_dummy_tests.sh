#!/bin/bash

# Script to consolidate dummy initialization tests
# This should be run after the main reorganization script
# Usage: ./consolidate_dummy_tests.sh [--dry-run]

set -euo pipefail

# Colors for output
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

# Target directory
TARGET_DIR="integration/initialization"
TARGET_FILE="$TARGET_DIR/test_edge_cases.rs"

# Source files to consolidate
DUMMY_FILES=(
    "integration/test_existing_dummy_initialize_market.rs"
    "integration/test_no_dummy_initialize_market.rs"
    "integration/test_system_dummy_initialize_market.rs"
)

echo "Consolidating dummy initialization tests..."
echo "=========================================="
echo

# Check if source files exist
echo "Checking source files..."
found_files=()
for file in "${DUMMY_FILES[@]}"; do
    if [[ -f "$file" ]]; then
        echo -e "${GREEN}Found:${NC} $file"
        found_files+=("$file")
    else
        echo -e "${YELLOW}Not found:${NC} $file"
    fi
done
echo

if [[ ${#found_files[@]} -eq 0 ]]; then
    echo -e "${YELLOW}No dummy test files found to consolidate.${NC}"
    exit 0
fi

# Create consolidated file
echo -e "${BLUE}Creating consolidated test file:${NC} $TARGET_FILE"

if [[ "$DRY_RUN" == false ]]; then
    # Ensure target directory exists
    mkdir -p "$TARGET_DIR"
    
    # Create the consolidated file
    cat > "$TARGET_FILE" << 'EOF'
// Consolidated edge case tests for market initialization
// This file combines various dummy and edge case initialization tests

use crate::common::*;

mod existing_dummy_tests {
    use super::*;
    
    // Tests from test_existing_dummy_initialize_market.rs
    #[test]
    fn test_initialize_with_existing_dummy() {
        // TODO: Import test content from test_existing_dummy_initialize_market.rs
    }
}

mod no_dummy_tests {
    use super::*;
    
    // Tests from test_no_dummy_initialize_market.rs
    #[test]
    fn test_initialize_without_dummy() {
        // TODO: Import test content from test_no_dummy_initialize_market.rs
    }
}

mod system_dummy_tests {
    use super::*;
    
    // Tests from test_system_dummy_initialize_market.rs
    #[test]
    fn test_initialize_with_system_dummy() {
        // TODO: Import test content from test_system_dummy_initialize_market.rs
    }
}

// Additional edge cases
mod other_edge_cases {
    use super::*;
    
    #[test]
    fn test_initialize_with_invalid_params() {
        // Test initialization with various invalid parameters
    }
    
    #[test]
    fn test_initialize_duplicate_market() {
        // Test attempting to initialize an already initialized market
    }
}
EOF

    echo -e "${GREEN}Created consolidated file template${NC}"
    echo
    
    # Extract test functions from each file and append as comments
    echo "Extracting test content from source files..."
    
    for file in "${found_files[@]}"; do
        echo -e "${BLUE}Processing:${NC} $file"
        echo "" >> "$TARGET_FILE"
        echo "// ============================================" >> "$TARGET_FILE"
        echo "// Content from $file" >> "$TARGET_FILE"
        echo "// ============================================" >> "$TARGET_FILE"
        echo "/*" >> "$TARGET_FILE"
        
        # Extract test functions (simple extraction, may need manual cleanup)
        grep -A 50 "#\[test\]" "$file" >> "$TARGET_FILE" 2>/dev/null || true
        
        echo "*/" >> "$TARGET_FILE"
    done
    
    echo
    echo -e "${GREEN}Consolidation complete!${NC}"
else
    echo -e "${YELLOW}Would create:${NC} $TARGET_FILE"
fi

echo
echo -e "${YELLOW}Manual steps required:${NC}"
echo "1. Edit $TARGET_FILE to properly integrate the test content"
echo "2. Update imports and module structure as needed"
echo "3. Remove the original dummy test files:"
for file in "${found_files[@]}"; do
    echo "   rm $file"
done
echo "4. Update integration/mod.rs to remove references to the old files"
echo

if [[ "$DRY_RUN" == true ]]; then
    echo -e "${YELLOW}This was a dry run. Run without --dry-run to create the file.${NC}"
fi