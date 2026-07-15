# NLE v1.0 UI and Release Verification

Date: 2026-07-15

Platform: Windows, Tauri release build

## Reproducible checks executed

1. `npm run typecheck` — passed.
2. `npm run test` — 20/20 passed, including MP4-only requests, legacy output normalization, effect defaults, asset/track compatibility, job merge and terminal states.
3. `npm run build` — passed; Vite transformed 1,594 modules.
4. `npm run desktop:build` — passed; release executable, MSI and NSIS were produced.
5. The release executable was started for three seconds and remained alive with window title `YouTube Project Manager`; the smoke process was then stopped without installing or changing a Library.
6. `scripts/release-checks.ps1 -Check All` and `scripts/installer-smoke.ps1 -InspectOnly` — passed.

## Numbered editor flow inspected

The rendered component order and labels are:

1. Select a compatible asset and run FFprobe.
2. Select a compatible track and add the clip.
3. Edit clip timing, transition and video-only typed effects.
4. Configure subtitle burn-in style.
5. Save portable `timeline.json`.
6. Enqueue MP4 export to `09_exports/final.mp4`.
7. Observe project-scoped progress or cancel FFmpeg.
8. Save publishing metadata and complete optional OAuth.
9. Run a no-upload dry-run.
10. Explicitly confirm YouTube upload.

The layout collapses editor grids to one column at 980px and 720px breakpoints. Export progress has an accessible name, errors use `role=alert`, status uses `role=status`, and the YouTube upload retains both UI and Tauri confirmation gates.

## Deliberately not executed

- MSI/NSIS install, upgrade and uninstall were not executed on the current user profile. The inspect-only smoke verified artifacts without mutating the installed system.
- YouTube upload was not executed because that would transmit user media and requires user-provided OAuth credentials and explicit confirmation.
