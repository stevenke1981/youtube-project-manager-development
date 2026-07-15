[CmdletBinding()]
param(
    [ValidateSet('All', 'Environment', 'Artifacts', 'Checksums')]
    [string]$Check = 'All',
    [string]$ArtifactRoot,
    [string]$ChecksumPath,
    [string]$FfmpegPath
)

$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path | Split-Path -Parent
if ([string]::IsNullOrWhiteSpace($ArtifactRoot)) { $ArtifactRoot = Join-Path $repoRoot 'target\release\bundle' }
if ([string]::IsNullOrWhiteSpace($ChecksumPath)) { $ChecksumPath = Join-Path $repoRoot 'target\release\SHA256SUMS.txt' }

function Write-ReleaseStatus {
    param(
        [ValidateSet('PASS', 'SKIP', 'FAIL', 'INFO')]
        [string]$Status,
        [Parameter(Mandatory)]
        [string]$Message
    )

    $color = switch ($Status) {
        'PASS' { 'Green' }
        'SKIP' { 'Yellow' }
        'FAIL' { 'Red' }
        default { 'Cyan' }
    }

    Write-Host "[$Status] $Message" -ForegroundColor $color
}

function Resolve-ReleasePath {
    param(
        [Parameter(Mandatory)]
        [string]$Path
    )

    if ([IO.Path]::IsPathRooted($Path)) {
        return [IO.Path]::GetFullPath($Path)
    }

    return [IO.Path]::GetFullPath((Join-Path (Split-Path $PSScriptRoot -Parent) $Path))
}

function Get-TauriReleaseConfig {
    $configPath = Join-Path (Split-Path $PSScriptRoot -Parent) 'apps\desktop\src-tauri\tauri.conf.json'
    if (-not (Test-Path -LiteralPath $configPath -PathType Leaf)) {
        throw "RELEASE_CONFIG_FAIL: Tauri config was not found at '$configPath'."
    }

    try {
        $config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
    }
    catch {
        throw "RELEASE_CONFIG_FAIL: Could not parse '$configPath': $($_.Exception.Message)"
    }

    if ([string]::IsNullOrWhiteSpace([string]$config.productName) -or [string]::IsNullOrWhiteSpace([string]$config.version)) {
        throw "RELEASE_CONFIG_FAIL: productName and version must be present in '$configPath'."
    }

    return $config
}

function Get-WebView2Runtime {
    $registryPatterns = @(
        'HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\*',
        'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\*',
        'HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\*'
    )

    foreach ($pattern in $registryPatterns) {
        try {
            $entries = @(Get-ItemProperty -Path $pattern -ErrorAction Stop)
        }
        catch {
            continue
        }

        foreach ($entry in $entries) {
            $entryName = [string]$entry.name
            $entryPath = [string]$entry.PSPath
            $version = [string]$entry.pv
            if (-not [string]::IsNullOrWhiteSpace($version) -and ($entryName -match 'WebView2' -or $entryPath -match '\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5\}')) {
                return [pscustomobject]@{
                    Version = $version
                    Name = $entryName
                    RegistryPath = $entryPath
                }
            }
        }
    }

    return $null
}

function Resolve-FFmpeg {
    param(
        [string]$Path
    )

    if (-not [string]::IsNullOrWhiteSpace($Path)) {
        $resolved = Resolve-Path -LiteralPath $Path -ErrorAction SilentlyContinue
        if ($null -eq $resolved -or -not (Test-Path -LiteralPath $resolved.Path -PathType Leaf)) {
            throw "RELEASE_ENVIRONMENT_FAIL: FFmpeg was not found at '$Path'. Install FFmpeg or pass -FfmpegPath to ffmpeg.exe."
        }

        return $resolved.Path
    }

    $command = Get-Command ffmpeg.exe -ErrorAction SilentlyContinue
    if ($null -eq $command -or $command.CommandType -ne 'Application') {
        $wingetHint = if (Get-Command winget.exe -ErrorAction SilentlyContinue) {
            'Suggested command: winget install --id Gyan.FFmpeg.Shared -e'
        }
        else {
            'winget is unavailable; install FFmpeg manually and add its bin directory to PATH'
        }

        throw "RELEASE_ENVIRONMENT_FAIL: FFmpeg (ffmpeg.exe) is unavailable. $wingetHint."
    }

    if (-not [string]::IsNullOrWhiteSpace([string]$command.Path)) {
        return $command.Path
    }

    return $command.Source
}

