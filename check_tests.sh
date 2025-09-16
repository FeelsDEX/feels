#!/bin/bash
# Quick test compilation check

echo "Checking test compilation..."

# Check if we're in the right directory
if [ ! -f "Anchor.toml" ]; then
    echo "Error: Not in project root (no Anchor.toml found)"
    exit 1
fi

# Try to compile tests
echo "Running cargo check on tests..."
cargo check --tests 2>&1 | head -100

echo ""
echo "Checking specific test files..."
echo "1. Exact output swap test:"
cargo check --test test_exact_output_swap 2>&1 | head -20

echo ""
echo "2. Helper functions:"
cargo check --lib --tests 2>&1 | grep -A5 -B5 "helpers\|exact_output" | head -30

echo ""
echo "Done!"