#!/bin/bash
# CALIBER PostgreSQL Initialization Script
# Creates the caliber-pg extension and initializes schema

set -e

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    -- Create the caliber-pg extension
    CREATE EXTENSION IF NOT EXISTS caliber_pg;

    -- Verify extension is loaded
    SELECT extname, extversion FROM pg_extension WHERE extname = 'caliber_pg';

    -- Log success
    DO \$\$
    BEGIN
        RAISE NOTICE 'caliber-pg extension initialized successfully';
    END
    \$\$;
EOSQL

echo "CALIBER database initialized"
