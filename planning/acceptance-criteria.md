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
