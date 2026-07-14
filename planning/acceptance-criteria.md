# Acceptance Criteria

## AC-01 建立專案

Given 有效 Library root，When 輸入中文標題建立專案，Then 3 秒內完成、資料夾唯一、`project.json` 可解析、必要目錄存在。

## AC-02 同名專案

Given 同日已有同名專案，When 再建立，Then 使用 `-02`、`-03` 後綴，不覆寫舊專案。

## AC-03 非法字元

Given 標題含 Windows 非法字元，When 建立，Then 自動替換，不產生 root 外路徑，UI 顯示實際資料夾名稱。

## AC-04 搬移

Given 專案被搬到另一個 Library，When 掃描，Then 可由 `project.json` 識別；舊 root_path 只產生警告。

## AC-05 驗證

Given 缺少字幕資料夾，When 驗證，Then 回傳 machine-readable issue code、路徑、`error` severity 與修復建議，且 `valid=false`。

## AC-06 App 移除

Given 使用者解除安裝 App，Then Library 內的所有影片專案與素材保持不變。

## AC-07 Agent

Given Agent 執行 `ytpm list --json`，Then stdout 只包含有效 JSON，診斷訊息輸出 stderr，exit code 符合規格。

## AC-08 編號化操作流程

Given 使用者首次開啟 App，When 尚未選擇 Library、建立專案或進入工作區，Then 畫面分別顯示 `Step 1`、`Step 2`、`Step 3` 與 `Next step:`，且下一步按鈕文字直接指出可執行操作。

## AC-09 SQLite derived index

Given Library 內有有效與損壞的 `project.json`，When 執行 `index rebuild` 或在桌面搜尋，Then 有效專案可查詢、損壞檔案只計入 `invalid`，且刪除 `.ytpm/index.sqlite3` 後可重建，不修改任何 `project.json`。

## AC-10 Task/Kanban

Given 專案已建立，When 使用者新增、編輯、移動任務，Then `tasks.json` 以 atomic write 更新，五個 Kanban 欄位可顯示，完成狀態有 `completed_at`，nullable 欄位可清除為 `null`。

## AC-11 Asset Catalog

Given 專案含中文檔名、遺失記錄與 metadata，When 執行素材掃描，Then `assets.json` 保留可攜式相對路徑、SHA-256、size、state，忽略 `.ytpm`／metadata 並保留 missing record。

## AC-12 真正文件編輯器

Given 使用者在腳本或發布分頁編輯，When 停止輸入 800ms，Then 內容以原子寫入實際專案文件，UI 顯示 Saving/Saved/Error；重載時可恢復未完成的本機草稿。

## AC-13 Journal 與 junction 安全

Given archive/restore 在 `prepared` 或 `moved` 階段中斷，When 啟動 Library 掃描或執行 journal recover，Then 可安全清理已確認狀態並修復 metadata，歧義狀態保留 journal；Windows junction 指向 Library 外時 validation 回傳 `REQUIRED_DIRECTORY_SYMLINK` 且不讀寫 target。
