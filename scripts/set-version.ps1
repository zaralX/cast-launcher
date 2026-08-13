#!/usr/bin/env pwsh
# Смена версии лаунчера, Windows. Вся логика - в set-version.mjs рядом.
#
# ВАЖНО: файл должен лежать в UTF-8 с BOM. Windows PowerShell 5.1 без BOM читает
# его как ANSI, кириллица превращается в мусор и скрипт падает на разборе.
#
#   .\scripts\set-version.ps1            показать текущие версии
#   .\scripts\set-version.ps1 1.5.0      проставить новую везде
#   .\scripts\set-version.ps1 1.5.0 -n   показать, что изменится, но не писать

$ErrorActionPreference = "Stop"

if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Error "Не найден node - он нужен и для сборки фронта, поставьте Node.js"
    exit 1
}

& node (Join-Path $PSScriptRoot "set-version.mjs") @args

exit $LASTEXITCODE
