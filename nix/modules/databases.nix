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
      ROCKS_PATH="$PROJECT_ROOT/data/rocksdb"
      
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
      ROCKS_PATH="$PROJECT_ROOT/data/rocksdb"
      
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
    {
      name = "pg-start";
      command = ''
        mkdir -p $PWD/test-data/postgres
        if ! pg_isready -h localhost -p 5432 > /dev/null 2>&1; then
          echo "Starting PostgreSQL..."
          initdb -D $PWD/test-data/postgres > /dev/null 2>&1 || true
          pg_ctl -D $PWD/test-data/postgres -l $PWD/test-data/postgres.log start
        else
          echo "PostgreSQL already running"
        fi
      '';
      help = "Start PostgreSQL server for testing";
    }
    {
      name = "pg-stop";
      command = ''
        pg_ctl -D $PWD/test-data/postgres stop 2>/dev/null || true
      '';
      help = "Stop PostgreSQL server";
    }
    {
      name = "redis-start";
      command = ''
        mkdir -p $PWD/test-data/redis
        if ! redis-cli ping > /dev/null 2>&1; then
          echo "Starting Redis..."
          redis-server --dir $PWD/test-data/redis --daemonize yes --logfile $PWD/test-data/redis.log
        else
          echo "Redis already running"
        fi
      '';
      help = "Start Redis server for testing";
    }
    {
      name = "redis-stop";
      command = ''
        redis-cli shutdown 2>/dev/null || true
      '';
      help = "Stop Redis server";
    }
    {
      name = "services-start";
      command = ''
        pg-start && redis-start
      '';
      help = "Start all database services";
    }
    {
      name = "services-stop";
      command = ''
        pg-stop && redis-stop
      '';
      help = "Stop all database services";
    }
  ];
  
  env = [
    {
      name = "ROCKSDB_DATA_PATH";
      value = "$PWD/data/rocksdb";
    }
    {
      name = "PGDATA";
      value = "$PWD/test-data/postgres";
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
        echo "Database development environment loaded"
        echo "Available databases:"
        echo "  ✓ PostgreSQL 15"
        echo "  ✓ Redis"
        echo "  ✓ RocksDB with all compression libs"
        echo ""
        echo "Available commands:"
        echo "  services-start/services-stop - Control all services"
        echo "  pg-start/pg-stop            - Control PostgreSQL"
        echo "  redis-start/redis-stop      - Control Redis"
        echo "  init-rocksdb/clean-rocksdb  - Manage RocksDB data"
        echo ""
        echo "Environment variables set:"
        echo "  ROCKSDB_DATA_PATH=$ROCKSDB_DATA_PATH"
        echo "  DATABASE_URL=$DATABASE_URL"
        echo "  REDIS_URL=$REDIS_URL"
        echo ""
        
        # Check if data directory exists
        if [ ! -d "$ROCKSDB_DATA_PATH" ]; then
          echo "Run 'init-rocksdb' to create the RocksDB data directory"
        else
          echo "RocksDB data directory exists at: $ROCKSDB_DATA_PATH"
        fi
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
