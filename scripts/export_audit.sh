#!/usr/bin/env bash
# export_audit.sh: Export AGC audit log via REST API
#
# Usage: AGC_URL=http://127.0.0.1:8080 ./scripts/export_audit.sh [ndjson|csv]
#
# In v0.2.0, the export endpoints are:
#   GET /api/v1/audit/export.ndjson
#   GET /api/v1/audit/export.csv
set -euo pipefail

AGC_URL="${AGC_URL:-http://127.0.0.1:8080}"
FORMAT="${1:-ndjson}"
OUT_FILE="audit-$(date +%Y%m%d-%H%M%S).${FORMAT}"

echo "AGC URL  : $AGC_URL"
echo "Format   : $FORMAT"
echo "Output   : $OUT_FILE"

# Health check
if ! curl -sf "$AGC_URL/health" > /dev/null; then
  echo "Error: AGC API not reachable at $AGC_URL" >&2
  exit 1
fi

# Export (v0.2.0+ endpoint)
curl -sf "$AGC_URL/api/v1/audit/export.$FORMAT" -o "$OUT_FILE"
echo "Exported : $OUT_FILE ($(wc -l < "$OUT_FILE") lines)"
