# Error Handling

## Error Shape

```json
{
  "code": "PROJECT_ROOT_NOT_WRITABLE",
  "message": "無法在指定位置建立影片專案。",
  "detail": "Access denied: D:\\YouTube-Projects",
  "recoverable": true,
  "actions": ["CHOOSE_ANOTHER_ROOT", "OPEN_PERMISSION_HELP"]
}
```

## Error UX

- Field error：輸入旁。
- Toast：已完成、可忽略短暫事件。
- Banner：影響目前頁面的問題。
- Error center：長任務、索引、外部工具問題。
- Fatal screen：只有 App 無法載入核心設定時。

## Retry

只對 idempotent operation 自動 retry；寫入與 external process 不盲目重試。Retry 使用 exponential backoff＋上限。
