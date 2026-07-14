$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

Write-Host "[1/4] Checking Node.js..."
node --version
npm --version

Write-Host "[2/4] Checking Rust..."
rustc --version
cargo --version

Write-Host "[3/4] Installing frontend dependencies..."
npm install

Write-Host "[4/4] Verifying toolchain..."
cargo metadata --no-deps --format-version 1 | Out-Null
npm run typecheck

Write-Host "Bootstrap complete." -ForegroundColor Green
