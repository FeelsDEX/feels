# Next.js and frontend development environment
{ pkgs, inputs', lib, ... }:

let
  # Node.js version to use
  nodejs = pkgs.nodejs_20;
  
in {
  packages = with pkgs; [
    # Node.js and package management
    nodejs
    # Using npm instead of pnpm for better Nix compatibility
    
    # Development tools
    nodePackages.typescript
    nodePackages.typescript-language-server
    nodePackages."@tailwindcss/language-server"
    nodePackages.eslint
    nodePackages.prettier
# nodePackages.next  # Not available in this nixpkgs version
    
    # Git hooks and formatting (commented out - not available in this nixpkgs version)
    # nodePackages.husky
    # nodePackages.lint-staged
    
    # Additional utilities
    jq
    curl
  ];
  
  commands = [
    {
      name = "app-setup";
      command = ''
        if [ ! -d "feels-app" ]; then
          echo "Creating Next.js app..."
          mkdir -p feels-app && cd feels-app
          npx create-next-app@latest . --typescript --tailwind --eslint --app --src-dir --import-alias '@/*' --yes
        else
          echo "App directory exists. Installing dependencies..."
          cd feels-app && npm install
        fi
      '';
      help = "Set up Next.js application";
    }
  ];
  
  env = [
    {
      name = "NODE_ENV";
      value = "development";
    }
    {
      name = "NEXT_TELEMETRY_DISABLED";
      value = "1";
    }
    {
      name = "NODE_OPTIONS";
      value = "--max-old-space-size=4096";
    }
    {
      name = "TS_NODE_COMPILER_OPTIONS";
      value = ''{"module":"commonjs"}'';
    }
  ];
  
  startup = {
    frontend = {
      deps = [];
      text = ''
        echo "Frontend Tools"
        echo "=============="
        echo ""
        echo "Runtime & Package Manager:"
        echo "  node $(node --version)    - JavaScript runtime"
        echo "  npm $(npm --version)      - Package manager"
        echo ""
        echo "Development Commands (via just):"
        echo "  just frontend dev       - Start Next.js dev server"
        echo "  just frontend build     - Build for production"
        echo "  just frontend lint      - Run ESLint"
        echo "  just frontend format    - Format with Prettier"
        echo ""
        echo "Nix Commands:"
        echo "  app-setup               - Initialize Next.js app"
        echo ""
      '';
    };
  };
  
  # Build derivation for the Next.js app
  buildApp = { src, name ? "feels-nextjs-app" }: pkgs.stdenv.mkDerivation {
    inherit name src;
    
    buildInputs = [ nodejs ];
    
    buildPhase = ''
      export HOME=$TMPDIR
      export npm_config_cache=$TMPDIR/.npm
      
      # Install dependencies
      npm ci
      
      # Build the application
      npm run build
    '';
    
    installPhase = ''
      mkdir -p $out
      cp -r .next $out/
      cp -r public $out/
      cp package.json $out/
      cp next.config.* $out/ 2>/dev/null || true
    '';
  };
}
