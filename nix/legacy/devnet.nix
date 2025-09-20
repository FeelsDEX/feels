# Local Solana test validator environment for development and testing
# Automatically sets up devnet, generates keypairs, and deploys programs
{ pkgs, inputs', projectConfig }:

let
  devnet = pkgs.writeShellScriptBin "devnet" ''
    set -euo pipefail
    
    echo "Starting ${projectConfig.projectName} local development environment..."
    
    # Configuration
    VALIDATOR_LOG_DIR="${projectConfig.directories.logs}"
    LEDGER_DIR="${projectConfig.directories.ledger}"
    KEYPAIR_DIR="${projectConfig.directories.keypairs}"
    
    # Create directories
    mkdir -p "$VALIDATOR_LOG_DIR" "$LEDGER_DIR" "$KEYPAIR_DIR"
    
    # Function to cleanup on exit
    cleanup() {
      echo "Cleaning up..."
      pkill -f solana-test-validator || true
      echo "Cleanup complete"
    }
    trap cleanup EXIT INT TERM
    
    # Generate or use existing keypairs
    if [ ! -f "$KEYPAIR_DIR/payer.json" ]; then
      echo "Generating development keypairs..."
      ${inputs'.zero-nix.packages.solana-tools}/bin/solana-keygen new --no-bip39-passphrase --silent --outfile "$KEYPAIR_DIR/payer.json"
      ${inputs'.zero-nix.packages.solana-tools}/bin/solana-keygen new --no-bip39-passphrase --silent --outfile "$KEYPAIR_DIR/program.json"
    fi
    
    # Build the program
    echo "Building ${projectConfig.projectName}..."
    ${inputs'.zero-nix.packages.solana-tools}/bin/anchor build
    
    # Start the validator
    echo "Starting local validator..."
    ${inputs'.zero-nix.packages.solana-node}/bin/solana-test-validator \
      --ledger "$LEDGER_DIR" \
      --keypair "$KEYPAIR_DIR/payer.json" \
      --bind-address ${projectConfig.validator.bindAddress} \
      --rpc-port ${toString projectConfig.validator.rpcPort} \
      --rpc-bind-address ${projectConfig.validator.bindAddress} \
      --dynamic-port-range ${projectConfig.validator.portRange} \
      --enable-rpc-transaction-history \
      --enable-extended-tx-metadata-storage \
      --log "$VALIDATOR_LOG_DIR/validator.log" \
      --reset \
      --quiet &
    
    VALIDATOR_PID=$!
    
    # Wait for validator to start
    echo "Waiting for validator to start..."
    timeout=30
    while ! ${inputs'.zero-nix.packages.solana-tools}/bin/solana cluster-version --url http://localhost:${toString projectConfig.validator.rpcPort} >/dev/null 2>&1; do
      sleep 1
      timeout=$((timeout - 1))
      if [ $timeout -le 0 ]; then
        echo "Validator failed to start within 30 seconds"
        exit 1
      fi
    done
    
    echo "Validator is running!"
    
    # Configure Solana CLI
    ${inputs'.zero-nix.packages.solana-tools}/bin/solana config set --url http://localhost:${toString projectConfig.validator.rpcPort}
    ${inputs'.zero-nix.packages.solana-tools}/bin/solana config set --keypair "$KEYPAIR_DIR/payer.json"
    
    # Fund the payer account
    echo "Funding payer account..."
    ${inputs'.zero-nix.packages.solana-tools}/bin/solana airdrop ${toString projectConfig.validator.airdropAmount} "$KEYPAIR_DIR/payer.json" --url http://localhost:${toString projectConfig.validator.rpcPort}
    
    # Deploy the program
    echo "Deploying ${projectConfig.projectName}..."
    if ${inputs'.zero-nix.packages.solana-tools}/bin/anchor deploy --provider.cluster localnet; then
      echo "Program deployed successfully!"
    else
      echo "Program deployment failed, but validator is still running"
    fi
    
    # Show useful information
    echo
    echo "${projectConfig.projectName} Local Development Environment"
    echo "=============================================="
    echo "RPC URL: http://localhost:${toString projectConfig.validator.rpcPort}"
    echo "WebSocket URL: ws://localhost:${toString projectConfig.validator.wsPort}"
    echo "Payer keypair: $KEYPAIR_DIR/payer.json"
    echo "Program keypair: $KEYPAIR_DIR/program.json"
    echo "Logs: $VALIDATOR_LOG_DIR/validator.log"
    echo
    echo "Useful commands:"
    echo "  solana balance                 - Check balance"
    echo "  solana logs                    - Stream program logs"
    echo "  anchor test --skip-local-validator - Run tests against local validator"
    echo
    echo "Press Ctrl+C to stop the validator"
    
    # Keep the script running
    wait $VALIDATOR_PID
  '';
in

{
  inherit devnet;
}