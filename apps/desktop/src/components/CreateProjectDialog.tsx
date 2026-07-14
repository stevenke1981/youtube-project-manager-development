import { useState, type FormEvent } from "react";
import type { CreateProjectRequest } from "../types";

type Props = {
  open: boolean;
  rootPath: string;
  busy: boolean;
  error: string | null;
  onClose: () => void;
  onSubmit: (request: CreateProjectRequest) => Promise<void>;
};

export function CreateProjectDialog({ open, rootPath, busy, error, onClose, onSubmit }: Props) {
  const [title, setTitle] = useState("");
  const [channel, setChannel] = useState("");
  const [aspectRatio, setAspectRatio] = useState("16:9");
  const [language, setLanguage] = useState("zh-TW");

  if (!open) return null;

  async function submit(event: FormEvent) {
    event.preventDefault();
    await onSubmit({
      title,
      channel: channel || undefined,
      aspectRatio,
      language,
      tags: []
    });
  }

  return (
    <div className="dialog-backdrop" onMouseDown={onClose}>
      <section className="dialog" role="dialog" aria-modal="true" aria-labelledby="create-title" onMouseDown={(e) => e.stopPropagation()}>
        <div className="dialog-header">
          <div><span className="eyebrow">NEW VIDEO PROJECT</span><h2 id="create-title">建立新影片</h2></div>
          <button className="icon-button" onClick={onClose} aria-label="關閉">×</button>
        </div>
        <form onSubmit={submit}>
          <label>影片標題<input autoFocus value={title} onChange={(e) => setTitle(e.target.value)} required maxLength={200} placeholder="例如：大模型為什麼不適合計算" /></label>
          <div className="form-grid">
            <label>頻道<input value={channel} onChange={(e) => setChannel(e.target.value)} placeholder="我的頻道" /></label>
            <label>畫面比例<select value={aspectRatio} onChange={(e) => setAspectRatio(e.target.value)}><option>16:9</option><option>9:16</option><option>1:1</option></select></label>
            <label>語言<select value={language} onChange={(e) => setLanguage(e.target.value)}><option value="zh-TW">繁體中文</option><option value="en-US">English</option><option value="ja-JP">日本語</option></select></label>
          </div>
          <div className="folder-preview"><span>建立位置</span><code>{rootPath || "尚未設定 Library"} / 日期_影片標題</code></div>
          {error && <div className="error-banner">{error}</div>}
          <div className="dialog-actions"><button type="button" className="secondary" onClick={onClose}>取消</button><button className="primary" disabled={busy || !title.trim()}>{busy ? "建立中…" : "建立專案"}</button></div>
        </form>
      </section>
    </div>
  );
}
