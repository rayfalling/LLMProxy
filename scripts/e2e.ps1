#!/usr/bin/env pwsh
# End-to-end harness for the LLMProxy WebUI:
#   1. builds the frontend bundle (web/dist)
#   2. builds the dashboard binary (which embeds web/dist)
#   3. wipes the temp e2e SQLite DB
#   4. runs Playwright (which spawns the dashboard via webServer)
#
# Usage: scripts/e2e.ps1
$ErrorActionPreference = 'Continue'

$root = Split-Path -Parent $PSScriptRoot
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
if (-not (Test-Path $cargo)) { $cargo = 'cargo' }

Write-Host '== building web/dist (vite) ==' -ForegroundColor Cyan
Push-Location (Join-Path $root 'web')
try {
    npm run build 2>&1 | ForEach-Object { "$_" }
    if ($LASTEXITCODE -ne 0) { throw "vite build failed: $LASTEXITCODE" }
} finally {
    Pop-Location
}

Write-Host '== building dashboard binary ==' -ForegroundColor Cyan
& $cargo build -p dashboard 2>&1 | ForEach-Object { "$_" }
if ($LASTEXITCODE -ne 0) { throw "cargo build failed: $LASTEXITCODE" }

$testData = Join-Path $root 'web\test-data'
if (Test-Path $testData) {
    Write-Host "== wiping $testData ==" -ForegroundColor Cyan
    Remove-Item $testData -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $testData | Out-Null

Write-Host '== running playwright ==' -ForegroundColor Cyan
Push-Location (Join-Path $root 'web')
try {
    npx playwright test @args
    if ($LASTEXITCODE -ne 0) { throw "playwright test failed: $LASTEXITCODE" }
} finally {
    Pop-Location
}
