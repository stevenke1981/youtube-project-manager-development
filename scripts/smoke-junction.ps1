$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

$runningOnWindows = [System.Environment]::OSVersion.Platform -eq [System.PlatformID]::Win32NT
if (-not $runningOnWindows) {
    Write-Host "SKIP: junction smoke test requires Windows."
    exit 0
}

# Keep the script source ASCII-safe for Windows PowerShell 5.1 (UTF-8 without a
# BOM is otherwise decoded with the active code page). Build the Chinese path
# components from Unicode code points so this smoke test really exercises a
# non-ASCII Windows path.
$chinese = [string]::Concat([char]0x4E2D, [char]0x6587)
$base = Join-Path ([System.IO.Path]::GetTempPath()) ("ytpm-junction-smoke-" + $chinese + "-" + [guid]::NewGuid().ToString("N"))
$libraryRoot = Join-Path $base ($chinese + " Library")
New-Item -ItemType Directory -Path $libraryRoot -Force | Out-Null

cargo run --quiet -p ytpm-cli -- create --root $libraryRoot --title "junction fixture" --channel "smoke"
if ($LASTEXITCODE -ne 0) {
    throw "建立 junction smoke project 失敗，exit code=$LASTEXITCODE"
}

$project = Get-ChildItem -LiteralPath $libraryRoot -Directory |
    Where-Object { $_.Name -ne "_archive" } |
    Select-Object -First 1
if ($null -eq $project) {
    throw "找不到 junction smoke project"
}

$requiredDirectory = Join-Path $project.FullName "06_subtitles/translations"
$junctionTarget = Join-Path $base ($chinese + " junction target")
New-Item -ItemType Directory -Path $junctionTarget -Force | Out-Null
[System.IO.Directory]::Delete($requiredDirectory)

$mklinkArguments = '/c mklink /J "{0}" "{1}"' -f $requiredDirectory, $junctionTarget
$mklink = Start-Process -FilePath "cmd.exe" -ArgumentList $mklinkArguments -Wait -PassThru -WindowStyle Hidden
if ($mklink.ExitCode -ne 0) {
    throw "建立 junction fixture 失敗，exit code=$($mklink.ExitCode)"
}

$validationOutput = & cargo run --quiet -p ytpm-cli -- validate --path $project.FullName --json 2>&1
$validationExitCode = $LASTEXITCODE
$validationOutput | Out-Host

if ($validationExitCode -eq 0) {
    throw "junction validation 應回傳非零 exit code"
}
if (($validationOutput -join "`n") -notmatch "REQUIRED_DIRECTORY_SYMLINK") {
    throw "validation output 缺少 REQUIRED_DIRECTORY_SYMLINK"
}

Write-Host "Junction smoke passed: validation returned non-zero as expected. Fixture: $base" -ForegroundColor Green
