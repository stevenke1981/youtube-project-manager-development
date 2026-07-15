[CmdletBinding()]
param(
    [string]$ArtifactRoot,
    [switch]$InspectOnly
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $MyInvocation.MyCommand.Path | Split-Path -Parent
if ([string]::IsNullOrWhiteSpace($ArtifactRoot)) { $ArtifactRoot = Join-Path $repo 'target\release\bundle' }
$config = Get-Content (Join-Path $repo 'apps\desktop\src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json
$root = [IO.Path]::GetFullPath($ArtifactRoot)
if (-not (Test-Path -LiteralPath $root -PathType Container)) {
    throw "INSTALLER_SMOKE_FAIL: artifact root not found: $root"
}
$files = @(Get-ChildItem -LiteralPath $root -Recurse -File)
$msi = @($files | Where-Object { $_.Extension -eq '.msi' -and $_.Name -match [regex]::Escape([string]$config.version) })
$nsis = @($files | Where-Object { $_.Extension -eq '.exe' -and $_.Name -match 'setup' -and $_.Name -match [regex]::Escape([string]$config.version) })
if ($msi.Count -eq 0 -or $nsis.Count -eq 0) {
    throw "INSTALLER_SMOKE_FAIL: expected MSI and NSIS artifacts for version $($config.version) under $root"
}
foreach ($artifact in @($msi) + @($nsis)) {
    if ($artifact.Length -le 0) { throw "INSTALLER_SMOKE_FAIL: empty artifact $($artifact.FullName)" }
    $hash = (Get-FileHash -LiteralPath $artifact.FullName -Algorithm SHA256).Hash
    Write-Host "PASS $($artifact.Name) SHA256=$hash" -ForegroundColor Green
}
Write-Host "PASS installer artifacts match $($config.productName) $($config.version)" -ForegroundColor Green
if ($InspectOnly) {
    Write-Host 'INFO inspect-only mode: no installer was executed and no user installation was changed.' -ForegroundColor Cyan
}
