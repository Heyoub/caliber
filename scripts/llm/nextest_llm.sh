#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/target/nextest"
JSONL="$OUT_DIR/summary.jsonl"
SUMMARY_JSON="$OUT_DIR/summary.json"
JUNIT_XML="$OUT_DIR/junit.xml"

mkdir -p "$OUT_DIR"

# LLM-friendly defaults without flag spam
export RUST_BACKTRACE="${RUST_BACKTRACE:-short}"
export NEXTEST_MESSAGE_FORMAT="${NEXTEST_MESSAGE_FORMAT:-libtest-json-plus}"
export NEXTEST_MESSAGE_FORMAT_VERSION="${NEXTEST_MESSAGE_FORMAT_VERSION:-1}"

# Run tests and capture JSONL output
cargo nextest run --profile ci --workspace --exclude caliber-pg \
  2>&1 | tee "$JSONL"

# Convert JSONL to JUnit + summary JSON
node "$ROOT_DIR/scripts/llm/nextest_to_junit.js" \
  "$JSONL" "$JUNIT_XML" "$SUMMARY_JSON"

echo "LLM reports written:"
echo "  $JSONL"
echo "  $SUMMARY_JSON"
echo "  $JUNIT_XML"
