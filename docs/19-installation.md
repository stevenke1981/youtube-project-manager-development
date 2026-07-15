# Windows installation and first run

1. Install the MSI or NSIS package as the current Windows user.
2. Install Microsoft Edge WebView2 Runtime if the installer asks for it.
3. Install FFmpeg with FFprobe and put both executables on `PATH` (or set `YTPM_FFMPEG_PATH` and `YTPM_FFPROBE_PATH`).
4. Open the app and choose a Library root.
5. Create or open a project; the portable project folder remains the source of truth.
6. Open `影片`, choose a source asset, then use `2A. 加入片段`, `2B. 儲存時間軸`, `3. FFprobe 探勘`, and `4. FFmpeg 匯出`.
7. In `發布`, save metadata, run `6. Dry-run 檢查`, then explicitly confirm `7. 確認並上傳 YouTube`.

YouTube upload requires a Google Cloud Desktop OAuth client and the process environment variables `YTPM_YOUTUBE_CLIENT_ID` and `YTPM_YOUTUBE_CLIENT_SECRET`. Use desktop steps `5A` and `5B` to complete PKCE; the refresh token stays in the current App process for immediate upload. A new session may additionally receive `YTPM_YOUTUBE_REFRESH_TOKEN`. The application never writes access or refresh tokens into the project folder or SQLite index.
