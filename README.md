# YouTube Project Manager（YTPM）開發包

> 離線優先、資料夾為核心、AI Agent 友善的 YouTube 影片專案管理桌面應用程式。

本開發包不是單純的企劃文件；它同時提供：

- 完整產品需求與使用流程設計。
- UI／UX 規格、頁面線框與設計 Token。
- Rust 共用核心、CLI、Tauri 2＋React 桌面端可使用 v0.2 media workstation。
- 可實際建立、列出、驗證、索引影片專案，並管理 Task/Kanban、Asset Catalog 與文件編輯。
- 可封存／還原專案，並保留封存前製作階段。
- JSON Schema、SQLite migration、專案資料夾範本。
- Codex／OpenCode 可直接遵循的 `AGENTS.md`、Gate、驗收與回滾規則。
- Windows PowerShell 開發、測試、建置腳本。
- CI、測試計畫、風險清單、里程碑與交付標準。

## 產品核心原則

1. **一支影片一個獨立資料夾**，可單獨搬移、備份或封存。
2. `project.json` 是專案真實來源；SQLite 只負責快速索引，可隨時重建。
3. GUI 與 CLI 共用 `ytpm-core`，避免規則分裂。
4. 預設不永久刪除素材；改移至 `10_archive/`。
5. `timeline.json`、FFprobe／FFmpeg 與 YouTube 發布 adapter 都保留可攜式檔案 source of truth。
6. 所有資料可由一般檔案總管、文字編輯器與第三方工具讀取。

## 技術組合

- Desktop：Tauri 2
- Core／CLI：Rust
- Frontend：React 19＋TypeScript＋Vite
- Local index：SQLite（可刪除重建的 derived cache；`project.json` 仍是 source of truth）
- Media：FFmpeg／FFprobe（argv-only、非破壞式 trim/export）
- Tests：Rust tests、Vitest、Playwright、PowerShell smoke tests

## 目錄

```text
.
├── AGENTS.md
├── planning/                 # 開發工作包與驗收
├── docs/                     # 完整產品、流程、UI、架構設計
├── schemas/                  # project/tasks/assets JSON Schema
├── migrations/               # SQLite schema
├── templates/                # 新影片專案範本
├── crates/
│   ├── ytpm-core/            # 共用領域核心
│   └── ytpm-cli/             # AI Agent／命令列介面
├── apps/desktop/             # Tauri 2 + React 桌面 App
├── scripts/                  # Windows 開發腳本
├── samples/                  # 範例專案
└── .github/workflows/ci.yml
```

## 快速開始（Windows 10／11）

先安裝 Node.js、Rust、Visual Studio Build Tools（Desktop development with C++）及 Tauri 的 Windows prerequisites。

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\bootstrap.ps1
.\scripts\dev.ps1
```

只測試 Rust 核心與 CLI：

```powershell
cargo test --workspace
cargo run -p ytpm-cli -- structure
cargo run -p ytpm-cli -- create --root D:\YouTube-Projects --title "第一支影片" --channel "我的頻道"
```

桌面端：

```powershell
npm install
npm run desktop:dev
```

## MVP 已實作能力

- 建立影片專案與標準資料夾。
- 寫入 `project.json`、腳本／研究／發布資料範本。
- 掃描專案根目錄並列出專案。
- 驗證 `project.json` 與必要資料夾。
- SQLite：`index rebuild/search`、FTS 查詢與外部變更反映。
- Task/Kanban：`tasks.json` atomic CRUD、五欄狀態、順序與完成時間。
- Asset Catalog：掃描、kind、size、SHA-256、missing 與安全路徑檢查。
- 真正文件編輯器：腳本／發布文件讀寫、800ms autosave、失敗草稿恢復。
- CLI：`create`、`list`、`validate`、`structure`、`archive`、`restore`、`migrate`、`index`、`task`、`asset`、`document`、`journal`。
- Desktop：儀表板、專案卡片、新增專案視窗、專案工作區與編號化 Step/Next step 導引。
- Desktop：folder picker 選擇 Library root，設定保存在本機 webview storage。
- Desktop：影片工作台以 1–7 步驟完成素材選擇、timeline trim、FFprobe、FFmpeg、metadata dry-run 與 YouTube 確認上傳。
- CLI：`timeline`、`media probe/export`、`publish config/dry-run/upload --confirm`。
- Release：Tauri v0.2.0 MSI／NSIS、`scripts/release-checks.ps1`、`scripts/installer-smoke.ps1`。

## 建議開發順序

請從 `planning/plan.md`、`planning/spec.md`、`planning/todos.md` 開始。完成每一 Gate 後更新 `planning/final.md`，不要跳過測試與資料遷移驗證。

## 官方技術參考

- Tauri 2：`https://v2.tauri.app/`
- React：`https://react.dev/`
- Vite：`https://vite.dev/guide/`
- SQLx：`https://docs.rs/sqlx/latest/sqlx/`

## 授權

此開發包採 MIT License。實際產品使用第三方套件、字型、圖示、影音素材與 YouTube API 時，仍需分別確認授權與服務條款。
