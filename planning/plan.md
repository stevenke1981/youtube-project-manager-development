# YTPM 開發計畫

## 目標

在 Windows 10／11 先完成可離線獨立運行的桌面 App，讓使用者以「一支影片一個專案資料夾」管理完整 YouTube 製作流程。之後再延伸 Linux、macOS、AI Adapter、FFmpeg 與 YouTube API。

## 分期

### Phase 0 — 工程基線

- Rust workspace、React/Tauri、格式化、測試、CI。
- 建立 schema、folder contract、error taxonomy。
- 驗收：空專案可 build；CI 可執行；文件與程式結構一致。

### Phase 1 — Project Core（MVP）

- 設定專案根目錄。
- 建立、列出、開啟、驗證、封存專案。
- 自動建立標準資料夾與模板。
- 驗收：中文標題、長路徑、同名專案、無權限路徑均有測試。

### Phase 2 — Asset Catalog

- 掃描圖片、音訊、影片、字幕、封面。
- 取得副檔名、大小、建立時間、媒體長度、尺寸、hash。
- 標記採用、淘汰、版本、來源、提示詞與授權。
- 驗收：外部移動／改名後可重新連結，不破壞素材。

### Phase 3 — Workflow & Tasks

- Kanban、checklist、截止日期、製作階段與進度計算。
- 自動缺漏檢查：腳本、語音、字幕、封面、成品。
- 驗收：狀態轉移規則與完成百分比一致。

### Phase 4 — Editors

- Markdown 腳本編輯器。
- 字幕表格、時間重疊檢查。
- 發布資料、章節、置頂留言、複製功能。
- 驗收：自動儲存、版本與復原可靠。

### Phase 5 — Media Integration

- FFprobe metadata、縮圖、waveform。
- FFmpeg 音訊正規化、proxy、字幕 burn-in 預覽。
- 驗收：找不到 FFmpeg 時 App 不崩潰，提供修復指引。

### Phase 6 — AI Adapters

- Whisper.cpp、Qwen3-TTS、VoxCPM、Grok prompt export。
- Job queue、取消、重試、資源限制。
- 驗收：AI 服務不可用時核心功能仍可用；所有外部傳輸需明示。

### Phase 7 — Publishing & Analytics

- YouTube OAuth、草稿上傳、metadata、縮圖、字幕。
- 發布日曆與成效快照。
- 驗收：Token 使用 OS credential store；上傳前有最終確認。

## 每期工作節奏

1. 更新 spec／ADR。
2. 先寫核心與 contract tests。
3. 實作 CLI。
4. 實作 Tauri command。
5. 實作 UI。
6. 跑自動測試與人工 smoke test。
7. 更新 final、changelog、migration notes。

## 交付 Gate

- G1：格式與型別通過。
- G2：核心單元／整合測試通過。
- G3：資料 migration 與回復測試通過。
- G4：Windows 中文路徑 smoke test 通過。
- G5：安裝、升級、移除不影響影片專案資料。
