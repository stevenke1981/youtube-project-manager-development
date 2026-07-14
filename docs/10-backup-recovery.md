# Backup & Recovery

## Backup Units

最小單位是一個 Project folder；Library index 不需要備份即可重建。

## Strategies

- Manual export：產生 manifest＋checksum，可選是否含 raw media。
- Scheduled copy：交由使用者選擇 NAS／外接碟；App 不默認雲端。
- Snapshot before migration：`.ytpm-backup/schema-vN-timestamp/`。

## Recovery

1. 掃描 `project.json`。
2. 驗證 schema。
3. 執行逐版 migration 到 temp。
4. 備份原檔。
5. 原子取代。
6. 重建 SQLite index。

## Corruption

損壞 JSON 不自動覆寫。保留原檔，嘗試 `.bak`／temp，提供 diagnostics 與手動選擇。
