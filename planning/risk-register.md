# Risk Register

| ID | 風險 | 機率 | 影響 | 緩解 |
|---|---|---:|---:|---|
| R1 | 大量素材掃描卡住 UI | 中 | 高 | background worker、batch、cancel、virtualization |
| R2 | 外部工具改名導致連結失效 | 高 | 中 | UUID record、hash relink、missing state |
| R3 | SQLite 與 JSON 不一致 | 中 | 高 | JSON truth、transaction、rebuild index |
| R4 | Windows 路徑與權限差異 | 高 | 高 | path policy、中文矩陣、可行動錯誤 |
| R5 | FFmpeg 命令注入 | 低 | 高 | argv API、不經 shell、測試惡意檔名 |
| R6 | AI API 洩漏素材 | 中 | 高 | local-first、explicit consent、adapter permission |
| R7 | Scope 變成完整剪輯器 | 高 | 高 | Boundary、roadmap gate、NLE integration only |
| R8 | Tauri／前端依賴更新 | 中 | 中 | lockfile、Renovate、release branch、CI |
| R9 | 長任務中斷造成半成品 | 中 | 高 | operation journal、temp output、resume／cleanup |
