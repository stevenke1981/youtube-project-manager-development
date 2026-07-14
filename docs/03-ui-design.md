# UI／UX 完整設計

## 1. Design Principles

1. **Next action first**：每個頁面先回答「現在該做什麼」。
2. **Files remain visible**：永遠可開啟資料夾與複製路徑。
3. **Safe by default**：封存、undo、preview、operation summary。
4. **State is explicit**：Loading／empty／error／offline／missing 都要有畫面。
5. **AI is an assistant**：AI 產出需顯示來源、模型、提示詞、狀態，不覆寫人工內容。

## 2. Layout

- Sidebar：240px，收合後 72px。
- Topbar：64px。
- Content max width：1600px；資料表可滿寬。
- Inspector：右側 360px，可收合。
- 最小視窗：1100×700；小於時 sidebar 預設收合。

## 3. Design Tokens

```css
--space-1: 4px;
--space-2: 8px;
--space-3: 12px;
--space-4: 16px;
--space-6: 24px;
--space-8: 32px;
--radius-control: 8px;
--radius-card: 12px;
--font-ui: "Inter", "Noto Sans TC", system-ui, sans-serif;
--font-mono: "Cascadia Code", "Noto Sans Mono", monospace;
```

語意顏色必須有明暗模式對應：surface、border、text、muted、accent、success、warning、danger、info。狀態不可只靠顏色，需搭配 label／icon。

## 4. Dashboard

### 區塊

- Greeting＋日期。
- KPI：進行中、本週發布、待審核、缺漏。
- Continue working：最近修改且未完成。
- Upcoming：七天發布日曆。
- Attention：逾期、缺檔、索引錯誤。

### Project Card

```text
[Thumbnail 16:9]
狀態 badge                      ⋮
影片標題（最多兩行）
頻道 · 預計發布日
████████░░ 80%
下一步：修正第 3 段字幕
[開啟專案] [開啟資料夾]
```

## 5. Project Workspace

### Overview

- Header：title、status、progress、publish date、more menu。
- Next action banner。
- Deliverables matrix。
- Recent assets。
- Activity timeline。

### Asset Page

Toolbar：分類、搜尋、sort、grid/table、import、scan。

Grid item：preview、filename、duration/dimensions、version、adopted、missing、source badge。

Inspector：metadata、prompt、license、related script scene、versions、open path、archive。

### Script Editor

三欄可切換：Outline／Editor／Scene Inspector。支援 Markdown、字數、預估時長、角色、畫面、音效、prompt。Autosave 狀態固定顯示。

### Subtitle Page

- 左：影片 preview。
- 中：時間軸字幕列。
- 右：validation issues。
- 快捷鍵：Space 播放、Ctrl+Enter 套用、Alt+↑↓ 移動列。

### Thumbnail Page

- 16:9 大圖比較。
- 320px／160px 縮圖可讀性預覽。
- A/B 版本、標題、prompt、採用狀態。

### Publish Page

- Title character counter。
- Description、hashtags、tags、chapters、pinned comment。
- Deliverable checklist。
- Copy buttons。
- 未來 YouTube upload 是明確的 Review → Confirm 流程。

## 6. Create Project Wizard

- 3 steps，不超過 8 個必要輸入。
- 即時顯示資料夾 preview。
- 標題非法字元由系統處理，不要求使用者理解 Windows 規則。
- Advanced settings 預設收合。

## 7. Empty／Loading／Error

- Empty：說明為何為空＋單一主要 CTA。
- Loading：shell 先出現；卡片 skeleton；超過 2 秒顯示文字。
- Error：摘要、影響、建議動作、詳情、copy diagnostics。
- Partial：可用內容照常顯示，問題以 banner 標示。

## 8. Accessibility

- 完整鍵盤導覽。
- Focus ring 2px，不能移除。
- Dialog focus trap／restore。
- icon-only button 有 accessible name。
- 支援 reduced motion。
- 文字／背景 WCAG AA 對比。

## 9. Microcopy

不要：「操作失敗」。

使用：「無法在 `D:\YouTube-Projects` 建立資料夾。請確認磁碟未滿，且目前帳號具有寫入權限。」

危險動作按鈕明確寫結果，例如「封存 3 個素材」，不寫泛用「確定」。
