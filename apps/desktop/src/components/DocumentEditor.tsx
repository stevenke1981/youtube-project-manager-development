import { useCallback, useEffect, useRef, useState } from "react";
import { FileText, RefreshCw } from "lucide-react";
import { documentRead, documentWrite, getErrorMessage } from "../lib/commands";
import type { DocumentType } from "../types";

export type EditorSaveStatus = "loading" | "idle" | "saving" | "saved" | "error";

export const EDITOR_DOCUMENT_PATHS: Record<DocumentType, string> = {
  script: "02_script/script.md",
  publish: "08_metadata/description.md"
};

export function getDocumentPath(documentType: DocumentType): string {
  return EDITOR_DOCUMENT_PATHS[documentType];
}

export function getSaveStatusLabel(status: EditorSaveStatus): string {
  return {
    loading: "Loading…",
    idle: "Unsaved changes",
    saving: "Saving…",
    saved: "Saved",
    error: "Error"
  }[status];
}

export function getEditorNextStep(documentType: DocumentType): string {
  return documentType === "script"
    ? "Next step: 完成腳本後，切換到語音製作並建立旁白任務。"
    : "Next step: 確認描述與發布資訊後，前往發布前審核。";
}

type Props = { projectPath: string; documentType: DocumentType };

export function DocumentEditor({ projectPath, documentType }: Props) {
  const relativePath = getDocumentPath(documentType);
  const [content, setContent] = useState("");
  const [ready, setReady] = useState(false);
  const [status, setStatus] = useState<EditorSaveStatus>("loading");
  const [error, setError] = useState<string | null>(null);
  const [savedAt, setSavedAt] = useState<string | null>(null);
  const [recoveredDraft, setRecoveredDraft] = useState(false);
  const originalContent = useRef("");
  const changeVersion = useRef(0);

  const draftStorageKey = `ytpm:draft:${projectPath}:${relativePath}`;

  function readDraft(): string | null {
    try {
      return window.localStorage.getItem(draftStorageKey);
    } catch {
      return null;
    }
  }

  function saveDraft(value: string) {
    try {
      window.localStorage.setItem(draftStorageKey, value);
    } catch {
      // The on-disk document remains the source of truth when storage is unavailable.
    }
  }

  function clearDraft() {
    try {
      window.localStorage.removeItem(draftStorageKey);
    } catch {
      // Best-effort cleanup only; a failed localStorage write must not block saving.
    }
  }

  const loadDocument = useCallback(async () => {
    setReady(false);
    setStatus("loading");
    setError(null);
    setSavedAt(null);
    try {
      const nextContent = await documentRead(projectPath, relativePath);
      const draft = readDraft();
      const hasDraft = draft !== null && draft !== nextContent;
      originalContent.current = nextContent;
      changeVersion.current = 0;
      setContent(hasDraft ? draft : nextContent);
      setRecoveredDraft(hasDraft);
      setReady(true);
      setStatus(hasDraft ? "idle" : "saved");
    } catch (reason) {
      originalContent.current = "";
      changeVersion.current = 0;
      setContent("");
      setRecoveredDraft(false);
      setReady(true);
      setStatus("error");
      setError(getErrorMessage(reason));
    }
  }, [projectPath, relativePath]);

  useEffect(() => { void loadDocument(); }, [loadDocument]);

  useEffect(() => {
    if (!ready || content === originalContent.current) return;
    const version = changeVersion.current;
    const value = content;
    const timer = window.setTimeout(() => {
      void (async () => {
        setStatus("saving");
        try {
          const result = await documentWrite(projectPath, relativePath, value);
          if (version !== changeVersion.current) return;
          originalContent.current = value;
          clearDraft();
          setRecoveredDraft(false);
          setSavedAt(result.saved_at);
          setStatus("saved");
          setError(null);
        } catch (reason) {
          if (version !== changeVersion.current) return;
          setStatus("error");
          setError(getErrorMessage(reason));
        }
      })();
    }, 800);
    return () => window.clearTimeout(timer);
  }, [content, projectPath, ready, relativePath]);

  function handleChange(value: string) {
    changeVersion.current += 1;
    setContent(value);
    saveDraft(value);
    setStatus("idle");
    setError(null);
  }

  return <section className="panel tab-panel document-editor" role="tabpanel">
    <div className="section-heading">
      <div><span className="eyebrow">STEP 3 · {documentType === "script" ? "SCRIPT" : "PUBLISH"}</span><h2>{documentType === "script" ? "腳本編輯器" : "發布描述編輯器"}</h2><p><FileText size={15} /> <code>{relativePath}</code> · 真正讀寫專案檔案，不以 localStorage 作為唯一來源。</p></div>
      <button className="secondary" type="button" onClick={() => void loadDocument()} disabled={status === "loading" || status === "saving"}><RefreshCw size={15} /> Step 3: 重新讀取</button>
    </div>
    <div className="editor-status-row"><span className={`save-status save-status-${status}`} role="status" aria-live="polite">{getSaveStatusLabel(status)}</span>{savedAt && <small>saved_at: {new Date(savedAt).toLocaleString("zh-TW")}</small>}<span className="editor-debounce">800ms autosave</span></div>
    {recoveredDraft && <div className="editor-recovery-note" role="status">已從上次未完成的本機草稿恢復；確認內容後會在 800ms 後寫回專案檔案。</div>}
    {error && <div className="error-banner" role="alert">{error}</div>}
    <label className="editor-label" htmlFor={`document-editor-${documentType}`}>文件內容</label>
    <textarea id={`document-editor-${documentType}`} className="document-textarea" value={content} onChange={(event) => handleChange(event.target.value)} placeholder="在這裡開始編輯 Markdown…" spellCheck={false} />
    <div className="editor-next-step"><strong>{getEditorNextStep(documentType)}</strong><small>離開分頁前請確認狀態顯示 Saved。</small></div>
  </section>;
}
