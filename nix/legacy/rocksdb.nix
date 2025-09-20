# RocksDB development environment and utilities
{
  pkgs,
  inputs',
  projectConfig,
  ...
}: let
  inherit (pkgs) lib stdenv;
  
  # RocksDB dependencies for macOS
  rocksdbDeps = with pkgs; [
    # Build tools
    pkg-config
    cmake
    
    # Compression libraries
    zlib
    bzip2
    lz4
    zstd
    snappy
    
    # RocksDB itself (for tools and debugging)
    rocksdb
  ] ++ lib.optionals stdenv.isDarwin [
    # macOS frameworks
    darwin.apple_sdk.frameworks.Security
    darwin.apple_sdk.frameworks.SystemConfiguration
  ];

  # Script to initialize RocksDB data directory
  initRocksDBScript = pkgs.writeShellApplication {
    name = "init-rocksdb";
    runtimeInputs = [ pkgs.coreutils ];
    text = ''
      # Initialize RocksDB storage directory
      set -e
      
      PROJECT_ROOT="$(pwd)"
      ROCKS_PATH="$PROJECT_ROOT/data/rocksdb"
      
      echo "=== RocksDB Initialization ==="
      echo "Creating RocksDB data directory at: $ROCKS_PATH"
      
      mkdir -p "$ROCKS_PATH"
      
      # Set proper permissions
      chmod 755 "$ROCKS_PATH"
      
      echo "✓ RocksDB data directory initialized"
      echo "  Path: $ROCKS_PATH"
      echo "  Use this path in your application configuration"
      
      # Create environment file
      cat > "$PROJECT_ROOT/.rocksdb_env" << EOF
export ROCKSDB_DATA_PATH="$ROCKS_PATH"
EOF
      
      echo "Environment variables saved to .rocksdb_env"
      echo "Run 'source .rocksdb_env' to load in your shell"
    '';
  };

  # Script to clean RocksDB data
  cleanRocksDBScript = pkgs.writeShellApplication {
    name = "clean-rocksdb";
    runtimeInputs = [ pkgs.coreutils ];
    text = ''
      # Clean RocksDB data directory
      set -e
      
      PROJECT_ROOT="$(pwd)"
      ROCKS_PATH="$PROJECT_ROOT/data/rocksdb"
      
      echo "=== RocksDB Cleanup ==="
      
      if [ -d "$ROCKS_PATH" ]; then
        echo "Removing RocksDB data at: $ROCKS_PATH"
        rm -rf "$ROCKS_PATH"
        echo "✓ RocksDB data cleaned"
      else
        echo "No RocksDB data directory found at: $ROCKS_PATH"
      fi
      
      # Remove environment file
      if [ -f "$PROJECT_ROOT/.rocksdb_env" ]; then
        rm "$PROJECT_ROOT/.rocksdb_env"
        echo "✓ Environment file removed"
      fi
    '';
  };

