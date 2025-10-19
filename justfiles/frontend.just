# Frontend application module

# Import dependencies
import 'common.just'

# Frontend app directory
app-dir := FEELS_APP_PATH

# ===== Main Frontend Commands =====

# Install Node.js dependencies
install:
    @just show-progress "Installing Next.js dependencies"
    @cd {{app-dir}} && npm install

# Start development server
dev:
    @just show-progress "Starting Next.js development server"
    @cd {{app-dir}} && npm run dev:watch

# Build for production
build:
    @just show-progress "Building Next.js app for production"
    @cd {{app-dir}} && npm run build
    @just show-success "Production build complete"

# Run linter
lint:
    @just show-progress "Linting Next.js application"
    @cd {{app-dir}} && npm run lint

# Run tests
test:
    @just show-progress "Running Next.js app tests"
    @cd {{app-dir}} && npm test

# Clean build artifacts
clean:
    @just show-progress "Cleaning Next.js build artifacts"
    @cd {{app-dir}} && rm -rf .next out node_modules/.cache
    @just show-success "Build artifacts cleaned"

# Generate TypeScript SDK from IDL
generate-sdk:
    @just show-progress "Preparing TypeScript SDK for frontend"
    @just _ensure-idl-generated
    @just _copy-idl-to-app
    @just _fix-app-idl
    @just show-success "SDK ready for frontend use"

# ===== Private Helper Functions =====

# Ensure IDL is generated (run canonical command if needed)
[private]
_ensure-idl-generated:
    #!/usr/bin/env bash
    if [[ ! -f "{{IDL_PATH}}" ]]; then
        just show-progress "IDL not found, generating via canonical command"
        just build idl
    else
        echo "  [OK] IDL already exists, using existing version"
    fi

# Copy generated IDL to frontend app directory
[private]
_copy-idl-to-app:
    #!/usr/bin/env bash
    if [[ -f "{{IDL_PATH}}" ]]; then
        mkdir -p {{app-dir}}/src/idl
        cp {{IDL_PATH}} {{app-dir}}/src/idl/feels.json
        echo "  IDL copied to app"
    else
        just show-error "IDL not found. Run 'just build idl' first"
        exit {{EXIT_CONFIG_ERROR}}
    fi

# Fix IDL for browser compatibility
[private]
_fix-app-idl:
    #!/usr/bin/env bash
    if [[ -f "{{app-dir}}/src/lib/fix-idl.js" ]]; then
        node {{app-dir}}/src/lib/fix-idl.js \
            {{app-dir}}/src/idl/feels.json \
            {{app-dir}}/src/idl/feels.json
        echo "  IDL fixed for app usage"
    else
        just show-warning "fix-idl.js not found, skipping IDL fix"
    fi