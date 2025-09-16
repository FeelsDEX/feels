#!/bin/bash
# Diagnose potential compilation issues

echo "=== Diagnostic Report for Feels Protocol Tests ==="
echo ""

# Check file existence
echo "1. Checking if all modified files exist..."
files=(
    "sdk/src/exact_output_swap.rs"
    "sdk/src/lib.rs"
    "sdk/src/error.rs"
    "programs/feels/tests/common/helpers.rs"
    "programs/feels/tests/integration/test_exact_output_swap.rs"
    "programs/feels/tests/unit/test_helpers.rs"
)

for file in "${files[@]}"; do
    if [ -f "$file" ]; then
        echo "  ✓ $file exists"
    else
        echo "  ✗ $file NOT FOUND"
    fi
done

echo ""
echo "2. Checking for potential import issues..."

# Check if mpl_token_metadata needs explicit import
if grep -q "mpl_token_metadata::ID" programs/feels/tests/common/helpers.rs; then
    echo "  ⚠ helpers.rs uses mpl_token_metadata::ID"
    echo "    Checking if it's imported..."
    if grep -q "use.*mpl_token_metadata" programs/feels/tests/common/helpers.rs; then
        echo "    ✓ Explicit import found"
    else
        echo "    ⚠ No explicit import - may rely on parent module"
    fi
fi

echo ""
echo "3. Checking for common syntax issues..."

# Check for common Rust syntax errors
echo "  Checking for unclosed brackets..."
for file in "${files[@]}"; do
    if [ -f "$file" ]; then
        open_braces=$(grep -o '{' "$file" | wc -l)
        close_braces=$(grep -o '}' "$file" | wc -l)
        if [ "$open_braces" -ne "$close_braces" ]; then
            echo "    ⚠ $file: Brace mismatch ({: $open_braces, }: $close_braces)"
        fi
    fi
done

echo ""
echo "4. Checking for TODO/FIXME comments..."
grep -n "TODO\|FIXME" programs/feels/tests/common/helpers.rs 2>/dev/null || echo "  ✓ No TODO/FIXME in helpers.rs"

echo ""
echo "5. Checking Market struct compatibility..."
echo "  Fields in test_helpers.rs:"
grep -A70 "pub fn create_test_market" programs/feels/tests/unit/test_helpers.rs | grep -E "^\s+[a-z_]+:" | head -20

echo ""
echo "6. Quick fixes to try if compilation fails:"
echo ""
echo "  a) If mpl_token_metadata not found:"
echo "     Add to programs/feels/tests/common/mod.rs:"
echo "     extern crate mpl_token_metadata;"
echo ""
echo "  b) If SwapResult conflicts:"
echo "     Check if it's already defined elsewhere"
echo "     Use crate::common::SwapResult explicitly"
echo ""
echo "  c) If SDK functions not found:"
echo "     Ensure sdk is properly imported:"
echo "     use feels_sdk as sdk;"
echo ""
echo "  d) Run these commands:"
echo "     cargo clean"
echo "     cargo update"
echo "     anchor build --program-name feels"
echo "     cargo test --no-run"
echo ""

# Create a minimal test file to verify basic compilation
cat > /tmp/minimal_test.rs << 'EOF'
#[cfg(test)]
mod tests {
    #[test]
    fn test_basic() {
        assert_eq!(2 + 2, 4);
    }
}
EOF

echo "7. Minimal compilation test created at /tmp/minimal_test.rs"
echo "   You can test basic Rust setup with: rustc --test /tmp/minimal_test.rs"
echo ""
echo "=== End of Diagnostic Report ==="