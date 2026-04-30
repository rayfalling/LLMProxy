#!/usr/bin/env bash
# End-to-end harness for the LLMProxy WebUI:
#   1. builds the frontend bundle (web/dist)
#   2. builds the dashboard binary (which embeds web/dist)
#   3. wipes the temp e2e SQLite DB
#   4. runs Playwright (which spawns the dashboard via webServer)
#
# Usage: scripts/e2e.sh [extra playwright args...]
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"

echo "== building web/dist (vite) =="
( cd "$root/web" && npm run build )

echo "== building dashboard binary =="
( cd "$root" && cargo build -p dashboard )

testdata="$root/web/test-data"
if [ -d "$testdata" ]; then
  echo "== wiping $testdata =="
  rm -rf "$testdata"
fi
mkdir -p "$testdata"

echo "== running playwright =="
( cd "$root/web" && npx playwright test "$@" )
