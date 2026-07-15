# NLE v1.0 Implementation Blueprint

Status: approved by the user's 2026-07-15 implementation request.

## Outcome

Deliver a usable non-destructive desktop editor whose portable `timeline.json` remains the source of truth. The milestone adds typed professional editing controls, subtitle burn-in, deterministic FFmpeg graph compilation, and cancellable background export without claiming binary feature parity with Adobe Premiere Pro.

## Data contract

- Upgrade `timeline.json` from schema v1 to v2.
- Add validated typed clip effects: color adjustment, blur, sharpen, vignette, chroma key, fade in/out, and transform/opacity.
- Add portable subtitle render style to render settings.
- Migrate v1 atomically, write a unique `.ytpm-backup/` snapshot first, and keep the migration rerunnable.
- Reject raw FFmpeg filter text, absolute paths, traversal, reparse points, invalid numbers, unsupported effect kinds, and invalid time ranges.

## Render pipeline

1. Validate the portable timeline and resolve project-relative media paths.
2. Parse subtitle tracks from SRT or WebVTT and transform cue times into timeline time.
3. Generate a temporary UTF-8 ASS file with the selected style.
4. Compile one deterministic `filter_complex` from typed values for video overlays, audio mixing, transitions, effects, transform, and subtitle burn-in.
5. Spawn FFmpeg with an argument array, monitor progress, and kill the child when cancelled.
6. Atomically publish only a successful output; preserve the previous output and source media on failure.

## Background jobs

- One desktop-owned worker serializes CPU/GPU-heavy exports.
- Jobs expose `queued`, `running`, `completed`, `failed`, and `cancelled` states plus progress and actionable messages.
- Queue state is process-local. Portable timeline/source files and finalized outputs are the durable recovery boundary.
- Cancelling a queued job prevents start; cancelling a running job terminates its FFmpeg child and cleans temporary work files.

## Desktop flow

1. Select a project asset and target track.
2. Add or adjust the clip and typed effects.
3. Configure subtitle style when a subtitle track is used.
4. Queue export, observe progress, or cancel.
5. Review the completed output before publishing.

Every numbered step must identify the next primary control in the UI and remain usable at Windows 125% and 150% scaling.

## Acceptance

- A migrated v1 timeline opens as v2 and retains a backup.
- A multi-track fixture produces a deterministic filter graph with video placement, mixed audio, typed effects, and ASS subtitle burn-in.
- SRT and WebVTT cues are trimmed and offset correctly.
- Background export returns immediately, reports progress, completes a real FFmpeg fixture, and can cancel a running child.
- Invalid paths/effect values fail before FFmpeg starts.
- Rust format/clippy/tests and frontend typecheck/tests/build pass.
- Playwright or a reproducible desktop verification record covers the numbered editor flow.
- MSI and NSIS release smoke checks pass; no Library fixture is removed.
