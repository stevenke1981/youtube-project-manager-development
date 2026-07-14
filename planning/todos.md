# Implementation TODOs

狀態：`[ ]` 未開始、`[-]` 進行中、`[x]` 完成、`[!]` 阻塞。

## M0 — Baseline

- [x] 建立 Rust workspace。
- [x] 建立 React／Tauri desktop skeleton。
- [x] 建立初始 JSON Schema 與 SQLite migration。
- [x] 建立 CI 與 PowerShell scripts。
- [x] 在 Windows 安裝依賴並產生 `package-lock.json`。
- [x] 執行第一次完整 build，修正實際版本相容性。

## M1 — Project Core

- [x] 定義 Project 與 CreateProjectRequest。
- [x] 實作安全資料夾命名。
- [x] 實作標準目錄建立。
- [x] 實作原子寫入 project.json。
- [x] 實作 list projects。
- [x] 實作 validate project。
- [x] CLI create/list/validate/structure。
- [x] 實作 archive／restore（含 operation journal、衝突不覆寫、symlink/junction 拒絕、封存前狀態恢復）。
- [x] schema v1→v2 migration（`.ytpm-backup/` snapshot、`ytpm migrate`）。
- [x] Library root 設定持久化（本機 webview storage）與 folder picker。
- [ ] SQLite project index 與 rebuild。
- [x] Windows reserved names tests（sanitize unit coverage）。
- [-] symlink／junction policy（archive/restore 拒絕 symlink；junction 需 Windows smoke test）。

## M2 — Desktop UX

- [x] App shell 與 sidebar。
- [x] Dashboard summary cards。
- [x] Project cards。
- [x] Create project dialog。
- [x] Project workspace shell。
- [x] 實際 folder picker。
- [-] toast／persistent error center（目前先有 global error banner）。
- [ ] table view、filter、sort。
- [ ] light mode 與 system mode。
- [ ] keyboard shortcuts／command palette。

## M3 — Asset Catalog

- [ ] Asset domain model。
- [ ] file scanner 與 ignore rules。
- [ ] FFprobe adapter。
- [ ] thumbnail cache。
- [ ] import／link／relink。
- [ ] asset grid、preview、metadata panel。
- [ ] archive instead of delete。

## M4 — Workflow

- [ ] Task CRUD。
- [ ] Kanban transitions。
- [ ] progress calculation rules。
- [ ] missing deliverables validator。
- [ ] publishing checklist。

## M5 — Editors

- [ ] Markdown script editor。
- [ ] autosave draft and recovery。
- [ ] subtitle parser／overlap validation。
- [ ] metadata editor／clipboard actions。
- [ ] versions／history。

## M6 — Release

- [x] Windows MSI／NSIS build。
- [ ] clean install test。
- [ ] upgrade test。
- [ ] uninstall-preserves-library test。
- [ ] signed build strategy。
- [ ] SBOM／third-party notices。
