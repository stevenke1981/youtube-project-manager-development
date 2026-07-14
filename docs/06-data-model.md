# Data Model

## Project

- id: UUID
- schema_version: integer；目前為 2。v1 可透過 `ytpm migrate` 升級，升級前備份至 `.ytpm-backup/`。
- title／folder_name／channel／series
- status
- archived_from_status（封存前狀態，還原時恢復）
- aspect_ratio／language／target_duration_seconds
- planned_publish_at／published_at
- progress
- tags
- created_at／updated_at
- app_version

## Asset

- id、project_id、kind、relative_path
- display_name、extension、mime_type
- size_bytes、sha256（延遲計算）
- duration_ms、width、height、fps、channels、sample_rate
- source_type：created/imported/generated/linked/downloaded
- generator、model、prompt、seed
- license、source_url、attribution
- version_group_id、version_number、is_adopted
- state：available/missing/archived/processing/error

## Task

- id、project_id、title、description
- status：todo/doing/review/blocked/done
- priority、due_at、completed_at
- related_asset_ids、checklist、acceptance_criteria
- order_key

## Event

append-only，包含 actor（user/agent/system）、action、entity、before／after 摘要、timestamp、operation_id。

## Relative Paths

專案內所有 asset 路徑以 `/` 正規化的 relative path 儲存。顯示或存取時才與實際 project root join，並再次確認 canonical path 未逃出 root。

## Progress

MVP 使用 deliverable 權重，不以檔案數量計算：

- Research 5
- Script 20
- Voice 15
- Visuals 15
- Edit 20
- Subtitle 10
- Thumbnail 5
- Publish metadata 5
- Review 5

模板可覆寫權重，但總和必須 100。
