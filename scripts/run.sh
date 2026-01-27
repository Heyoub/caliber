#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${BASH_VERSION:-}" ]]; then
  echo "This script must be run with bash. Try: bash $0"
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

# -----------------------------------------------------------------------------
# Detection helpers
# -----------------------------------------------------------------------------

have_cmd() { command -v "$1" >/dev/null 2>&1; }

is_local_db() {
  case "${CALIBER_DB_HOST:-}" in
    ""|localhost|127.0.0.1|::1) return 0 ;;
    *) return 1 ;;
  esac
}

db_env_ready() {
  [[ -n "${CALIBER_DB_HOST:-}" ]] \
    && [[ -n "${CALIBER_DB_PORT:-}" ]] \
    && [[ -n "${CALIBER_DB_USER:-}" ]] \
    && [[ -n "${CALIBER_DB_NAME:-}" ]]
}

detect_db_tests_default() {
  if db_env_ready; then
    echo 1
  else
    echo 0
  fi
}

# -----------------------------------------------------------------------------
# Defaults (smart but conservative)
# -----------------------------------------------------------------------------

RUN_DB="$(detect_db_tests_default)"
RUN_FUZZ="${FUZZ:-0}"
RUN_BENCH="${BENCH:-0}"
DB_RESET="${CALIBER_DB_RESET:-0}"
PG_INSTALL="${CALIBER_PG_INSTALL:-0}"
FUZZ_RUNS="${FUZZ_RUNS:-}"

# -----------------------------------------------------------------------------
# CLI args (decision matrix trench coat)
# -----------------------------------------------------------------------------

usage() {
  cat <<'EOF'
Caliber Runner (smart wrapper around scripts/test.sh)

Usage:
  scripts/run.sh [options]

Options:
  --all           Run DB + fuzz + bench (if available)
  --db            Force DB-backed tests on
  --no-db         Force DB-backed tests off
  --reset         Set CALIBER_DB_RESET=1
  --pg-install    Set CALIBER_PG_INSTALL=1
  --fuzz          Set FUZZ=1 (stable fuzz path)
  --fuzz-runs N   Set FUZZ_RUNS=N (default: 10000 when fuzzing)
  --bench         Set BENCH=1 (short benches)
  -h, --help      Show this help

Examples:
  scripts/run.sh --db --pg-install
  scripts/run.sh --all --reset
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --all)
      RUN_DB=1
      RUN_FUZZ=1
      RUN_BENCH=1
      ;;
    --db) RUN_DB=1 ;;
    --no-db) RUN_DB=0 ;;
    --reset) DB_RESET=1 ;;
    --pg-install) PG_INSTALL=1 ;;
    --fuzz) RUN_FUZZ=1 ;;
    --fuzz-runs)
      RUN_FUZZ=1
      FUZZ_RUNS="${2:-}"
      if [[ -z "${FUZZ_RUNS}" ]]; then
        echo "--fuzz-runs requires an integer argument"
        exit 2
      fi
      shift
      ;;
    --bench) RUN_BENCH=1 ;;
    -h|--help) usage; exit 0 ;;
    *)
      echo "Unknown option: $1"
      usage
      exit 2
      ;;
  esac
  shift
done

# -----------------------------------------------------------------------------
# Capability detection (for friendly output)
# -----------------------------------------------------------------------------

HAVE_BUN=0
HAVE_PGBENCH=0
HAVE_PGRX=0

have_cmd bun && HAVE_BUN=1
have_cmd pgbench && HAVE_PGBENCH=1
if have_cmd cargo && cargo pgrx --help >/dev/null 2>&1; then
  HAVE_PGRX=1
fi

# -----------------------------------------------------------------------------
# Pretty-ish ASCII status board
# -----------------------------------------------------------------------------

echo
echo "CALIBER Runner"
echo "==============="
printf "Core tests     : %s\n" "ON"
printf "DB tests       : %s (%s)\n" \
  "$([[ "${RUN_DB}" == "1" ]] && echo ON || echo OFF)" \
  "$([[ "${RUN_DB}" == "1" ]] && db_env_ready && echo "env:ready" || echo "env:missing")"
printf "Fuzz (stable)  : %s (bun:%s)\n" \
  "$([[ "${RUN_FUZZ}" == "1" ]] && echo ON || echo OFF)" \
  "$([[ "${HAVE_BUN}" == "1" ]] && echo yes || echo no)"
if [[ "${RUN_FUZZ}" == "1" ]]; then
  effective_fuzz_runs="${FUZZ_RUNS:-10000}"
  printf "Fuzz runs      : %s\n" "${effective_fuzz_runs}"
fi
printf "Bench (short)  : %s (pgbench:%s)\n" \
  "$([[ "${RUN_BENCH}" == "1" ]] && echo ON || echo OFF)" \
  "$([[ "${HAVE_PGBENCH}" == "1" ]] && echo yes || echo no)"
printf "PG install     : %s (cargo-pgrx:%s)\n" \
  "$([[ "${PG_INSTALL}" == "1" ]] && echo ON || echo OFF)" \
  "$([[ "${HAVE_PGRX}" == "1" ]] && echo yes || echo no)"
printf "DB reset       : %s\n" "$([[ "${DB_RESET}" == "1" ]] && echo ON || echo OFF)"

echo
echo "Flow"
echo "----"
echo "[Clippy] -> [Unit/Prop Tests] -> [DB Bootstrap?] -> [DB Tests]"
if [[ "${RUN_FUZZ}" == "1" ]]; then
  echo "                     \\-> [Fuzz (stable path)]"
fi
if [[ "${RUN_BENCH}" == "1" ]]; then
  echo "                     \\-> [Benches (short hotpaths + pgbench + api-smoke)]"
fi

echo

# -----------------------------------------------------------------------------
# Guard rails
# -----------------------------------------------------------------------------

if [[ "${RUN_DB}" == "1" ]] && ! db_env_ready; then
  cat <<'EOF'
DB tests requested but CALIBER_DB_* env is incomplete.
Set at least: CALIBER_DB_HOST, CALIBER_DB_PORT, CALIBER_DB_USER, CALIBER_DB_NAME
Optional: CALIBER_DB_PASSWORD, CALIBER_DB_BOOTSTRAP_USER, CALIBER_DB_BOOTSTRAP_PASSWORD
EOF
  exit 2
fi

if [[ "${PG_INSTALL}" == "1" ]] && [[ "${RUN_DB}" != "1" ]]; then
  echo "Note: --pg-install has no effect without DB tests; enabling DB tests."
  RUN_DB=1
fi

# -----------------------------------------------------------------------------
# Execute (single source of truth: scripts/test.sh)
# -----------------------------------------------------------------------------

export DB_TESTS="${RUN_DB}"
export FUZZ="${RUN_FUZZ}"
export BENCH="${RUN_BENCH}"
export CALIBER_DB_RESET="${DB_RESET}"
export CALIBER_PG_INSTALL="${PG_INSTALL}"
if [[ "${RUN_FUZZ}" == "1" ]]; then
  export FUZZ_RUNS="${FUZZ_RUNS:-10000}"
fi

exec ./scripts/test.sh
