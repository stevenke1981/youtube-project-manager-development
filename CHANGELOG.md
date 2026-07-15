# Changelog

## NLE v1.0 — 2026-07-15

- Timeline schema v2：typed effects、subtitle style、v1 exact backup 與 atomic migration。
- 單一 multi-track FFmpeg filter graph：影音定位／混音、color、blur、sharpen、vignette、chroma key、fade、transform、transition。
- SRT／WebVTT parser、clip trim/time offset、UTF-8 ASS generation 與 subtitle burn-in。
- 可取消的 FFmpeg child、progress pipe、partial-output/temp cleanup。
- Process-local single-worker export queue 與 Tauri enqueue/status/list/cancel commands。
- React NLE 工作台改為 1–10 編號流程，加入 track、effect、subtitle style、背景進度與取消控制。
- Rust 84 tests、React 20 tests、release build、MSI/NSIS checksum gate 通過。

## 0.1.0 — Development Kit

- 建立產品、流程、UI、架構與資料模型完整規格。
- 建立 Rust workspace：`ytpm-core`、`ytpm-cli`。
- 建立 Tauri 2＋React 桌面應用骨架。
- 實作專案建立、列出、驗證的 MVP 核心。
- 加入可操作的 Step 1/2/3 工作流、搜尋／篩選／排序、validation 回饋、status 更新與確認後封存。
- 實作 archive／restore：完整資料夾移動、operation journal、衝突保護與 symlink/junction 拒絕。
- 加入 JSON Schema、SQLite migration、PowerShell scripts、CI。
- 完成可重建的 SQLite project index／FTS search，並接入 Tauri 與 CLI。
- 完成 `tasks.json` Task/Kanban、`assets.json` Asset Catalog、SHA-256/missing 掃描與原子寫入。
- 完成腳本／發布文件編輯器、800ms autosave、失敗草稿恢復與 document path allowlist。
- 完成 journal 自動恢復 metadata 修補、fail-closed reparse policy 與中文 Windows junction smoke test。
- Desktop 目前提供每個工作步驟的 `Step 1/2/3`、`Next step:` 與可操作控制項。