function Assert-ReleaseEnvironment {
    param(
        [string]$FfmpegPath
    )

    $webView2 = Get-WebView2Runtime
    if ($null -eq $webView2) {
        $wingetHint = if (Get-Command winget.exe -ErrorAction SilentlyContinue) {
            'Suggested command: winget install --id Microsoft.EdgeWebView2Runtime -e'
        }
        else {
            'winget is unavailable; install the WebView2 Runtime manually'
        }

        throw "RELEASE_ENVIRONMENT_FAIL: Microsoft Edge WebView2 Runtime is unavailable. $wingetHint. The Tauri installer smoke cannot continue without it."
    }

    Write-ReleaseStatus PASS "WebView2 Runtime $($webView2.Version) detected ($($webView2.Name))."

    $ffmpeg = Resolve-FFmpeg -Path $FfmpegPath
    try {
        $null = & $ffmpeg '-version' 2>&1
        $ffmpegExitCode = $LASTEXITCODE
    }
    catch {
        throw "RELEASE_ENVIRONMENT_FAIL: FFmpeg could not be started at '$ffmpeg': $($_.Exception.Message)"
    }

    if ($ffmpegExitCode -ne 0) {
        throw "RELEASE_ENVIRONMENT_FAIL: FFmpeg at '$ffmpeg' returned exit code $ffmpegExitCode for -version. Verify the installation and PATH."
    }

    Write-ReleaseStatus PASS "FFmpeg is available at '$ffmpeg'."

    if (-not (Get-Command winget.exe -ErrorAction SilentlyContinue)) {
        Write-ReleaseStatus SKIP 'winget is unavailable; prerequisite installation is not attempted by these checks.'
    }
    else {
        Write-ReleaseStatus INFO 'winget is available for actionable prerequisite repair hints.'
    }

    return [pscustomobject]@{
        WebView2 = $webView2
        FfmpegPath = $ffmpeg
    }
}

function Get-ReleaseArtifacts {
    param(
        [Parameter(Mandatory)]
        [string]$Path
    )

    $artifactRoot = Resolve-ReleasePath -Path $Path
    if (-not (Test-Path -LiteralPath $artifactRoot -PathType Container)) {
        throw "BUILD_ARTIFACT_FAIL: Release artifact directory was not found at '$artifactRoot'. Run 'npm run desktop:build' first."
    }

    $config = Get-TauriReleaseConfig
    $namePattern = [regex]::Escape([string]$config.productName)
    $versionPattern = [regex]::Escape([string]$config.version)
    $files = @(Get-ChildItem -LiteralPath $artifactRoot -Recurse -File)
    $msi = @($files | Where-Object { $_.Name -match "^${namePattern}_${versionPattern}.*\.msi$" })
    $nsis = @($files | Where-Object { $_.Name -match "^${namePattern}_${versionPattern}.*-setup\.exe$" })

    if ($msi.Count -eq 0) {
        throw "BUILD_ARTIFACT_FAIL: No MSI for $($config.productName) version $($config.version) was found below '$artifactRoot'. Run 'npm run desktop:build' and inspect target\release\bundle\msi."
    }

    if ($nsis.Count -eq 0) {
        throw "BUILD_ARTIFACT_FAIL: No NSIS installer for $($config.productName) version $($config.version) was found below '$artifactRoot'. Run 'npm run desktop:build' and inspect target\release\bundle\nsis."
    }

    $all = @($msi) + @($nsis)
    foreach ($artifact in $all) {
        if ($artifact.Length -le 0) {
            throw "BUILD_ARTIFACT_FAIL: Artifact '$($artifact.FullName)' is empty. Rebuild the desktop bundle."
        }
    }

    Write-ReleaseStatus PASS "Found $($msi.Count) MSI artifact(s) and $($nsis.Count) NSIS artifact(s) for version $($config.version)."

    return [pscustomobject]@{
        Root = $artifactRoot
        Config = $config
        Msi = $msi
        Nsis = $nsis
        All = $all
    }
}

