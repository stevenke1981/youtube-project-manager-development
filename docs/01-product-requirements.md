# 詳細產品需求

## Persona A：AI 影片創作者

同時使用 ChatGPT、Grok Imagine、TTS、Whisper、CapCut／DaVinci Resolve。需要保留每次生成提示詞與版本。

## Persona B：多頻道管理者

需要依頻道、系列、發布日期、狀態篩選，避免錯用封面、字幕與描述。

## Persona C：Agent-first 開發者

希望 Codex／OpenCode 可透過結構化 JSON 與 CLI 自動建立工作、驗證缺漏，但禁止危險刪除。

## Use Cases

### UC-01 從模板建立影片

- 選擇「AI 科普 8 分鐘」。
- 自動建立 8 個腳本段落、16:9、zh-TW、必要 deliverables。
- 建立預估字數與任務。

### UC-02 匯入現有資料夾

- 使用者選擇沒有 `project.json` 的舊影片目錄。
- App 預覽偵測結果，不直接移動檔案。
- 使用者確認分類後產生 metadata。

### UC-03 發布前檢查

- 檢查 final video、thumbnail、title、description、subtitle。
- 檢查字幕重疊、影片時長、封面比例、缺少來源註記。
- 產生 report，可標記例外理由。

## Requirement Priority

- P0：建立／掃描／驗證／安全寫入。
- P1：素材 catalog、任務、腳本、字幕、發布資料。
- P2：FFmpeg、AI adapter、日曆。
- P3：YouTube API、分析、團隊協作。
