#!/usr/bin/env bash
set -euo pipefail

mkdir -p target/tmp
export TMPDIR="$PWD/target/tmp"

echo "==> Clippy (workspace)"
cargo clippy --workspace --all-targets --all-features --exclude caliber-pg

echo "==> Cargo tests (workspace, excluding pgrx)"
cargo test --workspace --all-targets --exclude caliber-pg

if [[ "${DB_TESTS:-}" == "1" ]]; then
  echo "==> DB-backed API tests (requires CALIBER_DB_* env)"
  cargo test -p caliber-api --tests --features db-tests
fi

echo "==> PGRX tests (pg18)"
cargo pgrx test pg18 --package caliber-pg
