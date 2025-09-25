# Development workflow recipes

# Import dependencies
import 'common.just'

# ===== Quick Development Commands =====

# Quick development setup (internal - use 'just dev' from main justfile)
_dev: _format _lint _test-unit
    @just show-success "Development checks passed"

# Full CI workflow (internal - use 'just ci' from main justfile)
_ci: _format _lint test build idl-validate
    @just show-success "CI pipeline passed"

# ===== Code Quality =====

# Format all code (internal - use 'just format' from main justfile)
_format:
    @just show-progress "Formatting Rust code"
    @cargo fmt --all
    @just show-progress "Formatting Nix files"
    @nixpkgs-fmt nix/*.nix 2>/dev/null || true
    @just show-success "Code formatted"

# Run linter (internal - use 'just lint' from main justfile)
_lint:
    @just show-progress "Running clippy"
    @if cargo clippy --all-targets --all-features -- -W clippy::all -A clippy::too_many_arguments -A clippy::result_large_err -A clippy::cast_abs_to_unsigned -A clippy::unnecessary_cast -A clippy::doc_lazy_continuation -A clippy::derivable_impls -A clippy::manual_div_ceil -A clippy::manual_range_contains; then \
        just show-success "Lint checks passed"; \
    else \
        just exit-with-error {{EXIT_BUILD_FAILURE}} "Lint errors found"; \
    fi

# Check code without building
cargo-check:
    @just show-progress "Running cargo check"
    @cargo check --all-targets

# ===== Documentation =====

# Generate and open documentation (internal - use 'just docs' from main justfile)
_docs:
    @just show-progress "Generating documentation"
    @cargo doc --no-deps --open

# Generate documentation without opening
docs-build:
    @just show-progress "Generating documentation"
    @cargo doc --no-deps

# ===== Dependency Management =====

# Update dependencies
update-deps:
    @just show-progress "Updating Cargo dependencies"
    @cargo update
    @just show-progress "Updating npm dependencies"
    @cd {{FEELS_APP_PATH}} && npm update
    @cd {{E2E_PATH}}/streaming-adapter && npm update
    @just show-success "Dependencies updated"

# Audit dependencies for vulnerabilities
audit:
    @just show-progress "Auditing Cargo dependencies"
    @cargo audit || just show-warning "Some vulnerabilities found in Cargo deps"
    @just show-progress "Auditing npm dependencies"
    @cd {{FEELS_APP_PATH}} && npm audit || just show-warning "Some vulnerabilities found in npm deps"

# ===== Workspace Management =====

# Show workspace information
workspace-info:
    @echo "Workspace Structure:"
    @echo "==================="
    @echo "Programs:"
    @cargo tree --depth 1 -p feels 2>/dev/null | head -5
    @cargo tree --depth 1 -p feels-jupiter-adapter 2>/dev/null | head -5
    @echo ""
    @echo "SDK:"
    @cargo tree --depth 1 -p feels-sdk 2>/dev/null | head -5
    @echo ""
    @echo "Supporting:"
    @echo "  - Frontend: {{FEELS_APP_PATH}}"
    @echo "  - Indexer: {{PROJECT_ROOT}}/feels-indexer"
    @echo "  - E2E: {{E2E_PATH}}"

# Clean all workspace artifacts
clean-workspace: clean
    @just show-progress "Cleaning all workspace artifacts"
    @cd {{FEELS_APP_PATH}} && just clean
    @cd {{PROJECT_ROOT}}/feels-indexer && cargo clean
    @cd {{E2E_PATH}}/streaming-adapter && rm -rf node_modules
    @rm -rf {{GENERATED_SDK_PATH}}
    @rm -rf {{TEST_LEDGER_PATH}}
    @just show-success "Workspace cleaned"

# ===== Release Management =====

# Prepare release
prepare-release version:
    #!/usr/bin/env bash
    just show-progress "Preparing release {{version}}"
    
    # Run all checks
    just ci
    
    # Update version in Cargo.toml files
    for cargo_file in $(find . -name Cargo.toml -not -path "./target/*"); do
        if grep -q "^version = " "$cargo_file"; then
            sed -i.bak "s/^version = .*/version = \"{{version}}\"/" "$cargo_file"
            rm "${cargo_file}.bak"
            echo "  Updated: $cargo_file"
        fi
    done
    
    # Update package.json files
    for package_file in $(find . -name package.json -not -path "*/node_modules/*"); do
        if [[ -f "$package_file" ]]; then
            jq ".version = \"{{version}}\"" "$package_file" > "$package_file.tmp"
            mv "$package_file.tmp" "$package_file"
            echo "  Updated: $package_file"
        fi
    done
    
    just show-success "Release {{version}} prepared"
    echo ""
    echo "Next steps:"
    echo "1. Review changes with 'git diff'"
    echo "2. Commit with 'git commit -am \"Release v{{version}}\"'"
    echo "3. Tag with 'git tag -a v{{version}} -m \"Release v{{version}}\"'"
    echo "4. Push with 'git push && git push --tags'"

# ===== Development Helpers =====

# Watch for changes and run tests
watch:
    @just show-progress "Watching for changes"
    @cargo watch -x "test -p feels test_unit_" -x "clippy"

# Run benchmarks
bench:
    @just show-progress "Running benchmarks"
    @cargo bench

# Profile test execution
profile-tests:
    @just show-progress "Profiling test execution"
    @cargo test --profile profiling -- --profile-time

# Generate test coverage (requires cargo-tarpaulin)
coverage:
    @just show-progress "Generating test coverage"
    @cargo tarpaulin --out Html --output-dir {{LOGS_PATH}}/coverage

# ===== Environment Setup =====

# Setup development environment
setup-dev:
    @just show-progress "Setting up development environment"
    @just check-command cargo "Rust/Cargo"
    @just check-command anchor "Anchor framework"
    @just check-command node "Node.js"
    @just check-command solana "Solana CLI"
    @mkdir -p {{LOGS_PATH}} {{KEYPAIRS_PATH}} {{GENERATED_SDK_PATH}}
    @just show-success "Development environment ready"

# Install development tools
install-dev-tools:
    @just show-progress "Installing development tools"
    @cargo install cargo-watch cargo-audit cargo-tarpaulin || true
    @just show-success "Development tools installed"

# ===== Debugging =====

# Debug build with all features
debug-build:
    @just show-progress "Building with debug symbols"
    @RUST_BACKTRACE=1 cargo build --all-features

# Run with debug logging
debug-run test="":
    #!/usr/bin/env bash
    export RUST_LOG=debug
    export RUST_BACKTRACE=full
    if [[ -z "{{test}}" ]]; then
        just show-error "Please specify a test to run"
        exit {{EXIT_GENERAL_ERROR}}
    fi
    just show-progress "Running {{test}} with debug logging"
    cargo test {{test}} -- --nocapture

# ===== Git Helpers =====

# Show files changed in current branch
changes:
    @git diff --name-status main...HEAD

# Run pre-commit checks
pre-commit: _format _lint _test-unit
    @just show-success "Pre-commit checks passed"