# YouTube publishing

Publishing is a two-gate operation:

1. Save `08_metadata/publish.json`.
2. Run `publish_dry_run`; it checks title, description, tags, schedule, the final `09_exports/final.mp4`, and OAuth readiness without network access.
3. Explicitly confirm upload. The core exchanges a refresh token and uses YouTube Data API v3 resumable `videos.insert` with `snippet,status`.

Credentials are supplied through `YTPM_YOUTUBE_CLIENT_ID` and `YTPM_YOUTUBE_CLIENT_SECRET`; they are never persisted in the portable project or SQLite. The OAuth helper builds an installed-app loopback URL with state and PKCE. The desktop UI provides step `5B` to paste the returned callback URL; a refresh token is held only in the current App process so the user can upload immediately. A new App session can be bootstrapped with `YTPM_YOUTUBE_REFRESH_TOKEN` when required.

An upload requires `confirm=true` in Tauri or `--confirm` in CLI. Missing credentials, invalid metadata, missing final video, API refusal, and cancellation are surfaced as recoverable errors.
