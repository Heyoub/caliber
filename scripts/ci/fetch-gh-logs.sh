#!/usr/bin/env bash
set -euo pipefail

if ! command -v gh >/dev/null 2>&1; then
  echo "gh CLI not found. Install GitHub CLI first."
  exit 1
fi

if [ "${1:-}" = "" ]; then
  echo "Usage: $0 <run-id>"
  exit 1
fi

run_id="$1"
dest=".github/.logs/logs_${run_id}"

mkdir -p "$dest"
gh run download "$run_id" --log -D "$dest"
echo "Logs downloaded to $dest"
