# Engineering Design

## 架構決策

採用 Hexagonal Architecture：

```text
React UI ──Tauri Commands──> Application Services ──> Domain
CLI ───────────────────────> Application Services ──> Domain
                                  │
                         Ports / Repositories
                       ┌──────────┴──────────┐
                    File System          SQLite Index
```

`ytpm-core` 不依賴 Tauri，也不依賴 React。核心輸入輸出使用明確 DTO。

## 模組

- `domain`：Project、Asset、Task、Workflow、Validation。
- `application`：create/list/validate/archive/import。
- `infrastructure-fs`：資料夾、JSON、hash、watcher。
- `infrastructure-db`：SQLite cache、FTS、migration。
- `adapter-cli`：clap commands。
- `adapter-tauri`：invoke commands、events、permissions。
- `frontend`：React view state、forms、preview。

## 一致性策略

1. 先寫 project temp JSON。
2. flush／sync。
3. rename 取代正式檔。
4. 更新 SQLite transaction。
5. 發送 `project://updated` event。

SQLite 更新失敗時，專案檔仍有效；App 顯示「索引待重建」。

## 檔案監控

- debounce 500ms。
- 以 project root 為監控單位。
- 合併重複事件。
- 掃描工作可取消。
- 不因外部刪除立即移除 asset record，先標 missing。

## ID

- Project、Asset、Task 使用 UUID v4。
- 檔案路徑不是 ID。
- 搬移檔案後可用 hash＋大小＋候選路徑重新連結。

## 版本

- `schema_version`：資料格式。
- `app_version`：建立／最後修改 App 版本。
- migration 採逐版函式，不允許直接跳過中間版。

## 錯誤

核心錯誤必須包含：code、human_message、technical_detail、recoverable、suggested_action。UI 不直接顯示 Rust debug string。
