#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${BASH_VERSION:-}" ]]; then
  echo "This script must be run with bash. Try: bash $0"
  exit 2
fi

mkdir -p target/tmp
export TMPDIR="$PWD/target/tmp"
if [[ -z "${CARGO_TARGET_DIR:-}" ]]; then
  export CARGO_TARGET_DIR="$PWD/.target"
fi

echo "==> Clippy (workspace)"
cargo clippy --workspace --all-targets --all-features --exclude caliber-pg

echo "==> Cargo tests (workspace, excluding pgrx)"
cargo test --workspace --all-targets --exclude caliber-pg

if [[ "${FUZZ:-}" == "1" ]]; then
  echo "==> Fuzz (stable)"
  FUZZ_RUNS="${FUZZ_RUNS:-10000}"
  # Guard against typos like FUZZ_RUNS=10mm.
  if ! [[ "${FUZZ_RUNS}" =~ ^[0-9]+$ ]]; then
    echo "FUZZ_RUNS must be an integer (got: ${FUZZ_RUNS})."
    exit 2
  fi
  echo "Fuzz runs: ${FUZZ_RUNS}"
  if command -v bun >/dev/null 2>&1; then
    FUZZ_RUNS="${FUZZ_RUNS}" bun test tests/fuzz/ || true
  else
    echo "bun not found; skipping JS fuzz tests."
  fi
  # Keep Rust prop tests meaningful but bounded by default.
  if [[ -z "${PROPTEST_CASES:-}" ]]; then
    if (( FUZZ_RUNS < 2000 )); then
      PROPTEST_CASES=256
    elif (( FUZZ_RUNS < 20000 )); then
      PROPTEST_CASES=512
    else
      PROPTEST_CASES=1024
    fi
  fi
  PROPTEST_CASES="${PROPTEST_CASES}" \
    PROPTEST_MAX_SHRINK_ITERS="${PROPTEST_MAX_SHRINK_ITERS:-64}" \
    cargo test -p caliber-dsl -p caliber-core || true
fi

if [[ "${BENCH:-}" == "1" ]]; then
  BENCH_SAMPLE_SIZE="${BENCH_SAMPLE_SIZE:-10}"
  BENCH_MEAS_TIME="${BENCH_MEAS_TIME:-2}"
  BENCH_ARGS=(-- --sample-size "${BENCH_SAMPLE_SIZE}" --measurement-time "${BENCH_MEAS_TIME}")

  echo "==> Bench (SDK)"
  if command -v bun >/dev/null 2>&1 && [[ -f caliber-sdk/bench/index.ts ]]; then
    (cd caliber-sdk && bun run bench) || true
  else
    echo "SDK bench not available (bun or bench script missing)."
  fi

  if [[ -f caliber-dsl/benches/dsl_hotpath.rs ]]; then
    echo "==> Bench (Rust DSL hotpath)"
    cargo bench -p caliber-dsl --bench dsl_hotpath "${BENCH_ARGS[@]}" || true
  else
    echo "DSL hotpath bench missing; skipping."
  fi

  if [[ -f caliber-core/benches/context_hotpath.rs ]]; then
    echo "==> Bench (Rust context hotpath)"
    cargo bench -p caliber-core --bench context_hotpath "${BENCH_ARGS[@]}" || true
  else
    echo "Context hotpath bench missing; skipping."
  fi

  if [[ "${DB_TESTS:-}" == "1" ]]; then
    if command -v pgbench >/dev/null 2>&1 && [[ -f scripts/bench/caliber_pgbench.sql ]]; then
      echo "==> Bench (Postgres pgbench, short)"
      PGPASSWORD="${CALIBER_DB_PASSWORD:-}" pgbench \
        -h "${CALIBER_DB_HOST}" -p "${CALIBER_DB_PORT}" \
        -U "${CALIBER_DB_USER}" -d "${CALIBER_DB_NAME}" \
        -T 5 -c 2 -j 1 -f scripts/bench/caliber_pgbench.sql || true
    else
      echo "pgbench or scripts/bench/caliber_pgbench.sql missing; skipping Postgres bench."
    fi
  fi

  if [[ -x scripts/bench/api_smoke.sh ]]; then
    echo "==> Bench (API smoke)"
    scripts/bench/api_smoke.sh || true
  fi
fi

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
  reset_schema_local() {
    sudo_psql_local -c "DROP SCHEMA public CASCADE;"
    sudo_psql_local -c "CREATE SCHEMA public;"
    sudo_psql_local -c "GRANT ALL ON SCHEMA public TO ${CALIBER_DB_USER};"
    sudo_psql_local -c "CREATE EXTENSION IF NOT EXISTS vector;" >/dev/null
  }

