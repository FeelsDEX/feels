# Next.js and Tailwind development environment for Feels Protocol frontend
{ pkgs, ... }:

let
  # Node.js version to use
  nodejs = pkgs.nodejs_20;
  
  # Package manager
  pnpm = pkgs.pnpm;
  
in {
  # Development shell for Next.js application
  devShell = pkgs.mkShell {
    buildInputs = with pkgs; [
      # Node.js and package management
      nodejs
      pnpm
      yarn
      
      # Development tools
      nodePackages.typescript
      nodePackages.typescript-language-server
      nodePackages."@tailwindcss/language-server"
      nodePackages.eslint
      nodePackages.prettier
      
      # Build tools
      nodePackages.next
      
      # Git hooks and formatting
      nodePackages.husky
      nodePackages.lint-staged
      
      # Additional utilities
      jq
      curl
    ];
    
    shellHook = ''
      echo "Feels Protocol Next.js Development Environment"
      echo "Node.js version: $(node --version)"
      echo "pnpm version: $(pnpm --version)"
      echo ""
      echo "Available commands:"
      echo "  pnpm install     - Install dependencies"
      echo "  pnpm dev         - Start development server"
      echo "  pnpm build       - Build for production"
      echo "  pnpm lint        - Run ESLint"
      echo "  pnpm format      - Format code with Prettier"
      echo ""
      
      # Set up environment variables
      export NODE_ENV=development
      export NEXT_TELEMETRY_DISABLED=1
      
      # Ensure pnpm store is in project directory
      export PNPM_HOME="$PWD/.pnpm"
      export PATH="$PNPM_HOME:$PATH"
      
      # Create app directory if it doesn't exist
      if [ ! -d "app" ]; then
        echo "App directory not found. You can create it with:"
        echo "  mkdir -p app && cd app"
        echo "  pnpm create next-app@latest . --typescript --tailwind --eslint --app --src-dir --import-alias '@/*'"
      fi
    '';
    
    # Environment variables
    env = {
      # Disable Next.js telemetry
      NEXT_TELEMETRY_DISABLED = "1";
      
      # Set Node.js options
      NODE_OPTIONS = "--max-old-space-size=4096";
      
      # TypeScript configuration
      TS_NODE_COMPILER_OPTIONS = ''{"module":"commonjs"}'';
    };
  };
  
  # Build derivation for the Next.js app
  buildApp = { src, name ? "feels-nextjs-app" }: pkgs.stdenv.mkDerivation {
    inherit name src;
    
    buildInputs = [ nodejs pnpm ];
    
    buildPhase = ''
      export HOME=$TMPDIR
      export PNPM_HOME=$TMPDIR/.pnpm
      export PATH="$PNPM_HOME:$PATH"
      
      # Install dependencies
      pnpm install --frozen-lockfile
      
      # Build the application
      pnpm build
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
