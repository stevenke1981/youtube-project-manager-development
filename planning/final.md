# Final Delivery Record

## 目前交付

此版本為 **可使用的離線桌面 NLE production workstation**；已加入 timeline schema v2、typed professional effects、多軌 FFmpeg filter graph、字幕燒錄、背景匯出 queue，以及明確確認的 YouTube resumable upload adapter。它不宣稱逐項複製 Adobe Premiere Pro 的專有引擎，也不長期代管使用者秘密。

已包含：

- 完整產品規格、使用流程、UI／UX、架構、資料模型與安全設計。
- Rust 共用核心與 CLI 原始碼。
- Tauri 2＋React 桌面端 UI 骨架。
- 建立、列出、驗證影片專案功能。
- 封存／還原影片專案：完整資料夾移入 `_archive`，以 operation journal 保護跨檔案移動，衝突不覆寫。
- archive／restore 拒絕包含 `..` 的路徑與祖先 symlink/junction/reparse point，並保留封存前狀態。
- schema v1→v2 migration：先建立 `.ytpm-backup/` snapshot，再以 temp + rename 寫回；CLI 提供 `migrate`。
- Tauri command 回傳結構化錯誤 DTO；CLI 實作 filesystem/project/argument exit code。
- Desktop Library root 使用 folder picker 與本機 storage，不再寫死開發者磁碟路徑。
- Desktop workflow 已可操作：Step 1/2/3 導引、搜尋／狀態篩選／排序、validation 面板、status 更新與確認後封存。
- 缺少必要標準資料夾時 validation 會明確失敗；active list 不會混入 `_archive` 專案。
- React 對 Tauri structured error 會顯示人類訊息與建議動作，不會顯示 `[object Object]`。
- SQLite project index／FTS search：位於 `.ytpm/index.sqlite3` 的可重建 derived cache，搜尋前會反映外部檔案變更，不作為 source of truth。
- Task/Kanban：`tasks.json` 為真實來源，支援新增、編輯、移動、完成時間、nullable 欄位清除與 atomic write。
- Asset Catalog：`assets.json` 為真實來源，支援掃描、kind、size、SHA-256、missing 保留、metadata ignore 與安全路徑檢查。
- 真正文件編輯器：腳本與發布文件直接讀寫專案檔案，800ms autosave、Saved/Error 狀態、失敗草稿與重載恢復。
- journal 自動恢復：啟動列舉前檢查 `.ytpm-operation.json`，可修補已搬移專案的 archive/restore metadata；歧義狀態保留 journal。
- Windows junction 專用驗收：中文路徑、`mklink /J`、required directory validation smoke，所有核心寫入路徑 fail-closed。
- Schema、SQLite migration、範本、CI、PowerShell scripts 與 CLI/Tauri contract。
- Timeline：`timeline.json` 保存 UUID tracks/clips、毫秒 trim、音量／靜音／transition 與輸出設定；支援新增、移動、更新、移除與 render manifest。
- FFprobe／FFmpeg：argv-only external process、JSON metadata、暫存 segment/concat export、拒絕覆寫既有輸出、cancel marker 與實際 Windows FFprobe fixture test。
- YouTube：metadata atomic save、readiness checklist、PKCE/state OAuth URL、refresh-token exchange、resumable `videos.insert`、dry-run 與 `confirm=true` gate。
- Desktop media workstation：影片／發布分頁具備 1–7 編號操作、timeline trim、probe、export、5A/5B PKCE callback、dry-run、確認上傳與錯誤／成功狀態。
- Windows release v0.2.0：MSI、NSIS、`release-checks.ps1` 與只讀 `installer-smoke.ps1`，產生 SHA-256 checksum。
- Timeline v2：typed color/blur/sharpen/vignette/chroma/fade/transform、subtitle style、v1 exact-byte backup 與 atomic migration。
- FFmpeg filter graph：單次 argv-only process 完成多軌 video overlay、audio delay/mix、transition、effects 與 ASS subtitle burn-in；不接受 raw filter text。
- Background export：process-local single worker、五種 job state、progress pipe、running child termination、partial output/temp cleanup。
- Data safety review：final output 改為 job-owned temp＋atomic no-overwrite publish；timeline migration/edit 以 per-project lock 序列化，queue/project path 與 Windows reserved／ADS／reparse 規則在入列前驗證。
- Desktop NLE：1–10 編號流程、asset/track 選擇、clip inspector、typed effect controls、字幕樣式、queue 進度與取消。

## 驗證狀態

- [x] ZIP 結構與檔案雜湊已產生。
- [x] JSON 檔案可被解析。
- [x] SQL migration 與 JSON Schema 已人工結構檢查。
- [x] `cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`。
- [x] `npm install`、`npm run typecheck`、`npm run test`、`npm run build`。
- [x] CLI 中文 Windows path smoke：create/list/validate。
- [x] `npm run desktop:build`：產生 MSI、NSIS 與 release executable。
- [x] core/workspace tests（84 Rust tests；核心含 index、Task、Asset、Document、journal、timeline v2、filter graph、subtitle、job queue、FFprobe、實際 FFmpeg、publish；React 20 tests）。
- [x] `scripts/smoke-junction.ps1`：中文 Windows 路徑與 junction validation 回傳非零且包含 `REQUIRED_DIRECTORY_SYMLINK`。
- [x] `cargo test --workspace`：84 tests；包含 timeline migration/serialization、atomic publish、filter allowlist、SRT/VTT/ASS、project-scoped job cancellation、FFprobe、實際 FFmpeg render、publish metadata/OAuth PKCE dry-run。
- [x] `cargo check -p ytpm-desktop`、`npm run typecheck`、`npm test`、`npm run build`。
- [x] `scripts/release-checks.ps1 -Check All`、`scripts/installer-smoke.ps1 -InspectOnly`：MSI／NSIS 與 SHA-256 通過，未變更使用者安裝。
- [x] Release executable launch smoke：程序維持運行且 window title 為 `YouTube Project Manager`；驗證後停止測試程序。
- [x] MSI SHA-256 `870EEB3E0AEE493E6A2538B8E8649E98CBF7B25100BB18A974854722C5C9A0A8`；NSIS SHA-256 `4227BF806E27F0FFA6FDE98487D402D8D278328F1AAF0A3648FB07FE8E516FA1`。
- [x] Feature commit `a0b6e39` 已 push 至 public GitHub `main`；最終文件 commit 後再次核對遠端 SHA parity。

## 下一個開發者第一步

1. 對 Asset Catalog 增加 incremental scan、thumbnail/preview、waveform 與 import/relink。
2. 增加 hardware encoder profiles、proxy cache、versions/history 與可持久化的 render journal。
3. 在隔離 Windows 測試機執行 signed clean install、upgrade 與 uninstall-preserves-library 測試。

## 回滾

本次測試只使用 `%TEMP%` fixture，未修改使用者既有 Library。migration 會在專案內建立 `.ytpm-backup/`；archive/restore 會寫 operation journal，rollback 失敗時保留 journal 供人工恢復。SQLite index 可安全刪除後重建；`project.json`、`tasks.json`、`assets.json` 與實際文件仍是可攜式 source of truth。

仍需在隔離 Windows 測試機執行實際 install/upgrade/uninstall-preserves-library，並規劃 signed installer；本次只做 inspect-only installer smoke。完整 Adobe Premiere Pro 專有特效、多機位、GPU hardware encoder profiles 與 persistent render journal 不在此 milestone 的完成宣稱內。