function Write-ReleaseChecksums {
    param(
        [Parameter(Mandatory)]
        [pscustomobject]$Artifacts,
        [Parameter(Mandatory)]
        [string]$Path
    )

    $checksumPath = Resolve-ReleasePath -Path $Path
    $checksumDirectory = Split-Path -Parent $checksumPath
    if (-not (Test-Path -LiteralPath $checksumDirectory -PathType Container)) {
        New-Item -ItemType Directory -Path $checksumDirectory -Force | Out-Null
    }

    $root = $Artifacts.Root.TrimEnd('\') + '\'
    $lines = foreach ($artifact in @($Artifacts.All | Sort-Object FullName)) {
        $relative = $artifact.FullName.Substring($root.Length).Replace('\', '/')
        $hash = (Get-FileHash -LiteralPath $artifact.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $relative"
    }

    $temporaryPath = "$checksumPath.$([guid]::NewGuid().ToString('N')).tmp"
    try {
        $content = ($lines -join [Environment]::NewLine) + [Environment]::NewLine
        [IO.File]::WriteAllText($temporaryPath, $content, [Text.UTF8Encoding]::new($false))
        Move-Item -LiteralPath $temporaryPath -Destination $checksumPath -Force
    }
    finally {
        if (Test-Path -LiteralPath $temporaryPath -PathType Leaf) {
            Remove-Item -LiteralPath $temporaryPath -Force
        }
    }

    Write-ReleaseStatus PASS "SHA-256 checksums written to '$checksumPath'."
    return $checksumPath
}

function Invoke-ReleaseChecks {
    param(
        [ValidateSet('All', 'Environment', 'Artifacts', 'Checksums')]
        [string]$Mode = 'All',
        [string]$ArtifactsPath = $ArtifactRoot,
        [string]$ChecksumsPath = $ChecksumPath,
        [string]$Ffmpeg = $FfmpegPath
    )

    $environment = $null
    $artifacts = $null

    if ($Mode -eq 'All' -or $Mode -eq 'Environment') {
        $environment = Assert-ReleaseEnvironment -FfmpegPath $Ffmpeg
    }

    if ($Mode -eq 'All' -or $Mode -eq 'Artifacts' -or $Mode -eq 'Checksums') {
        $artifacts = Get-ReleaseArtifacts -Path $ArtifactsPath
    }

    $checksum = $null
    if ($Mode -eq 'All' -or $Mode -eq 'Checksums') {
        $checksum = Write-ReleaseChecksums -Artifacts $artifacts -Path $ChecksumsPath
    }

    return [pscustomobject]@{
        Environment = $environment
        Artifacts = $artifacts
        ChecksumPath = $checksum
    }
}

# Keep the functions available when this file is dot-sourced by installer-smoke.ps1.
if ($MyInvocation.InvocationName -ne '.') {
    try {
        $null = Invoke-ReleaseChecks -Mode $Check -ArtifactsPath $ArtifactRoot -ChecksumsPath $ChecksumPath -Ffmpeg $FfmpegPath
        Write-ReleaseStatus PASS "Release checks completed in '$Check' mode."
        exit 0
    }
    catch {
        Write-ReleaseStatus FAIL $_.Exception.Message
        exit 1
    }
}
