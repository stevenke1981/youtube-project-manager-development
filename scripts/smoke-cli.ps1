$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)
$TempRoot = Join-Path $env:TEMP "ytpm-smoke-中文"
New-Item -ItemType Directory -Path $TempRoot -Force | Out-Null

cargo run -p ytpm-cli -- create --root $TempRoot --title "測試：影片？" --channel "測試頻道"
cargo run -p ytpm-cli -- list --root $TempRoot --json
$Project = Get-ChildItem $TempRoot -Directory | Select-Object -First 1
cargo run -p ytpm-cli -- validate --path $Project.FullName --json

Write-Host "Smoke test data retained at $TempRoot for inspection." -ForegroundColor Yellow
