# Product Specification

## 1. 問題

YouTube 製作者常將腳本、TTS、圖片、分鏡、影片、字幕、封面與上架文案分散在不同目錄和工具。常見問題是檔名混亂、版本不明、漏素材、無法快速接續工作，以及 AI Agent 不知道正確檔案與流程。

## 2. 使用者

- 單人 YouTube 創作者。
- 使用生成式 AI 製作圖片、語音、影片的創作者。
- 同時管理多個頻道或系列的人。
- 使用 Codex／OpenCode 自動化製作的人。

## 3. 核心情境

1. 建立新影片專案，自動產生完整資料夾。
2. 從儀表板看到每支影片目前階段與缺漏。
3. 管理腳本、語音、圖片、影片、字幕、封面版本。
4. 將任務從待處理移至完成。
5. 開啟外部工具處理檔案，返回 App 後自動更新。
6. 匯出或搬移單一專案，另一台電腦可重新索引。
7. 讓 AI Agent 透過 CLI 讀取、建立、驗證與更新專案。

## 4. MVP 功能需求

### FR-001 專案根目錄

- 可設定一個或多個 Library root。
- root 不可位於 App 安裝目錄。
- App 設定遺失後可重新指定 root 並掃描。

### FR-002 建立專案

必要欄位：標題。選填：頻道、比例、語言、預計長度、發布日期、模板、標籤。

建立時必須：

- 產生安全、可讀、唯一的資料夾名稱。
- 建立標準目錄與範本。
- 原子寫入 `project.json`。
- 回傳實際路徑與警告。

### FR-003 專案列表

- 卡片／表格模式。
- 搜尋標題、頻道、標籤。
- 依狀態、日期、進度、最近修改排序。
- 顯示缺漏與逾期。

### FR-004 專案工作區

分頁：總覽、任務、研究、腳本、語音、圖片、影片、字幕、封面、發布、歷史、設定。

### FR-005 驗證

- JSON 格式與 schema_version。
- 必要資料夾。
- root_path 一致性只作警告，允許專案搬移。
- 同一 asset id 不得重複。
- 檔案不存在時標為 missing，不自動刪除紀錄。

### FR-006 封存

- 專案封存只改狀態與移入 Library 的 `_archive/`（可設定）。
- 永久刪除是獨立高風險操作，MVP 不提供。

### FR-007 自動儲存

- 文字編輯 800ms debounce。
- 顯示「儲存中／已儲存／儲存失敗」。
- 失敗時保留本地 draft，不可靜默遺失。

### FR-008 CLI

- `create`、`list`、`validate`、`structure`。
- 預設人類可讀輸出；提供 `--json` 給 Agent。
- 非零 exit code 表示失敗或 validation error。

## 5. 非功能需求

- NFR-001：10,000 個素材的專案仍可操作；掃描需可取消。
- NFR-002：App 冷啟動目標 2 秒內顯示 shell，背景載入索引。
- NFR-003：所有核心功能離線可用。
- NFR-004：Windows 中文、空白、emoji 路徑可用。
- NFR-005：不執行來源不明的腳本或媒體內容。
- NFR-006：UI 125%／150% 縮放仍可操作。
- NFR-007：資料寫入可恢復、可追蹤、可重建。

## 5.1 v0.2 媒體製作與發布擴充

### FR-009 Portable NLE timeline

- 每個專案以 `timeline.json` 保存 tracks、clips、trim、音量、mute、transition 與輸出設定。
- Timeline 只引用專案相對路徑與 asset id，不複製媒體到私有資料庫。
- Clip 的 start/in/out/duration 必須是非負整數毫秒；非法 trim、重疊與不存在 asset 必須回傳可行動錯誤。
- UI 支援新增、移動、trim、排序、移除 clip；移除 clip 不刪除素材。

### FR-010 FFprobe／FFmpeg

- 以參數陣列呼叫使用者設定的 `ffprobe.exe`／`ffmpeg.exe`，禁止 shell command string。
- 顯示 format、duration、video/audio streams、解析度、fps、codec 與可理解的工具缺失錯誤。
- 提供 preview/proxy/final export，工作可取消、顯示進度，暫存與輸出檔案不得覆寫既有成品。
- FFmpeg 不可用時 App 仍可開啟與編輯專案，提供安裝路徑與修復指引。

### FR-011 YouTube publishing

- OAuth 使用 state + PKCE loopback callback；client id/secret 只來自本機設定或 OS credential reference。
- 上傳前必須由使用者明確確認；預設 dry-run，支援 metadata、thumbnail、subtitle、privacy、schedule。
- Upload job 支援進度、取消、可重試錯誤與不重試的授權／驗證錯誤；Token 不寫入 repo、Library 或 `project.json`。

### FR-012 Windows installer

- MSI/NSIS 安裝 App，不建立、搬移或刪除 Library。
- Release check 驗證 executable、MSI、NSIS、checksum 與 clean-install；uninstall smoke 必須確認 Library fixture 保留。

## 6. v0.1 暫不包含（v0.2 已納入實作範圍）

- 完整影片時間軸剪輯（v0.2 timeline 以 non-destructive cut/trim/export 為範圍）。
- 雲端協作與多人即時編輯。
- 未確認的自動上傳／發布（v0.2 僅允許 OAuth、明確確認後的 upload job）。
- 將所有素材複製到 App 私有儲存。

## 7. 成功指標

- 建立專案小於 3 秒。
- 使用者 30 秒內可找到任一腳本／封面／成品。
- 95% 常見操作不需離開專案工作區。
- 搬移專案後可在 1 次掃描內恢復。
- 重大資料遺失事件為 0。
