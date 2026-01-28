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

# Try per-job logs first (best structure). Fall back to full run log.
set +e
job_ids="$(gh run view "$run_id" --json jobs -q '.jobs[].databaseId' 2>/dev/null)"
job_names="$(gh run view "$run_id" --json jobs -q '.jobs[].name' 2>/dev/null)"
set -e

if [ -n "${job_ids:-}" ] && [ -n "${job_names:-}" ]; then
  i=1
  while IFS= read -r job_id; do
    job_name="$(printf "%s\n" "$job_names" | sed -n "${i}p")"
    safe_name="$(printf "%s" "$job_name" | tr '/:' '__')"
    gh run view "$run_id" --log --job "$job_id" > "$dest/${i}_${safe_name}.txt"
    i=$((i + 1))
  done <<< "$job_ids"
  echo "Logs downloaded to $dest"
  exit 0
fi

gh run view "$run_id" --log > "$dest/run.log"
echo "Logs downloaded to $dest (single file)"
