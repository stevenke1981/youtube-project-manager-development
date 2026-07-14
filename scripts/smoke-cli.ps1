$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)
$chinese = [string]::Concat([char]0x4E2D, [char]0x6587)
$TempRoot = Join-Path $env:TEMP ("ytpm-smoke-" + $chinese)
New-Item -ItemType Directory -Path $TempRoot -Force | Out-Null

$title = [string]::Concat("Test", [char]0xFF1A, "Video", [char]0xFF1F)
$channel = [string]::Concat("Test", [char]0x983B, [char]0x9053)
cargo run -p ytpm-cli -- create --root $TempRoot --title $title --channel $channel
cargo run -p ytpm-cli -- list --root $TempRoot --json
$Project = Get-ChildItem $TempRoot -Directory | Select-Object -First 1
cargo run -p ytpm-cli -- validate --path $Project.FullName --json

Write-Host "Smoke test data retained at $TempRoot for inspection." -ForegroundColor Yellow
