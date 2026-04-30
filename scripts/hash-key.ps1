#!/usr/bin/env pwsh
# Hashes a downstream API key with SHA-256 (hex). The proxy stores the
# hex digest in api_keys.hashed_key and rehashes the bearer token on
# every request to compare.
#
# Usage:
#   scripts/hash-key.ps1 'llmproxy-demo-key-replace-me'
param([Parameter(Mandatory=$true)][string]$Key)

$bytes = [System.Text.Encoding]::UTF8.GetBytes($Key)
$sha = [System.Security.Cryptography.SHA256]::Create()
$hash = $sha.ComputeHash($bytes)
($hash | ForEach-Object { $_.ToString('x2') }) -join ''
