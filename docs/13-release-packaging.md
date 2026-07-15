# Release packaging

YTPM v0.2.0 produces both Windows MSI and NSIS installers through Tauri:

```powershell
cd D:\YouTube-Project-Manager-Development\apps\desktop
npm run desktop:build
cd ..\..
.\scripts\release-checks.ps1 -Check All
.\scripts\installer-smoke.ps1 -InspectOnly
```

`release-checks.ps1` verifies WebView2, FFmpeg, MSI/NSIS artifacts and writes `target/release/SHA256SUMS.txt`. The smoke script only inspects generated files; it never silently installs, uninstalls, deletes, or modifies a user installation.

The installer does not bundle user project data, SQLite indexes, OAuth tokens, FFmpeg, or WebView2. FFmpeg/FFprobe are discovered from `PATH`, `YTPM_FFMPEG_PATH`, or `YTPM_FFPROBE_PATH` at runtime.
