# AGENTS.md — YouTube Project Manager

本文件是 Codex、OpenCode、Claude Code 或其他開發代理的最高層專案規則。

## 1. 任務目標

建立可離線獨立運行的 YouTube Project Manager。每支影片必須是可移植的獨立資料夾，管理研究、腳本、語音、圖片、影片、字幕、封面、發布資料、任務與版本。

## 2. 不可破壞的約束

- `project.json` 是可攜式專案的真實來源。
- SQLite 是索引，不可成為唯一資料來源。
- GUI 與 CLI 必須共用 `ytpm-core`。
- 不得把專案素材複製進不透明的私有資料庫 Blob。
- 不得在未確認的情況永久刪除使用者檔案。
- 路徑操作必須防止 `..`、絕對路徑注入、Windows 禁用名稱與非法字元。
- 寫入重要 JSON 必須使用 temp file + rename，避免半寫入。
- Schema 變更必須增加 `schema_version` 與 migration。
- 新功能必須具有可驗收條件與測試。

## 3. 預設權限規則

可以直接執行：讀檔、搜尋、格式化、build、test、lint、建立新檔、修改專案內程式碼。

必須先取得使用者確認：

- `rm`、`rmdir`、`del`、`Remove-Item -Recurse` 等不可逆刪除。
- `git push --force`、覆寫遠端歷史。
- 修改專案目錄以外的系統設定。
- 上傳任何使用者素材、Token 或個人資料到外部服務。

測試產生的暫存資料只能放在系統 temp 或 `target/`、`dist/`、`.tmp/`。

## 4. Controlled Workflow

### Gate 0：理解

- 讀取 `planning/spec.md`、`planning/todos.md`、相關 docs。
- 說明要改的範圍、輸入輸出、風險。

### Gate 1：設計

- 確認是否影響 schema、資料夾格式、CLI contract、Tauri command。
- 若影響，先更新文件與 migration。

### Gate 2：實作

- 優先修改 `ytpm-core`。
- CLI 與 GUI 只做薄介面。
- 不要複製核心商業規則到 TypeScript。

### Gate 3：驗證

至少執行：

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run typecheck
npm run test
npm run build
```

涉及 UI 時，補 Playwright 或可重現的人工驗證紀錄。

### Gate 4：交付

- 更新 `planning/todos.md`。
- 在 `planning/final.md` 記錄變更、測試、限制、回滾。
- 不得聲稱未執行的測試已通過。

## 5. Fail Class

- **F1 Build**：編譯、型別、依賴問題。
- **F2 Logic**：功能行為不符規格。
- **F3 Data**：資料遺失、schema、migration、路徑問題。
- **F4 UX**：不可操作、無回饋、鍵盤或縮放問題。
- **F5 Security**：路徑穿越、命令注入、Token 洩漏。
- **F6 Environment**：缺少 SDK、FFmpeg、WebView2、權限。

發生 F3 或 F5 時停止擴充功能，先修復並新增回歸測試。

## 6. Boundary

- 第一版不內建完整 NLE 時間軸剪輯器。
- 第一版不自動上傳 YouTube。
- 第一版不自動連接付費 AI API。
- AI 功能必須以 Adapter 加入，不得耦合核心資料模型。
- FFmpeg 呼叫需使用參數陣列，不得拼接 shell command string。

## 7. Rollback

- 檔案格式變更前先建立 `.ytpm-backup/` 快照。
- migration 要可重跑或具有明確回復說明。
- 寫入失敗不得破壞原檔。
- 跨檔案操作以 operation journal 記錄未完成步驟。

## 8. 完成定義

一項工作只有在以下條件都成立時才可標記完成：

- 需求與邊界清楚。
- 程式碼、文件、測試同步。
- 錯誤訊息可理解且可行動。
- Windows 路徑與中文檔名已測。
- 未留下未記錄的 TODO 或 mock 成品。
