# Final Delivery Record

## 目前交付

此版本是 **v0.1 開發內容與可擴充骨架**，不是已完成的商用安裝程式。

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
- Schema、SQLite migration、範本、CI、PowerShell scripts。

## 驗證狀態

- [x] ZIP 結構與檔案雜湊已產生。
- [x] JSON 檔案可被解析。
- [x] SQL migration 與 JSON Schema 已人工結構檢查。
- [x] `cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`。
- [x] `npm install`、`npm run typecheck`、`npm run test`、`npm run build`。
- [x] CLI 中文 Windows path smoke：create/list/validate。
- [x] `npm run desktop:build`：產生 MSI、NSIS 與 release executable。
- [x] core 12 integration tests、React 6 tests、workspace clippy/test、中文／空白路徑 smoke。

## 下一個開發者第一步

1. 完成 Windows junction 實機 smoke 與 operation journal 啟動恢復。
2. 完成 M1 SQLite index/rebuild，再進入 Asset Catalog。
3. 加入 Tauri/CLI contract tests、persistent error center 與真正的 Task/Asset/Editor workspace。

## 回滾

本次測試只使用 `%TEMP%` fixture，未修改使用者既有 Library。migration 會在專案內建立 `.ytpm-backup/`；archive/restore 會寫 operation journal，rollback 失敗時保留 journal 供人工恢復。尚未完成啟動時自動恢復 journal 與 junction 專用 Windows fixture。
