#!/bin/bash
BASE=http://127.0.0.1:8081
echo "--- (1) /api/setup/status ---"
curl -fs $BASE/api/setup/status; echo
echo
echo "--- (2) login ---"
LOGIN=$(curl -fs -X POST $BASE/api/auth/login \
  -H "Content-Type: application/json" \
  --data-binary @/tmp/login.json)
echo "$LOGIN" | head -c 80; echo "..."
TOK=$(echo "$LOGIN" | python3 -c "import sys,json;print(json.load(sys.stdin)['token'])")
echo "token-len=${#TOK}"
echo
hit() {
  local label=$1 path=$2
  echo "--- $label $path ---"
  curl -s -o /tmp/r -w "HTTP=%{http_code}\n" -H "Authorization: Bearer $TOK" "$BASE$path"
  cat /tmp/r; echo
  echo
}
hit "(3)" /api/me
hit "(4)" /api/providers
hit "(5)" /api/aliases
hit "(6)" /api/key-pools
hit "(7)" /api/vision-mappings
hit "(8)" /api/stats
hit "(9)" /api/events/failovers
echo "--- (10) GET /providers (SPA fallback) ---"
curl -fsI $BASE/providers | head -3
echo
echo "--- (11) proxy /healthz ---"
curl -fs http://127.0.0.1:8080/healthz; echo
