# Media pipeline

The media pipeline is deliberately non-destructive:

1. `timeline.json` schema v2 stores UUID clips, project-relative source paths, trim points, tracks, volume/mute, typed effects, transitions, subtitle style and render settings. A v1 file is backed up below `.ytpm-backup/` before atomic migration.
2. `media_probe` calls `ffprobe -v error -print_format json -show_format -show_streams <path>` using `std::process::Command`; no shell string is accepted.
3. The typed filter compiler creates one deterministic `filter_complex` for multi-track video overlays, audio placement/mixing, transform/opacity, color adjustment, blur, sharpen, vignette, chroma key, fades and validated transitions. Project JSON cannot contain raw filter fragments.
4. Subtitle tracks accept project-relative SRT or WebVTT. Cues are clipped and shifted to timeline time, escaped into a temporary UTF-8 ASS file, styled from `timeline.output.subtitle_style`, and burned into the video.
5. `media_export_enqueue` returns immediately. A single process-local worker exposes queued/running/completed/failed/cancelled state and progress; cancellation sets an atomic flag and terminates the running FFmpeg child.
6. FFmpeg receives argument arrays only and writes `render.mp4` inside its job-owned `.ytpm-media-*` directory with overwrite disabled. Success publishes through an atomic no-overwrite hard link; concurrent jobs cannot replace the final output. Failure or cancellation removes only job-owned temporary data without changing source media or an existing export.

Set `YTPM_FFPROBE_PATH` or `YTPM_FFMPEG_PATH` when the tools are not on `PATH`. A missing tool, unsafe path, invalid timeline, or FFmpeg failure is returned as an actionable structured error.

Queue records intentionally last only for the current App process. `timeline.json`, source assets and completed exports are the durable recovery boundary; restart the App and enqueue an unfinished export again.
