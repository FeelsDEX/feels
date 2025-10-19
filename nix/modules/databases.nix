# Database stack: PostgreSQL, Redis, RocksDB with compression libraries
{ pkgs, inputs', lib, ... }:

let
  inherit (pkgs) stdenv;
  
  # RocksDB compression libraries
  rocksdbDeps = with pkgs; [
    pkg-config
    cmake
    zlib
    bzip2
    lz4
    zstd
    snappy
    rocksdb
  ] ++ pkgs.lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.Security
    darwin.apple_sdk.frameworks.SystemConfiguration
  ];

  # Script to initialize RocksDB data directory
  initRocksDBScript = pkgs.writeShellApplication {
    name = "init-rocksdb";
    runtimeInputs = [ pkgs.coreutils ];
    text = ''
      set -e
      
      PROJECT_ROOT="$(pwd)"
      ROCKS_PATH="$PROJECT_ROOT/localnet/indexer-storage/rocksdb"
      
      echo "=== RocksDB Initialization ==="
      echo "Creating RocksDB data directory at: $ROCKS_PATH"
      
      mkdir -p "$ROCKS_PATH"
      chmod 755 "$ROCKS_PATH"
      
      echo "✓ RocksDB data directory initialized"
      echo "  Path: $ROCKS_PATH"
      
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
      set -e
      
      PROJECT_ROOT="$(pwd)"
      ROCKS_PATH="$PROJECT_ROOT/localnet/indexer-storage/rocksdb"
      
      echo "=== RocksDB Cleanup ==="
      
      if [ -d "$ROCKS_PATH" ]; then
        echo "Removing RocksDB data at: $ROCKS_PATH"
        rm -rf "$ROCKS_PATH"
        echo "✓ RocksDB data cleaned"
      else
        echo "No RocksDB data directory found at: $ROCKS_PATH"
      fi
      
      if [ -f "$PROJECT_ROOT/.rocksdb_env" ]; then
        rm "$PROJECT_ROOT/.rocksdb_env"
        echo "✓ Environment file removed"
      fi
    '';
  };

in {
  packages = rocksdbDeps ++ [
    # Database servers
    pkgs.postgresql_15
    pkgs.redis
    
    # Database management tools
    pkgs.pgcli
    pkgs.sqlx-cli
    
    # RocksDB utilities
    initRocksDBScript
    cleanRocksDBScript
    
    # Additional testing tools
    pkgs.procps
    pkgs.netcat
  ];
  
  commands = [];
  
  env = [
    {
      name = "ROCKSDB_DATA_PATH";
      value = "$PWD/localnet/indexer-storage/rocksdb";
    }
    {
      name = "PGDATA";
      value = "$PWD/localnet/data/postgres";
    }
    {
      name = "DATABASE_URL";
      value = "postgresql://localhost/feels_indexer_test";
    }
    {
      name = "REDIS_URL";
      value = "redis://localhost:6379/1";
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
  
  startup = {
    databases = {
      deps = [];
      text = ''
        echo "Database Tools"
        echo "=============="
        echo ""
        echo "Servers:"
        echo "  postgresql (v15) - SQL database"
        echo "  redis           - In-memory cache"
        echo ""
        echo "CLI Tools:"
        echo "  psql            - PostgreSQL interactive terminal"
        echo "  pgcli           - PostgreSQL CLI with autocomplete"
        echo "  redis-cli       - Redis command line interface"
        echo "  sqlx            - Rust SQL toolkit (migrations)"
        echo ""
        echo "Management (via just):"
        echo "  just services pg-start     - Start PostgreSQL"
        echo "  just services redis-start  - Start Redis"
        echo "  just services services-start - Start all services"
        echo ""
        echo "Environment:"
        echo "  DATABASE_URL=$DATABASE_URL"
        echo "  REDIS_URL=$REDIS_URL"
        echo "  ROCKSDB_DATA_PATH=$ROCKSDB_DATA_PATH"
        echo ""
      '';
    };
  };
  
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
}
