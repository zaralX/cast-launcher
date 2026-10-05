#!/usr/bin/env pwsh
# Changes the launcher version on Windows. All the logic lives in set-version.mjs next to it.
#
#   .\scripts\set-version.ps1            show the current versions
#   .\scripts\set-version.ps1 1.5.0      set a new one everywhere
#   .\scripts\set-version.ps1 1.5.0 -n   show what would change without writing

$ErrorActionPreference = "Stop"

if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Error "node not found: it is needed to build the frontend too, install Node.js"
    exit 1
}

& node (Join-Path $PSScriptRoot "set-version.mjs") @args

exit $LASTEXITCODE
