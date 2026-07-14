# Project Folder Specification

```text
YYYY-MM-DD_影片標題/
├── project.json
├── README.md
├── tasks.json
├── assets.json
├── activity.log
├── 01_research/
├── 02_script/
│   └── prompts/
├── 03_voice/
│   ├── raw/
│   ├── processed/
│   ├── music/
│   └── sound_effects/
├── 04_images/
│   ├── generated/
│   ├── references/
│   ├── characters/
│   ├── backgrounds/
│   └── selected/
├── 05_video/
│   ├── raw/
│   ├── generated/
│   ├── edited/
│   └── final/
├── 06_subtitles/
│   └── translations/
├── 07_thumbnail/
│   ├── source/
│   ├── drafts/
│   └── final/
├── 08_metadata/
├── 09_exports/
│   ├── review/
│   └── upload/
└── 10_archive/
```

## Naming Policy

- 替換 `< > : " / \\ | ? *` 與控制字元為 `_`。
- 移除結尾空白與句點。
- 避免 `CON PRN AUX NUL COM1.. LPT9`。
- 目標資料夾 component 建議不超過 80 字元。
- 同名依序加 `-02`。
- 不用 title slug 作 ID。

## Required vs Optional

MVP 所有編號資料夾建立。缺少必要編號資料夾或發現 symlink/junction 時，validator 回傳 `Error`；可選的內部子資料夾或非阻斷提示才使用 `Warning`。

## Hidden App Files

`.ytpm/` 可存 operation journal、lock、migration backup。不可存唯一內容。
