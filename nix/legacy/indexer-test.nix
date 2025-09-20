# Combined development environment for indexer testing
# Includes: Solana, Geyser support, PostgreSQL, Redis, RocksDB
{
  pkgs,
  inputs',
  projectConfig,
  ...
}: let
  indexerConfig = import ./indexer.nix { inherit pkgs inputs' projectConfig; };
  devshellConfig = import ./devshell.nix { inherit pkgs inputs' projectConfig; };
  geyserDevnetConfig = import ./geyser-devnet.nix { inherit pkgs inputs' projectConfig; };
in {
  packages = indexerConfig.packages ++ [
    # Database services
    pkgs.postgresql_15
    pkgs.redis
    
    # Database management tools
    pkgs.pgcli
    pkgs.sqlx-cli
    
    # Additional testing tools
    pkgs.procps  # for pkill
    pkgs.netcat  # for port checking
  ];
  
  commands = devshellConfig.commands ++ [
    {
      name = "geyser-devnet";
      command = "${geyserDevnetConfig.devnet}/bin/geyser-devnet";
      help = "Start Solana devnet with Geyser plugin";
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
      help = "Start all test services";
    }
    {
      name = "services-stop";
      command = ''
        pg-stop && redis-stop
      '';
      help = "Stop all test services";
    }
    {
      name = "test-integration";
      command = ''
        cd feels-indexer/tests && just test-integration
      '';
      help = "Run indexer integration tests";
    }
  ];
  
  env = devshellConfig.env ++ [
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
  ];
  
  devshell.startup.indexer-test-setup = {
    deps = [];
    text = ''
      echo ""
      echo "Feels Indexer Test Environment"
      echo "=============================="
      echo ""
      echo "This environment includes:"
      echo "  ✓ Solana validator and tools"
      echo "  ✓ Geyser gRPC streaming support"
      echo "  ✓ PostgreSQL 15"
      echo "  ✓ Redis"
      echo "  ✓ RocksDB with all compression libs"
      echo "  ✓ Rust development tools"
      echo ""
      echo "Quick commands:"
      echo "  services-start    - Start PostgreSQL and Redis"
      echo "  services-stop     - Stop all services"
      echo "  test-integration  - Run integration tests"
      echo ""
      echo "Manual service control:"
      echo "  pg-start/pg-stop     - Control PostgreSQL"
      echo "  redis-start/redis-stop - Control Redis"
      echo ""
      echo "Test data will be stored in: ./test-data/"
      echo ""
    '';
  };
}