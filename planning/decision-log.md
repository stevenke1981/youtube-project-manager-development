# Architecture Decision Log

## ADR-001：Tauri 2 而非 Electron

狀態：Accepted。理由：Rust 核心、較小桌面包、跨平台與系統整合。代價：Windows WebView2 與 Rust 工具鏈需求。

## ADR-002：資料夾／JSON 是真實來源

狀態：Accepted。理由：可攜、可備份、可由 Agent 與第三方工具操作。代價：需要掃描、同步與衝突策略。

## ADR-003：SQLite 是可重建索引

狀態：Accepted。理由：搜尋與儀表板效能；避免資料被 DB 鎖住。

## ADR-004：共享 Rust core

狀態：Accepted。理由：GUI／CLI 規則一致，方便未來 headless automation。

## ADR-005：MVP 不內建影片剪輯器

狀態：Accepted。理由：控制範圍，優先解決專案與素材混亂；透過外部工具開啟與 FFmpeg adapter 補足。
