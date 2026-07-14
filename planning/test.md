# Test Plan

## 測試層級

### Unit

- sanitize folder name。
- unique folder allocation。
- project serialization／deserialization。
- status transitions。
- progress calculation。
- subtitle timestamp validation。

### Integration

- 建立專案後目錄與範本完整。
- project.json 寫入中斷不破壞舊檔。
- 搬移專案後重新掃描。
- SQLite index 可從檔案重建。
- 外部新增／改名／刪除素材。

### Contract

- CLI JSON output schema。
- Tauri command request／response DTO。
- schema_version migration fixtures。

### UI

- 新增專案成功／失敗。
- 空狀態、loading、error、partial data。
- keyboard navigation、focus trap、Escape。
- 125%、150%、200% zoom。
- 1366×768 最小支援尺寸。

### E2E

1. 選擇 Library root。
2. 建立中文影片專案。
3. 開啟專案，確認所有分頁。
4. 外部加入一張圖與一個 SRT。
5. 重新整理後素材出現。
6. 驗證並修復缺漏。
7. 封存、還原。

## Windows 路徑矩陣

- `D:\YouTube Projects`
- `D:\影片製作\頻道 A`
- 長標題接近 Windows path limit。
- 標題含 `<>:"/\\|?*`。
- 標題含 emoji、全形空白、結尾句點。
- root 唯讀、網路磁碟中斷、磁碟滿。

## 效能基準

- 1,000 projects list：冷掃描 < 5 秒；索引後 < 500ms。
- 10,000 assets incremental scan < 3 秒（無內容變更）。
- UI scrolling 目標 60fps，素材列表使用 virtualization。

## 安全測試

- `../../outside` 路徑輸入。
- symlink 指向 root 外。
- 惡意 subtitle／markdown HTML。
- FFmpeg 參數注入檔名。
- 損壞 JSON、超大 JSON、重複 ID。

## 指令

```powershell
.\scripts\test.ps1
```

## 測試證據

每次 release 在 `planning/final.md` 記錄：環境、commit、命令、結果、未執行項目與原因。
