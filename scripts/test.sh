#!/usr/bin/env bash
set -euo pipefail

mkdir -p target/tmp
export TMPDIR="$PWD/target/tmp"

echo "==> Clippy (workspace)"
cargo clippy --workspace --all-targets --all-features --exclude caliber-pg

echo "==> Cargo tests (workspace, excluding pgrx)"
cargo test --workspace --all-targets --exclude caliber-pg

psql_as() {
  local user="$1"
  local password="$2"
  shift 2
  PGPASSWORD="${password}" psql -v ON_ERROR_STOP=1 -h "${CALIBER_DB_HOST}" -p "${CALIBER_DB_PORT}" -U "${user}" -d "${CALIBER_DB_NAME}" "$@"
}
is_local_db() {
  case "${CALIBER_DB_HOST:-}" in
    ""|localhost|127.0.0.1|::1)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}
sudo_psql_local() {
  sudo -u postgres psql -v ON_ERROR_STOP=1 -d "${CALIBER_DB_NAME}" "$@"
}

if [[ "${DB_TESTS:-}" == "1" ]]; then
  echo "==> DB-backed API tests (requires CALIBER_DB_* env)"
  if [[ "${CALIBER_PG_INSTALL:-}" == "1" ]]; then
    CARGO_BIN="$(command -v cargo || true)"
    if [[ -z "${CARGO_BIN}" ]]; then
      CARGO_BIN="$(rustup which cargo 2>/dev/null || true)"
    fi
    if [[ -z "${CARGO_BIN}" ]]; then
      echo "cargo not found. Run 'rustup default stable' or ensure cargo is on PATH."
      exit 1
    fi
    echo "==> Installing caliber_pg extension (requires sudo)"
    if [[ -z "${RUSTUP_TOOLCHAIN:-}" ]]; then
      RUSTUP_TOOLCHAIN="stable"
    fi
    sudo -E env "PATH=$PATH" "RUSTUP_TOOLCHAIN=${RUSTUP_TOOLCHAIN}" "${CARGO_BIN}" pgrx install --package caliber-pg --pg-config "/usr/lib/postgresql/18/bin/pg_config"
    if is_local_db; then
      if [[ -f "caliber-pg/sql/caliber_pg--0.4.4.sql" ]]; then
        sudo cp "caliber-pg/sql/caliber_pg--0.4.4.sql" "/usr/share/postgresql/18/extension/caliber_pg--0.4.4.sql"
      fi
    fi
  fi

  BOOTSTRAP_USER="${CALIBER_DB_BOOTSTRAP_USER:-${CALIBER_DB_USER}}"
  BOOTSTRAP_PASSWORD="${CALIBER_DB_BOOTSTRAP_PASSWORD:-${CALIBER_DB_PASSWORD:-}}"

  psql_as "${CALIBER_DB_USER}" "${CALIBER_DB_PASSWORD:-}" -c "select 1;" >/dev/null
  if ! psql_as "${CALIBER_DB_USER}" "${CALIBER_DB_PASSWORD:-}" -tAc "select 1 from pg_available_extensions where name = 'vector';" | rg -q "1"; then
    echo "pgvector extension not available for this Postgres install."
    echo "Install pgvector or point CALIBER_DB_* at a Postgres with pgvector."
    exit 1
  fi
  if ! psql_as "${BOOTSTRAP_USER}" "${BOOTSTRAP_PASSWORD}" -c "create extension if not exists vector;" >/dev/null; then
    if is_local_db && sudo_psql_local -c "create extension if not exists vector;" >/dev/null; then
      echo "Enabled pgvector via sudo for local Postgres."
    else
      echo "Failed to enable pgvector extension. Run as a superuser:"
      echo "  sudo -u postgres psql -d ${CALIBER_DB_NAME} -c \"CREATE EXTENSION IF NOT EXISTS vector;\""
      exit 1
    fi
  fi
  schema_ready="$(psql_as "${CALIBER_DB_USER}" "${CALIBER_DB_PASSWORD:-}" -tAc "select to_regclass('public.caliber_agent') is not null;")"
  if ! rg -q "t" <<<"${schema_ready}"; then
    if psql_as "${CALIBER_DB_USER}" "${CALIBER_DB_PASSWORD:-}" -tAc "select 1 from pg_available_extensions where name = 'caliber_pg';" | rg -q "1"; then
      if ! psql_as "${BOOTSTRAP_USER}" "${BOOTSTRAP_PASSWORD}" -c "create extension if not exists caliber_pg; select caliber_init();" >/dev/null; then
        if is_local_db && sudo_psql_local -c "create extension if not exists caliber_pg; select caliber_init();" >/dev/null; then
          echo "Initialized caliber_pg via sudo for local Postgres."
        elif is_local_db && sudo_psql_local -f "caliber-pg/sql/caliber_pg--0.4.4.sql" >/dev/null; then
          echo "Initialized caliber_pg schema via SQL bootstrap for local Postgres."
        else
          echo "Failed to initialize via extension. Run as a superuser:"
          echo "  sudo -u postgres psql -d ${CALIBER_DB_NAME} -c \"CREATE EXTENSION IF NOT EXISTS caliber_pg; SELECT caliber_init();\""
          exit 1
        fi
      fi
    else
      if ! psql_as "${BOOTSTRAP_USER}" "${BOOTSTRAP_PASSWORD}" -f "caliber-pg/sql/caliber_init.sql" >/dev/null; then
        if is_local_db && sudo_psql_local -f "caliber-pg/sql/caliber_init.sql" >/dev/null; then
          echo "Bootstrapped schema via sudo for local Postgres."
        else
          echo "Schema bootstrap failed. Grant schema ownership or run as a superuser:"
          echo "  sudo -u postgres psql -d ${CALIBER_DB_NAME} -f caliber-pg/sql/caliber_init.sql"
          echo "  sudo -u postgres psql -d ${CALIBER_DB_NAME} -c \"ALTER DATABASE ${CALIBER_DB_NAME} OWNER TO ${CALIBER_DB_USER};\""
          echo "  sudo -u postgres psql -d ${CALIBER_DB_NAME} -c \"ALTER SCHEMA public OWNER TO ${CALIBER_DB_USER};\""
          echo "  sudo -u postgres psql -d ${CALIBER_DB_NAME} -c \"GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO ${CALIBER_DB_USER};\""
          echo "  sudo -u postgres psql -d ${CALIBER_DB_NAME} -c \"GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO ${CALIBER_DB_USER};\""
          echo "  sudo -u postgres psql -d ${CALIBER_DB_NAME} -c \"GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public TO ${CALIBER_DB_USER};\""
          exit 1
        fi
      fi
    fi
    if ! psql_as "${CALIBER_DB_USER}" "${CALIBER_DB_PASSWORD:-}" -tAc "select to_regclass('public.caliber_agent') is not null;" | rg -q "t"; then
      echo "Schema missing (public.caliber_agent not found). Run bootstrap as a superuser."
      exit 1
    fi
  fi
  if ! psql_as "${CALIBER_DB_USER}" "${CALIBER_DB_PASSWORD:-}" -tAc "select 1 from pg_proc p join pg_namespace n on n.oid = p.pronamespace where n.nspname = 'public' and p.proname = 'caliber_agent_register';" | rg -q "1"; then
    if is_local_db; then
      if sudo_psql_local -c "create extension if not exists caliber_pg; select caliber_init();" >/dev/null; then
        echo "Initialized caliber_pg via sudo for local Postgres."
      elif sudo_psql_local -f "caliber-pg/sql/caliber_pg--0.4.4.sql" >/dev/null; then
        echo "Initialized caliber_pg schema via SQL bootstrap for local Postgres."
      fi
    fi
  fi
  if ! psql_as "${CALIBER_DB_USER}" "${CALIBER_DB_PASSWORD:-}" -tAc "select 1 from pg_proc p join pg_namespace n on n.oid = p.pronamespace where n.nspname = 'public' and p.proname = 'caliber_agent_register';" | rg -q "1"; then
    echo "Missing caliber_pg SQL functions (caliber_agent_register not found)."
    echo "Install the extension into this Postgres, then run:"
    echo "  sudo -u postgres psql -d ${CALIBER_DB_NAME} -c \"CREATE EXTENSION IF NOT EXISTS caliber_pg; SELECT caliber_init();\""
    echo "If the extension is not available, build/install it with pgrx:"
    echo "  cargo pgrx install --package caliber-pg --pg-config \"/usr/lib/postgresql/18/bin/pg_config\""
    exit 1
  fi
  cargo test -p caliber-api --tests --all-features
fi

echo "==> PGRX tests (pg18)"
cargo pgrx test pg18 --package caliber-pg
