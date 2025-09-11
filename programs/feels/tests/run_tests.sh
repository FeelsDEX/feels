#!/bin/bash
set -e

echo "Running Feels Protocol Test Suite"
echo "================================"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to run test category
run_test_category() {
    local category=$1
    local description=$2
    
    echo -e "\n${YELLOW}Running ${description}...${NC}"
    
    if cargo test --manifest-path ../../Cargo.toml --features test-utils ${category}:: -- --test-threads=1; then
        echo -e "${GREEN}${description} passed${NC}"
    else
        echo -e "${RED}${description} failed${NC}"
        exit 1
    fi
}

# Change to the program directory
cd "$(dirname "$0")/../../"

# Build the program first
echo -e "${YELLOW}Building program...${NC}"
cargo build-sbf

# Run each test category
run_test_category "unit" "Unit Tests"
run_test_category "integration" "Integration Tests"
run_test_category "property" "Property-based Tests"
run_test_category "e2e" "End-to-End Tests"

# Run static assertions separately
echo -e "\n${YELLOW}Running static assertions...${NC}"
if cargo test --manifest-path Cargo.toml static_assertions -- --test-threads=1; then
    echo -e "${GREEN}Static assertions passed${NC}"
else
    echo -e "${RED}Static assertions failed${NC}"
    exit 1
fi

echo -e "\n${GREEN}All tests passed!${NC}"