import { useEffect, useState, type FormEvent } from "react";
import type { CreateProjectRequest } from "../types";
import { StepGuide } from "./StepGuide";

type Props = {
  open: boolean;
  rootPath: string;
  busy: boolean;
  error: string | null;
  onClose: () => void;
  onSubmit: (request: CreateProjectRequest) => Promise<void>;
};

function getTitleError(value: string): string | null {
  if (!value.trim()) return "請輸入影片標題，才能建立專案";
  if (value.trim().length > 200) return "影片標題不可超過 200 個字元";
  return null;
}

export function CreateProjectDialog({ open, rootPath, busy, error, onClose, onSubmit }: Props) {
  const [title, setTitle] = useState("");
  const [channel, setChannel] = useState("");
  const [aspectRatio, setAspectRatio] = useState("16:9");
  const [language, setLanguage] = useState("zh-TW");
  const [titleError, setTitleError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setTitle("");
    setChannel("");
    setAspectRatio("16:9");
    setLanguage("zh-TW");
    setTitleError(null);
  }, [open]);

  if (!open) return null;

  async function submit(event: FormEvent) {
    event.preventDefault();
    const validationError = getTitleError(title);
    if (validationError) {
      setTitleError(validationError);
      return;
    }
    setTitleError(null);
    await onSubmit({
      title: title.trim(),
      channel: channel.trim() || undefined,
      aspectRatio,
      language,
      tags: []
    });
  }

  const previewLocation = rootPath
    ? `${rootPath.replace(/[\\/]+$/, "")}\\${new Date().toISOString().slice(0, 10)}_${title.trim() || "影片標題"}`
    : "尚未設定 Library；請先完成 Step 1";

  return (
    <div className="dialog-backdrop" onMouseDown={onClose}>
      <section className="dialog" role="dialog" aria-modal="true" aria-labelledby="create-title" onMouseDown={(event) => event.stopPropagation()}>
        <div className="dialog-header">
          <div><span className="eyebrow">NEW VIDEO PROJECT</span><h2 id="create-title">建立新影片</h2></div>
          <button className="icon-button" type="button" onClick={onClose} aria-label="關閉">×</button>
        </div>

        <StepGuide activeStep={2} className="dialog-guide" />

        <form onSubmit={submit} noValidate>
          <label>
            影片標題
            <input
              autoFocus
              value={title}
              onChange={(event) => { setTitle(event.target.value); if (titleError) setTitleError(getTitleError(event.target.value)); }}
              aria-invalid={Boolean(titleError)}
              aria-describedby={titleError ? "title-error" : undefined}
              maxLength={200}
              placeholder="例如：大模型為什麼不適合計算"
            />
            {titleError && <small className="field-error" id="title-error">{titleError}</small>}
          </label>
          <div className="form-grid">
            <label>頻道<input value={channel} onChange={(event) => setChannel(event.target.value)} placeholder="我的頻道" /></label>
            <label>畫面比例<select value={aspectRatio} onChange={(event) => setAspectRatio(event.target.value)}><option>16:9</option><option>9:16</option><option>1:1</option></select></label>
            <label>語言<select value={language} onChange={(event) => setLanguage(event.target.value)}><option value="zh-TW">繁體中文</option><option value="en-US">English</option><option value="ja-JP">日本語</option></select></label>
          </div>
          <div className="folder-preview"><span>建立位置（由核心安全命名）</span><code>{previewLocation}</code></div>
          {error && <div className="error-banner" role="alert">{error}</div>}
          <div className="dialog-actions"><button type="button" className="secondary" onClick={onClose}>取消</button><button type="submit" className="primary" disabled={busy}>{busy ? "Step 2: 建立中…" : "Step 2: 建立專案"}</button></div>
        </form>
      </section>
    </div>
  );
}
