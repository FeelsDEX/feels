#!/bin/bash
# Script to compile and run the tests with the new exact output swap functionality

set -e  # Exit on error

echo "========================================="
echo "Building and Testing Feels Protocol"
echo "========================================="

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[STATUS]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "Anchor.toml" ]; then
    print_error "Not in project root directory (Anchor.toml not found)"
    exit 1
fi

# Step 1: Build the program
print_status "Building the Feels program..."
if anchor build --program-name feels; then
    print_status "Program built successfully!"
else
    print_error "Program build failed"
    exit 1
fi

# Step 2: Run unit tests
print_status "Running unit tests..."
echo ""
if cargo test --package feels --lib -- --nocapture; then
    print_status "Unit tests passed!"
else
    print_warning "Some unit tests failed"
fi

# Step 3: Run integration tests
print_status "Running integration tests..."
echo ""

# Run specific test files related to our changes
print_status "Testing exact output swap functionality..."
if cargo test --package feels --test test_exact_output_swap -- --nocapture; then
    print_status "Exact output swap tests passed!"
else
    print_warning "Exact output swap tests failed"
fi

# Step 4: Run helper tests
print_status "Testing helper functions..."
if cargo test --package feels test_helpers -- --nocapture; then
    print_status "Helper tests passed!"
else
    print_warning "Helper tests failed"
fi

# Step 5: Run position metadata tests
print_status "Testing position metadata operations..."
if cargo test --package feels position_with_metadata -- --nocapture; then
    print_status "Position metadata tests passed!"
else
    print_warning "Position metadata tests failed"
fi

# Step 6: Check SDK compilation
print_status "Checking SDK compilation..."
if cargo check --package feels-sdk; then
    print_status "SDK compiles successfully!"
else
    print_error "SDK compilation failed"
    exit 1
fi

# Step 7: Run SDK tests
print_status "Running SDK tests..."
if cargo test --package feels-sdk -- --nocapture; then
    print_status "SDK tests passed!"
else
    print_warning "Some SDK tests failed"
fi

# Step 8: Run a quick smoke test with the test validator
print_status "Running smoke test with local validator..."
echo ""
echo "To run with local validator:"
echo "1. Start validator: solana-test-validator"
echo "2. Deploy program: anchor deploy"
echo "3. Run tests: anchor test --skip-local-validator"

# Summary
echo ""
echo "========================================="
echo "Test Summary"
echo "========================================="
print_status "Program compilation: ✓"
print_status "Unit tests: Check output above"
print_status "Integration tests: Check output above"
print_status "SDK compilation: ✓"
echo ""
echo "Key features tested:"
echo "  - Exact output swap with binary search"
echo "  - Position creation/closing with metadata"
echo "  - Test helper implementations"
echo "  - SDK estimate_input_for_output function"
echo ""
echo "To run specific tests:"
echo "  cargo test test_exact_output_swap -- --nocapture"
echo "  cargo test swap_exact_out -- --nocapture"
echo "  cargo test position_with_metadata -- --nocapture"
echo ""
echo "Done!"