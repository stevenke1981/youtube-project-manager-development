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
- [x] SQLite project index、FTS search 與 rebuild（SQLite 僅為可刪除的 derived cache）。
- [x] Windows reserved names tests（sanitize unit coverage）。
- [x] symlink／junction policy（archive/restore、asset、document 與 validation fail-closed；含 Windows junction smoke test）。

## M2 — Desktop UX

- [x] App shell 與 sidebar。
- [x] Dashboard summary cards。
- [x] Project cards。
- [x] Create project dialog。
- [x] Project workspace shell。
- [x] 實際 folder picker。
- [x] 編號化 StepGuide、`Next step:` 提示與工作區狀態更新操作。
- [-] toast／persistent error center（目前先有 global error banner）。
- [-] table view、filter、sort（目前已完成搜尋、狀態篩選與排序；table view 尚未加入）。
- [ ] light mode 與 system mode。
- [ ] keyboard shortcuts／command palette。

## M3 — Asset Catalog

- [x] Asset domain model 與 portable `assets.json` catalog。
- [x] file scanner、ignore rules、SHA-256 hash、missing record 與 atomic write。
- [x] FFprobe adapter：argv-only `ffprobe` JSON parser 與 Windows 路徑測試。
- [ ] thumbnail cache。
- [ ] import／link／relink。
- [ ] asset grid、preview、metadata panel。
- [ ] archive instead of delete。

## M4 — Workflow

- [x] Project status transition command/UI（status 更新以 project.json 為真實來源）。
- [x] Task CRUD（保留 JSON source of truth、支援 null 清除與 atomic write）。
- [x] Kanban transitions。
- [ ] progress calculation rules。
- [ ] missing deliverables validator。
- [x] publishing checklist：metadata readiness、dry-run、explicit confirmation。

## M5 — Editors

- [x] Markdown script／publish editor。
- [x] 800ms autosave、failed-save draft 保留與重啟草稿恢復。
- [ ] subtitle parser／overlap validation。
- [x] metadata editor／clipboard actions：發布 metadata UI 與 atomic `publish.json`。
- [ ] versions／history。

## M6 — Release

- [x] Windows MSI／NSIS build。
- [x] artifact/release smoke test：MSI、NSIS、version、SHA-256。
- [ ] clean install test（需要在隔離測試機執行 installer，不改動目前使用者安裝）。
- [ ] upgrade test。
- [ ] uninstall-preserves-library test。
- [ ] signed build strategy。
- [ ] SBOM／third-party notices。
