# Constants used across all justfile modules
# This file now serves as a compatibility layer, importing from nix-env.just

# Import the Nix environment bridge
import 'nix-env.just'

# ===== Exit Codes =====
# These remain constant regardless of environment
export EXIT_SUCCESS := "0"
export EXIT_GENERAL_ERROR := "1"
export EXIT_MISSING_DEPS := "2"
export EXIT_BUILD_FAILURE := "3"
export EXIT_DEPLOY_FAILURE := "4"
export EXIT_TEST_FAILURE := "5"
export EXIT_CONFIG_ERROR := "10"
export EXIT_ENV_SETUP_ERROR := "11"
export EXIT_SERVICE_START_FAILURE := "12"

# ===== Colors =====
# Terminal colors for output
export GREEN := '\033[0;32m'
export YELLOW := '\033[1;33m'
export RED := '\033[0;31m'
export NC := '\033[0m' # No Color

# All paths are now defined in nix-env.just
# All network configuration is now defined in nix-env.just
# All service ports are now defined in nix-env.just
# All program IDs are now defined in nix-env.just
# All test configuration is now defined in nix-env.just