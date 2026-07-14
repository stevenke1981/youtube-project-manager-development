# UI Design Deliverable

完整 UI 規格見 `docs/03-ui-design.md`。本文件是實作代理的摘要。

## 視覺方向

- 專業內容工作台，不做社群平台風格。
- 深色與淺色皆可；MVP 預設深色。
- 內容密度中等，重要狀態以文字＋圖示，不只靠顏色。
- 卡片圓角 12px，控制項 8px，間距採 4px 基準。

## 主導覽

```text
Dashboard
Projects
Calendar
Templates
Automation
Settings
```

專案內二級導覽：

```text
Overview / Tasks / Research / Script / Voice / Images / Video /
Subtitles / Thumbnail / Publish / History / Settings
```

## Dashboard 桌面線框

```text
┌──────────────────────────────────────────────────────────────────────┐
│ ☰  YouTube Project Manager        [搜尋]        [＋新增影片] [設定] │
├──────────────┬───────────────────────────────────────────────────────┤
│ Dashboard    │  歡迎回來                                  7/14     │
│ Projects     │  [進行中 8] [本週發布 3] [待審核 2] [有缺漏 4]       │
│ Calendar     │                                                       │
│ Templates    │  最近專案                         [卡片][表格][篩選] │
│ Automation   │  ┌──────────┐ ┌──────────┐ ┌──────────┐              │
│ Settings     │  │封面預覽  │ │封面預覽  │ │封面預覽  │              │
│              │  │影片標題  │ │影片標題  │ │影片標題  │              │
│              │  │狀態 45%  │ │狀態 80%  │ │缺字幕    │              │
│              │  └──────────┘ └──────────┘ └──────────┘              │
└──────────────┴───────────────────────────────────────────────────────┘
```

## 互動狀態

每一個 async action 必須具有 idle、pending、success、error。長任務另有 progress、canceling、canceled。

## 關鍵元件

- ProjectCard
- StatusBadge
- MissingAssetBanner
- CreateProjectWizard
- AssetGrid／AssetTable
- ScriptEditor
- SubtitleTable
- TaskBoard
- PublishChecklist
- CommandPalette
- Toast＋Persistent Error Panel

## 禁止

- 只用 hover 才能發現主要操作。
- 無確認的破壞性動作。
- 自動關閉且無法找回的錯誤訊息。
- 用「確定失敗」這類沒有修復方法的文案。
