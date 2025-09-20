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
        echo "Frontend development environment loaded"
        echo "Node.js version: $(node --version)"
        echo "npm version: $(npm --version)"
        echo ""
        echo "Available commands:"
        echo "  app-setup        - Set up Next.js application"
        echo ""
        echo "In feels-app directory:"
        echo "  npm install      - Install dependencies"
        echo "  npm run dev      - Start development server"
        echo "  npm run build    - Build for production"
        echo "  npm run lint     - Run ESLint"
        echo "  npm run format   - Format code with Prettier"
        echo ""
        
        # Navigate to feels-app directory if it exists
        if [ -d "feels-app" ]; then
          echo "Feels app directory found."
          echo "Run 'cd feels-app && npm run dev' to start development server."
        else
          echo "No feels-app directory found."
          echo "Run 'app-setup' to create a new Next.js application."
        fi
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