if [[ "${DB_TESTS:-}" == "1" ]]; then
  echo "==> DB-backed API tests (requires CALIBER_DB_* env)"
  if [[ -x "scripts/caliber_doctor.py" ]]; then
    echo "==> Caliber Doctor (pre-flight)"
    python3 scripts/caliber_doctor.py || true
  fi
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
    # cargo-pgrx sometimes fails to copy the versioned SQL install script.
    # Copy it explicitly to keep CREATE EXTENSION deterministic.
    PG_EXT_VERSION="$(awk -F'\"' '
      /^\[workspace\.package\]/ { in_wp = 1; next }
      /^\[/ { in_wp = 0 }
      in_wp && /^version = / { print $2; exit }
    ' Cargo.toml)"
    PG_SQL_SRC="caliber-pg/sql/caliber_pg--${PG_EXT_VERSION}.sql"
    PG_SQL_GEN="${TMPDIR}/caliber_pg--${PG_EXT_VERSION}.sql"
    PG_SQL_DST="/usr/share/postgresql/18/extension/caliber_pg--${PG_EXT_VERSION}.sql"
    if [[ -n "${PG_EXT_VERSION}" ]]; then
      echo "==> Generating extension SQL (schema)"
      SCHEMA_TARGET_DIR="${PWD}/target/schema-target"
      mkdir -p "${SCHEMA_TARGET_DIR}"
      # cargo-pgrx's --out flag appears unreliable in this environment; capture stdout instead.
      TMPDIR="${TMPDIR}" CARGO_TARGET_DIR="${SCHEMA_TARGET_DIR}" \
        "${CARGO_BIN}" pgrx schema --package caliber-pg --pg-config "/usr/lib/postgresql/18/bin/pg_config" \
        --features pg18 --no-default-features > "${PG_SQL_GEN}" || true
      if [[ -f "${PG_SQL_GEN}" ]] && rg -q "CREATE TABLE IF NOT EXISTS caliber_dsl_config" "${PG_SQL_GEN}"; then
        sudo cp -f "${PG_SQL_GEN}" "${PG_SQL_DST}"
        echo "Ensured extension SQL script: ${PG_SQL_DST}"
      elif [[ -f "${PG_SQL_SRC}" ]] && rg -q "CREATE TABLE IF NOT EXISTS caliber_dsl_config" "${PG_SQL_SRC}"; then
        echo "Schema generation did not yield expected DSL tables; using repo SQL."
        sudo cp -f "${PG_SQL_SRC}" "${PG_SQL_DST}"
        echo "Ensured extension SQL script: ${PG_SQL_DST}"
      else
        echo "Warning: no valid extension SQL script found for ${PG_EXT_VERSION}."
        echo "Look for a generated schema and copy it manually:"
        echo "  sudo find / -name 'caliber_pg--${PG_EXT_VERSION}.sql' 2>/dev/null | head"
      fi
    else
      echo "Warning: expected extension SQL script not found: ${PG_SQL_SRC}"
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
      if ! psql_as "${BOOTSTRAP_USER}" "${BOOTSTRAP_PASSWORD}" -c "create extension if not exists caliber_pg;" >/dev/null; then
        if is_local_db && sudo_psql_local -c "create extension if not exists caliber_pg;" >/dev/null; then
          echo "Initialized caliber_pg via sudo for local Postgres."
        elif is_local_db && [[ "${CALIBER_DB_RESET:-}" == "1" ]]; then
          echo "Resetting local schema to allow extension install (CALIBER_DB_RESET=1)."
          sudo_psql_local -c "drop extension if exists caliber_pg cascade;" >/dev/null
          reset_schema_local
          sudo_psql_local -c "create extension if not exists caliber_pg;" >/dev/null
          echo "Initialized caliber_pg after schema reset."
        else
          echo "Failed to initialize via extension. Run as a superuser:"
          echo "  sudo -u postgres psql -d ${CALIBER_DB_NAME} -c \"CREATE EXTENSION IF NOT EXISTS caliber_pg;\""
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
      if sudo_psql_local -c "create extension if not exists caliber_pg;" >/dev/null; then
        echo "Initialized caliber_pg via sudo for local Postgres."
      elif [[ "${CALIBER_DB_RESET:-}" == "1" ]]; then
        echo "Resetting local schema to allow extension install (CALIBER_DB_RESET=1)."
        sudo_psql_local -c "drop extension if exists caliber_pg cascade;" >/dev/null
        reset_schema_local
        sudo_psql_local -c "create extension if not exists caliber_pg;" >/dev/null
        echo "Initialized caliber_pg after schema reset."
      fi
    fi
  fi
  if ! psql_as "${CALIBER_DB_USER}" "${CALIBER_DB_PASSWORD:-}" -tAc "select 1 from pg_proc p join pg_namespace n on n.oid = p.pronamespace where n.nspname = 'public' and p.proname = 'caliber_agent_register';" | rg -q "1"; then
    echo "Missing caliber_pg SQL functions (caliber_agent_register not found)."
    echo "Install the extension into this Postgres, then run:"
    echo "  sudo -u postgres psql -d ${CALIBER_DB_NAME} -c \"CREATE EXTENSION IF NOT EXISTS caliber_pg;\""
    echo "If the extension is not available, build/install it with pgrx:"
    echo "  cargo pgrx install --package caliber-pg --pg-config \"/usr/lib/postgresql/18/bin/pg_config\""
    exit 1
  fi
  cargo test -p caliber-api --tests --all-features
fi

if [[ "${DB_TESTS:-}" == "1" || "${PGRX_TESTS:-}" == "1" ]]; then
  echo "==> PGRX tests (pg18)"
  cargo pgrx test pg18 --package caliber-pg
else
  echo "==> Skipping PGRX tests (set DB_TESTS=1 or PGRX_TESTS=1 to run)"
fi