in {
  # Packages for RocksDB development
  packages = rocksdbDeps ++ [
    initRocksDBScript
    cleanRocksDBScript
  ];

  # Commands to add to devshell
  commands = [
    {
      name = "init-rocksdb";
      package = initRocksDBScript;
      help = "Initialize RocksDB data directory";
    }
    {
      name = "clean-rocksdb";
      package = cleanRocksDBScript;
      help = "Clean RocksDB data directory";
    }
  ];

  # Environment variables for RocksDB compilation
  env = [
    {
      name = "ROCKSDB_DATA_PATH";
      value = "$PWD/data/rocksdb";
    }
    # RocksDB compression library paths
    {
      name = "ZLIB_INCLUDE_DIR";
      value = "${pkgs.zlib.dev}/include";
    }
    {
      name = "ZLIB_LIB_DIR";
      value = "${pkgs.zlib}/lib";
    }
    {
      name = "BZIP2_INCLUDE_DIR";
      value = "${pkgs.bzip2.dev}/include";
    }
    {
      name = "BZIP2_LIB_DIR";
      value = "${pkgs.bzip2}/lib";
    }
    {
      name = "LZ4_INCLUDE_DIR";
      value = "${pkgs.lz4.dev}/include";
    }
    {
      name = "LZ4_LIB_DIR";
      value = "${pkgs.lz4}/lib";
    }
    {
      name = "ZSTD_INCLUDE_DIR";
      value = "${pkgs.zstd.dev}/include";
    }
    {
      name = "ZSTD_LIB_DIR";
      value = "${pkgs.zstd}/lib";
    }
    {
      name = "SNAPPY_INCLUDE_DIR";
      value = "${pkgs.snappy}/include";
    }
    {
      name = "SNAPPY_LIB_DIR";
      value = "${pkgs.snappy}/lib";
    }
  ];

  # Crate overrides for Rust projects using RocksDB
  crateOverrides = {
    librocksdb-sys = attrs: {
      nativeBuildInputs = with pkgs; [
        pkg-config
        cmake
      ];
      buildInputs = rocksdbDeps;
      
      preBuild = ''
        # Set RocksDB compilation environment
        export ZLIB_INCLUDE_DIR="${pkgs.zlib.dev}/include"
        export ZLIB_LIB_DIR="${pkgs.zlib}/lib"
        export BZIP2_INCLUDE_DIR="${pkgs.bzip2.dev}/include"
        export BZIP2_LIB_DIR="${pkgs.bzip2}/lib"
        export LZ4_INCLUDE_DIR="${pkgs.lz4.dev}/include"
        export LZ4_LIB_DIR="${pkgs.lz4}/lib"
        export ZSTD_INCLUDE_DIR="${pkgs.zstd.dev}/include"
        export ZSTD_LIB_DIR="${pkgs.zstd}/lib"
        export SNAPPY_INCLUDE_DIR="${pkgs.snappy}/include"
        export SNAPPY_LIB_DIR="${pkgs.snappy}/lib"
        export MACOSX_DEPLOYMENT_TARGET="11.0"
        export DEVELOPER_DIR=""
      '';
    };
  };

  # Development shell configuration
  devShell = pkgs.mkShell {
    buildInputs = rocksdbDeps ++ [
      initRocksDBScript
      cleanRocksDBScript
    ];
    
    shellHook = ''
      # Set RocksDB environment variables
      export ROCKSDB_DATA_PATH="$PWD/data/rocksdb"
      export ZLIB_INCLUDE_DIR="${pkgs.zlib.dev}/include"
      export ZLIB_LIB_DIR="${pkgs.zlib}/lib"
      export BZIP2_INCLUDE_DIR="${pkgs.bzip2.dev}/include"
      export BZIP2_LIB_DIR="${pkgs.bzip2}/lib"
      export LZ4_INCLUDE_DIR="${pkgs.lz4.dev}/include"
      export LZ4_LIB_DIR="${pkgs.lz4}/lib"
      export ZSTD_INCLUDE_DIR="${pkgs.zstd.dev}/include"
      export ZSTD_LIB_DIR="${pkgs.zstd}/lib"
      export SNAPPY_INCLUDE_DIR="${pkgs.snappy}/include"
      export SNAPPY_LIB_DIR="${pkgs.snappy}/lib"
      export MACOSX_DEPLOYMENT_TARGET="11.0"
      export DEVELOPER_DIR=""
      
      echo "RocksDB development environment loaded"
      echo "Available commands:"
      echo "  init-rocksdb    - Initialize RocksDB data directory"
      echo "  clean-rocksdb   - Clean RocksDB data directory"
      echo ""
      echo "Environment variables set:"
      echo "  ROCKSDB_DATA_PATH=$ROCKSDB_DATA_PATH"
      echo "  Compression libraries configured for native builds"
      
      # Check if data directory exists
      if [ ! -d "$ROCKSDB_DATA_PATH" ]; then
        echo "Run 'init-rocksdb' to create the data directory"
      else
        echo "RocksDB data directory exists at: $ROCKSDB_DATA_PATH"
      fi
    '';
  };
}
