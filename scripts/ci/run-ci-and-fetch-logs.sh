#!/usr/bin/env bash
set -euo pipefail

workflow="${1:-CI}"
ref="${2:-}"

if [ -z "$ref" ]; then
  ref="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo main)"
fi

if ! command -v gh >/dev/null 2>&1; then
  echo "gh CLI not found. Install GitHub CLI first."
  exit 1
fi

start_iso="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

echo "Triggering workflow \"$workflow\" on ref \"$ref\"..."
gh workflow run "$workflow" --ref "$ref" >/dev/null

echo "Waiting for run to appear..."
run_id=""
for _ in {1..60}; do
  run_id="$(gh run list \
    --workflow "$workflow" \
    --branch "$ref" \
    --json databaseId,event,createdAt \
    -q "map(select(.event==\"workflow_dispatch\" and .createdAt >= \"$start_iso\")) | .[0].databaseId")"
  if [ -n "$run_id" ] && [ "$run_id" != "null" ]; then
    break
  fi
  sleep 5
done

if [ -z "$run_id" ] || [ "$run_id" = "null" ]; then
  echo "Could not find the workflow run ID. Try: gh run list --workflow \"$workflow\" --branch \"$ref\""
  exit 1
fi

echo "Run ID: $run_id"

set +e
gh run watch "$run_id" --exit-status
watch_status=$?
set -e

scripts/ci/fetch-gh-logs.sh "$run_id"
exit "$watch_status"
