#!/usr/bin/env bash
# Hashes a downstream API key with SHA-256 (hex). The proxy stores the
# hex digest in api_keys.hashed_key and rehashes the bearer token on
# every request to compare.
#
# Usage:
#   scripts/hash-key.sh 'llmproxy-demo-key-replace-me'
set -euo pipefail
if [ $# -ne 1 ]; then
  echo "usage: $0 <api-key>" >&2
  exit 2
fi
printf '%s' "$1" | sha256sum | awk '{print $1}'
