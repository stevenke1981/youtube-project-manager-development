# CLI 與 Tauri API Contract

## CLI

```text
ytpm create --root PATH --title TITLE [--channel NAME] [--language zh-TW] [--aspect-ratio 16:9] [--json]
ytpm list --root PATH [--json]
ytpm validate --path PROJECT_PATH [--json]
ytpm structure [--json]
ytpm archive --path PROJECT_PATH [--json]
ytpm restore --path ARCHIVED_PROJECT_PATH [--json]
ytpm migrate --path PROJECT_PATH [--json]
ytpm index rebuild --root LIBRARY_ROOT [--json]
ytpm index search --root LIBRARY_ROOT [--query TEXT] [--status STATUS] [--json]
ytpm task list|create|update|move --path PROJECT_PATH ... [--json]
ytpm asset scan|list --path PROJECT_PATH [--json]
ytpm document read|write --path PROJECT_PATH --relative-path RELATIVE_PATH ... [--json]
ytpm journal recover --root LIBRARY_ROOT [--json]
```

Exit codes：

- 0 success
- 2 invalid arguments
- 10 filesystem error
- 11 invalid project
- 12 validation issues found
- 20 unexpected internal error

`--json` stdout 不得混入 log。log 寫 stderr。

## Tauri Commands

### project_create

Request：`rootPath` 與巢狀 `request` DTO（`title`、`channel?`、`language`、`aspectRatio` 等）。

Response：Project DTO。

### project_list

Request：`rootPath`。Response：Project[]。掃描大量 project 時改 job API。

### project_validate

Request：`projectPath`。Response：ValidationReport。

### project_update_status

Request：`projectPath`、`status`。只允許工作流程狀態；`archived` 必須使用 `project_archive`。
成功後以 atomic write 更新 `project.json` 的 `status`、`progress` 下限與 `updated_at`。

### project_list 行為

預設只列出 active Library 內的專案；`_archive` 內的封存專案不會混入 active list，避免桌面端把封存路徑錯當成 Library 直接子資料夾。

### project_archive

Request：`projectPath`。將專案完整資料夾移至同一 Library root 的 `_archive`，先建立 operation journal；目的地存在或來源是 symlink/junction 時拒絕操作。

### project_restore

Request：`projectPath`，必須指向 `_archive` 的直接子資料夾。還原不覆寫既有專案，成功後恢復 `archived_from_status`（舊版沒有此欄位時回到 `idea`）。

### project_migrate

Request：`projectPath`。將 v1 `project.json` 備份至專案內 `.ytpm-backup/` 後，以 temp + rename 寫入目前 schema version。

### project_index_rebuild / project_index_search

以 `project.json` 重建或查詢 Library 的 SQLite derived index。資料庫位於
`LIBRARY_ROOT/.ytpm/index.sqlite3`，可安全刪除後重新建立；搜尋前會重建以反映外部檔案變更。

### task_list / task_create / task_update / task_move

以專案內 `tasks.json` 為真實來源，支援五欄 Kanban 狀態、順序、完成時間與 atomic write。
`task_update` 的 nullable 欄位可明確清除為 `null`。

### asset_scan / asset_list

掃描專案相對路徑並更新 `assets.json`；忽略 metadata／`.ytpm`／`.ytpm-backup`，計算 SHA-256，保留 missing record，且拒絕 symlink/junction/reparse path。

### document_read / document_write

只允許腳本、發布 metadata 與字幕白名單路徑；寫入使用 temp file + rename，桌面編輯器以 800ms debounce autosave 並保留失敗草稿。

### project_recover_journal

檢查 Library root 的 `.ytpm-operation.json`。只有 `prepared` 或可確認 `moved` 狀態才會自動清理；目的地已有專案 metadata 時會補齊 archive/restore 狀態，歧義狀態保留 journal 並回報錯誤。

## Future Job API

- `job_start(kind, payload) -> job_id`
- `job_cancel(job_id)`
- events：queued/running/progress/succeeded/failed/canceled。
