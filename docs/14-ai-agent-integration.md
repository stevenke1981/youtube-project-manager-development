# AI Agent Integration

## 原則

Agent 只透過 CLI／structured files 操作，不直接猜測目錄。所有修改留下 actor、operation_id、summary。

## 建議檔案

- `AGENTS.md`：規則。
- `project.json`：專案狀態。
- `tasks.json`：工作。
- `assets.json`：素材索引。
- `activity.log`：append-only JSON Lines。

## Agent Commands（Roadmap）

```text
ytpm task next --project PATH --json
ytpm task complete --project PATH --task-id UUID --evidence FILE
ytpm asset register --project PATH --kind image --file FILE --json
ytpm project validate --fix-safe --json
ytpm prompt export --project PATH --scene 03 --target grok-imagine
```

## Safety

- `--fix-safe` 只能建立缺少資料夾、正規化 metadata，不刪除或覆寫素材。
- Agent 產生檔案先放 `generated/`，採用由 `is_adopted` 決定。
- 高成本／外部 API 工作需 budget 與 explicit approval。
- 每個自動任務要有完成條件，不以「看起來完成」判定。
