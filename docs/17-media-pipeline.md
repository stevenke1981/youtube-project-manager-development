# Media pipeline

The media pipeline is deliberately non-destructive:

1. `timeline.json` stores UUID clips, project-relative source paths, trim points, tracks, volume/mute, transitions and render settings.
2. `media_probe` calls `ffprobe -v error -print_format json -show_format -show_streams <path>` using `std::process::Command`; no shell string is accepted.
3. `media_export` validates the timeline, trims each video clip into a temporary `.ytpm-media-*` folder, concatenates those segments with FFmpeg, and refuses to overwrite an existing output.
4. Temporary render files are removed after the operation; user source files and existing exports are not removed.

Set `YTPM_FFPROBE_PATH` or `YTPM_FFMPEG_PATH` when the tools are not on `PATH`. A missing tool, unsafe path, invalid timeline, or FFmpeg failure is returned as an actionable structured error.
