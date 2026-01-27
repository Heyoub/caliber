#!/usr/bin/env bash
set -euo pipefail

API_URL="${CALIBER_API_URL:-http://localhost:3000}"
HEALTH_PATH="${CALIBER_API_HEALTH_PATH:-/api/v1/health}"
REQUESTS="${CALIBER_API_BENCH_REQUESTS:-20}"

if ! command -v curl >/dev/null 2>&1; then
  echo "curl not found; skipping API smoke bench."
  exit 0
fi

url="${API_URL%/}${HEALTH_PATH}"
echo "API smoke bench: ${REQUESTS}x GET ${url}"

# Preflight: skip if the API is not reachable.
preflight_code="$(curl -sS -o /dev/null -w "%{http_code}" "${url}" || true)"
if [[ "${preflight_code}" != "200" ]]; then
  echo "API smoke bench: preflight returned ${preflight_code:-ERR}; skipping."
  exit 0
fi

start_ms="$(date +%s%3N)"
ok=0
for _ in $(seq 1 "${REQUESTS}"); do
  code="$(curl -sS -o /dev/null -w "%{http_code}" "${url}" || true)"
  if [[ "${code}" == "200" ]]; then
    ok=$((ok + 1))
  fi
done
end_ms="$(date +%s%3N)"

elapsed_ms=$((end_ms - start_ms))
if (( REQUESTS > 0 )); then
  avg_ms=$((elapsed_ms / REQUESTS))
else
  avg_ms=0
fi

echo "API smoke bench: ${ok}/${REQUESTS} succeeded, total=${elapsed_ms}ms, avg=${avg_ms}ms"
