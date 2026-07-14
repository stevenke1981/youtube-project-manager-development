# Security & Safety

## Threats

- 路徑穿越與 symlink escape。
- 惡意檔名造成 shell injection。
- Markdown／字幕內容注入 WebView。
- API Token 儲存不當。
- 外部 AI 上傳私密素材。
- 自動化代理批次刪除。

## Controls

- 所有 relative path join 後 canonicalize，確認仍在 project root。
- External process 使用 argv，不使用 `cmd /c` 拼接。
- Markdown sanitize，預設禁用 raw HTML。
- Token 存 OS credential manager，不進 JSON／log。
- Adapter 宣告 data egress，第一次使用明確同意。
- Archive 與 delete 分離；永久刪除需要 typed confirmation。
- Log redact home path、token、query string。

## Tauri

Frontend 不直接取得 unrestricted filesystem／shell。能力由 command 封裝與 allowlist 控制。

## YouTube OAuth

使用 system browser＋PKCE，refresh token 安全儲存；disconnect 時撤銷並刪除本機 credential。
