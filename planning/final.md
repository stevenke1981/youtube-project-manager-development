# Final Delivery Record

## 目前交付

此版本是 **v0.1 可使用的離線桌面 MVP**；尚不是包含完整 NLE 時間軸、FFprobe、YouTube 自動上傳與簽章流程的商用完成版。

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

## 驗證狀態

- [x] ZIP 結構與檔案雜湊已產生。
- [x] JSON 檔案可被解析。
- [x] SQL migration 與 JSON Schema 已人工結構檢查。
- [x] `cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`。
- [x] `npm install`、`npm run typecheck`、`npm run test`、`npm run build`。
- [x] CLI 中文 Windows path smoke：create/list/validate。
- [x] `npm run desktop:build`：產生 MSI、NSIS 與 release executable。
- [x] core 56 tests（含 index、Task、Asset、Document、journal、junction validation）、React 15 tests、workspace clippy/test、中文／空白路徑 smoke。
- [x] `scripts/smoke-junction.ps1`：中文 Windows 路徑與 junction validation 回傳非零且包含 `REQUIRED_DIRECTORY_SYMLINK`。

## 下一個開發者第一步

1. 加入完整 Tauri invoke contract/e2e tests 與 persistent error center。
2. 對 Asset Catalog 增加 incremental scan、FFprobe adapter、thumbnail/preview 與 import/relink。
3. 加入 subtitle parser、metadata checklist、versions/history，以及真正的非破壞性 NLE timeline（仍遵守本版 boundary）。

## 回滾

本次測試只使用 `%TEMP%` fixture，未修改使用者既有 Library。migration 會在專案內建立 `.ytpm-backup/`；archive/restore 會寫 operation journal，rollback 失敗時保留 journal 供人工恢復。SQLite index 可安全刪除後重建；`project.json`、`tasks.json`、`assets.json` 與實際文件仍是可攜式 source of truth。
