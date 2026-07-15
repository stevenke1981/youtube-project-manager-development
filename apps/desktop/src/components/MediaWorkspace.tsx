import { useEffect, useMemo, useRef, useState } from "react";
import {
  assetList,
  getErrorMessage,
  mediaExportEnqueue,
  mediaJobCancel,
  mediaJobList,
  mediaProbe,
  publishAuthCallback,
  publishAuthStart,
  publishConfigReference,
  publishDryRun,
  publishMetadataLoad,
  publishMetadataSave,
  publishUpload,
  timelineLoad,
  timelineSave
} from "../lib/commands";
import type {
  Asset,
  MediaJob,
  MediaJobStatus,
  MediaMetadata,
  Project,
  PublishConfigReference,
  PublishMetadata,
  PublishOAuthStart,
  PublishResult,
  Timeline,
  TimelineClip,
  TimelineClipEffect,
  TimelineSubtitleStyle,
  TimelineTrack
} from "../types";
import "./MediaWorkspace.css";

type Props = { projectPath: string; project: Project };
type EffectKind = TimelineClipEffect["kind"];

export const FINAL_OUTPUT_RELATIVE_PATH = "09_exports/final.mp4";
const LEGACY_OUTPUT_RELATIVE_PATH = "09_exports/timeline.mp4";

const EFFECT_LABELS: Record<EffectKind, string> = {
  color_adjust: "色彩校正",
  blur: "模糊",
  sharpen: "銳化",
  vignette: "暗角",
  chroma_key: "色鍵去背",
  fade_in: "淡入",
  fade_out: "淡出",
  transform: "位置／縮放／旋轉"
};

export const DEFAULT_SUBTITLE_STYLE: TimelineSubtitleStyle = {
  font_family: "Noto Sans CJK TC",
  font_size: 48,
  primary_color: "#FFFFFF",
  outline_color: "#000000",
  background_color: "#00000000",
  bold: false,
  italic: false,
  outline_width: 3,
  shadow_depth: 1,
  margin_left: 40,
  margin_right: 40,
  margin_vertical: 40,
  alignment: 2
};

export function createTimelineEffect(kind: EffectKind): TimelineClipEffect {
  switch (kind) {
    case "color_adjust": return { kind, brightness: 0, contrast: 1, saturation: 1, gamma: 1 };
    case "blur": return { kind, radius: 4 };
    case "sharpen": return { kind, amount: 1 };
    case "vignette": return { kind, angle: 0.5 };
    case "chroma_key": return { kind, color: "#00FF00", similarity: 0.1, blend: 0.05 };
    case "fade_in": return { kind, duration_ms: 500 };
    case "fade_out": return { kind, duration_ms: 500 };
    case "transform": return { kind, x: 0, y: 0, scale: 1, rotation_degrees: 0, opacity: 1 };
  }
}

export function isTerminalMediaJobStatus(status: MediaJobStatus): boolean {
  return status === "completed" || status === "failed" || status === "cancelled";
}

export function isAssetCompatibleWithTrack(
  asset: Pick<Asset, "kind" | "state">,
  track: Pick<TimelineTrack, "kind">
): boolean {
  if (asset.state !== "available") return false;
  switch (track.kind) {
    case "video":
      return asset.kind === "video" || asset.kind === "image" || asset.kind === "thumbnail" || asset.kind === "export";
    case "audio":
      return asset.kind === "voice" || asset.kind === "music" || asset.kind === "sound_effect";
    case "subtitle":
      return asset.kind === "subtitle";
    case "other":
      return asset.kind === "other";
  }
}

export function mergeMediaJob(current: MediaJob[], job: MediaJob): MediaJob[] {
  const index = current.findIndex((item) => item.id === job.id);
  if (index < 0) return [job, ...current];
  const next = [...current];
  next[index] = job;
  return next;
}

export function normalizeTimeline(value: Timeline): Timeline {
  const timeline = structuredClone(value);
  const outputPath = timeline.output.output_relative_path.trim();
  timeline.output.output_relative_path = !outputPath || outputPath === LEGACY_OUTPUT_RELATIVE_PATH
    ? FINAL_OUTPUT_RELATIVE_PATH
    : outputPath;
  timeline.output.format = "mp4";
  timeline.output.subtitle_style = timeline.output.subtitle_style ?? { ...DEFAULT_SUBTITLE_STYLE };
  for (const track of timeline.tracks) {
    for (const clip of track.clips) clip.effects = clip.effects ?? [];
  }
  return timeline;
}

function jobStatusLabel(status: MediaJobStatus): string {
  return ({ queued: "排隊中", running: "執行中", completed: "完成", failed: "失敗", cancelled: "已取消" } as const)[status];
}

export function MediaWorkspace({ projectPath, project }: Props) {
  const [timeline, setTimeline] = useState<Timeline | null>(null);
  const [assets, setAssets] = useState<Asset[]>([]);
  const [metadata, setMetadata] = useState<PublishMetadata | null>(null);
  const [config, setConfig] = useState<PublishConfigReference | null>(null);
  const [oauthStart, setOauthStart] = useState<PublishOAuthStart | null>(null);
  const [oauthCallbackUrl, setOauthCallbackUrl] = useState("");
  const [selectedAssetId, setSelectedAssetId] = useState("");
  const [selectedTrackId, setSelectedTrackId] = useState("");
  const [selectedClipId, setSelectedClipId] = useState("");
  const [effectKind, setEffectKind] = useState<EffectKind>("color_adjust");
  const [probe, setProbe] = useState<MediaMetadata | null>(null);
  const [jobs, setJobs] = useState<MediaJob[]>([]);
  const [publishResult, setPublishResult] = useState<PublishResult | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const queueMutationEpochRef = useRef(0);

  async function run<T>(label: string, action: () => Promise<T>): Promise<T | null> {
    setBusy(label);
    setError(null);
    setMessage(null);
    try { return await action(); }
    catch (reason) { setError(getErrorMessage(reason)); return null; }
    finally { setBusy(null); }
  }

  useEffect(() => {
    let disposed = false;
    void (async () => {
      const loaded = await run("1", async () => {
        const [nextTimeline, nextAssets, nextMetadata, nextConfig, nextJobs] = await Promise.all([
          timelineLoad(projectPath), assetList(projectPath), publishMetadataLoad(projectPath, project),
          publishConfigReference(projectPath), mediaJobList(projectPath)
        ]);
        if (disposed) return;
        const normalized = normalizeTimeline(nextTimeline);
        const initialAsset = nextAssets.find((asset) =>
          asset.kind === "video" && normalized.tracks.some((track) => isAssetCompatibleWithTrack(asset, track))
        ) ?? nextAssets.find((asset) => normalized.tracks.some((track) => isAssetCompatibleWithTrack(asset, track)));
        setTimeline(normalized);
        setAssets(nextAssets);
        setMetadata(nextMetadata);
        setConfig(nextConfig);
        setJobs(nextJobs);
        setSelectedAssetId(initialAsset?.id ?? "");
        setSelectedTrackId(normalized.tracks.find((track) => initialAsset && isAssetCompatibleWithTrack(initialAsset, track))?.id ?? "");
      });
      if (!disposed && loaded !== null) setMessage("1. 已載入時間軸、素材、背景工作與發布設定。下一步：在 1A 選素材並探勘。");
    })();
    return () => { disposed = true; };
  }, [project, projectPath]);

  const hasActiveJobs = jobs.some((job) => !isTerminalMediaJobStatus(job.status));
  useEffect(() => {
    if (!hasActiveJobs) return;
    let disposed = false;
    let timer: number | undefined;

    async function pollJobs() {
      const requestEpoch = queueMutationEpochRef.current;
      let keepPolling = true;
      try {
        const next = await mediaJobList(projectPath);
        if (disposed) return;
        if (requestEpoch === queueMutationEpochRef.current) {
          setJobs(next);
          keepPolling = next.some((job) => !isTerminalMediaJobStatus(job.status));
        }
      } catch (reason) {
        if (!disposed) setError(getErrorMessage(reason));
      } finally {
        if (!disposed && keepPolling) timer = window.setTimeout(() => void pollJobs(), 800);
      }
    }

    timer = window.setTimeout(() => void pollJobs(), 800);
    return () => {
      disposed = true;
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, [hasActiveJobs, projectPath]);

  const selectedAsset = useMemo(() => assets.find((asset) => asset.id === selectedAssetId) ?? null, [assets, selectedAssetId]);
  const selectedTrack = useMemo(() => timeline?.tracks.find((track) => track.id === selectedTrackId) ?? null, [timeline, selectedTrackId]);
  const selectedClip = useMemo(() => timeline?.tracks.flatMap((track) => track.clips).find((clip) => clip.id === selectedClipId) ?? null, [timeline, selectedClipId]);
  const selectedClipTrack = useMemo(
    () => timeline?.tracks.find((track) => track.clips.some((clip) => clip.id === selectedClipId)) ?? null,
    [timeline, selectedClipId]
  );
  const timelineAssets = useMemo(
    () => assets.filter((asset) => timeline?.tracks.some((track) => isAssetCompatibleWithTrack(asset, track))),
    [assets, timeline?.tracks]
  );
  const compatibleTracks = useMemo(
    () => timeline?.tracks.filter((track) => selectedAsset && isAssetCompatibleWithTrack(selectedAsset, track)) ?? [],
    [selectedAsset, timeline?.tracks]
  );
  const canAddSelectedClip = Boolean(
    selectedAsset && selectedTrack && isAssetCompatibleWithTrack(selectedAsset, selectedTrack)
  );

  function mutateTimeline(mutator: (next: Timeline) => void) {
    if (!timeline) return;
    const next = structuredClone(timeline);
    mutator(next);
    next.duration_ms = Math.max(0, ...next.tracks.flatMap((track) => track.clips.map((clip) => clip.start_ms + Math.max(1, clip.out_ms - clip.in_ms))));
    setTimeline(next);
  }

  function updateClip(clipId: string, patch: Partial<TimelineClip>) {
    mutateTimeline((next) => {
      const clip = next.tracks.flatMap((track) => track.clips).find((item) => item.id === clipId);
      if (!clip) return;
      Object.assign(clip, patch);
      clip.duration_ms = Math.max(1, clip.out_ms - clip.in_ms);
    });
  }

  function updateEffect(clipId: string, index: number, effect: TimelineClipEffect) {
    mutateTimeline((next) => {
      const clip = next.tracks.flatMap((track) => track.clips).find((item) => item.id === clipId);
      if (!clip?.effects[index]) return;
      clip.effects[index] = effect;
    });
  }

  function addEffect() {
    if (!selectedClip) { setError("請先在 3. 剪輯檢查器選擇片段。"); return; }
    if (selectedClipTrack?.kind !== "video") { setError("視覺效果只能加入 video 軌道片段。"); return; }
    updateClip(selectedClip.id, { effects: [...selectedClip.effects, createTimelineEffect(effectKind)] });
    setMessage(`3B. 已加入「${EFFECT_LABELS[effectKind]}」。下一步：調整參數後按 5. 儲存時間軸。`);
  }

  function removeEffect(clipId: string, index: number) {
    const clip = timeline?.tracks.flatMap((track) => track.clips).find((item) => item.id === clipId);
    if (!clip) return;
    updateClip(clipId, { effects: clip.effects.filter((_, effectIndex) => effectIndex !== index) });
  }

  function addSelectedClip() {
    if (!timeline || !selectedTrack || !selectedAsset) { setError("請先在 1A 選素材，再在 2A 選目標軌道。"); return; }
    if (!isAssetCompatibleWithTrack(selectedAsset, selectedTrack)) {
      setError(`素材 ${selectedAsset.kind} 不相容於 ${selectedTrack.kind} 軌道。`);
      return;
    }
    const trackEnd = Math.max(0, ...selectedTrack.clips.map((clip) => clip.start_ms + clip.duration_ms));
    const duration = Math.min(selectedAsset.duration_ms ?? 10_000, 60_000);
    const clip: TimelineClip = {
      id: crypto.randomUUID(), asset_id: selectedAsset.id, relative_path: selectedAsset.relative_path,
      label: selectedAsset.display_name ?? selectedAsset.relative_path, start_ms: trackEnd, in_ms: 0,
      out_ms: duration, duration_ms: duration, volume: 1, muted: false, transition: null, effects: []
    };
    mutateTimeline((next) => {
      const target = next.tracks.find((track) => track.id === selectedTrack.id);
      target?.clips.push(clip);
    });
    setSelectedClipId(clip.id);
    setMessage(`2B. 素材已加入 ${selectedTrack.label}。下一步：在 3. 剪輯檢查器調整時間與效果。`);
  }

  function selectAsset(assetId: string) {
    setSelectedAssetId(assetId);
    const asset = assets.find((item) => item.id === assetId);
    if (!asset || !timeline) {
      setSelectedTrackId("");
      return;
    }
    const currentTrack = timeline.tracks.find((track) => track.id === selectedTrackId);
    if (!currentTrack || !isAssetCompatibleWithTrack(asset, currentTrack)) {
      setSelectedTrackId(timeline.tracks.find((track) => isAssetCompatibleWithTrack(asset, track))?.id ?? "");
    }
  }

  async function probeSelected() {
    if (!selectedAsset) { setError("請先在 1A 選擇素材。"); return; }
    const result = await run("1B", () => mediaProbe(projectPath, selectedAsset.id, selectedAsset.relative_path));
    if (result) { setProbe(result); setMessage("1B. FFprobe 已讀取實際 metadata。下一步：在 2A 選軌並按 2B 加入片段。"); }
  }

  async function saveTimeline() {
    if (!timeline) return;
    const saved = await run("5", () => timelineSave(projectPath, timeline));
    if (saved) { setTimeline(normalizeTimeline(saved)); setMessage("5. timeline.json 已原子儲存。下一步：按 6. 加入背景匯出佇列。"); }
  }

  async function enqueueExport() {
    if (!timeline) return;
    if (!timeline.output.output_relative_path.toLowerCase().endsWith(".mp4")) {
      setError("最終輸出路徑必須以 .mp4 結尾。");
      return;
    }
    if (!window.confirm("確定要執行 6. 背景匯出嗎？既有輸出檔不會被覆寫。")) return;
    queueMutationEpochRef.current += 1;
    const job = await run("6", () => mediaExportEnqueue(projectPath, {
      source_asset_id: selectedAsset?.id ?? null,
      output_relative_path: timeline.output.output_relative_path,
      format: "mp4",
      timeline
    }, true));
    if (job) {
      queueMutationEpochRef.current += 1;
      setJobs((current) => mergeMediaJob(current, job));
      setMessage("6. 已加入背景匯出。下一步：在 7. 工作佇列觀察進度或取消。");
    }
  }

  async function cancelJob(jobId: string) {
    if (!window.confirm("確定要取消這個背景匯出工作嗎？")) return;
    queueMutationEpochRef.current += 1;
    const job = await run("7", () => mediaJobCancel(projectPath, jobId));
    if (job) {
      queueMutationEpochRef.current += 1;
      setJobs((current) => mergeMediaJob(current, job));
      setMessage("7. 已送出取消要求；執行中的 FFmpeg 會被終止，既有輸出不會被覆寫。");
    }
  }

  function updateSubtitleStyle(patch: Partial<TimelineSubtitleStyle>) {
    mutateTimeline((next) => { next.output.subtitle_style = { ...next.output.subtitle_style, ...patch }; });
  }

  async function savePublish() {
    if (!metadata) return;
    const saved = await run("8", () => publishMetadataSave(projectPath, metadata));
    if (saved) { setMetadata(saved); setMessage("8. 發布 metadata 已保存。下一步：8A OAuth 或 9. Dry-run。"); }
  }

  async function startOAuth() {
    const result = await run("8A", publishAuthStart);
    if (result) {
      setOauthStart(result); setOauthCallbackUrl("");
      window.open(result.authorize_url, "ytpm-youtube-oauth", "noopener,noreferrer");
      setMessage("8A. 已開啟 Google OAuth。下一步：將 callback URL 貼到 8B；Token 只留在目前 session。");
    }
  }

  async function completeOAuth() {
    if (!oauthStart || !oauthCallbackUrl.trim()) { setError("請先完成 8A Google 授權，再貼上 callback URL。"); return; }
    const result = await run("8B", () => publishAuthCallback(oauthCallbackUrl.trim(), oauthStart.state, oauthStart.code_verifier));
    if (result) { setConfig(await publishConfigReference(projectPath)); setMessage(`8B. ${result.message} 下一步：按 9. Dry-run。`); }
  }

  async function dryRunPublish() {
    if (!metadata) return;
    const result = await run("9", () => publishDryRun(projectPath, metadata));
    if (result) { setPublishResult(result); setMessage(`${result.message} 下一步：確認結果後按 10. 確認並上傳。`); }
  }

  async function uploadPublish() {
    if (!metadata || !window.confirm("確定要執行 10. YouTube 上傳嗎？這會把 09_exports/final.mp4 傳送到外部服務。")) return;
    const result = await run("10", () => publishUpload(projectPath, metadata, true));
    if (result) { setPublishResult(result); setMessage(result.message); }
  }

  if (!timeline || !metadata) return <section className="panel media-workspace"><p className="inline-status">1. 正在載入 media workstation…</p>{error && <div className="error-banner">{error}</div>}</section>;

  return <section className="panel media-workspace">
    <div className="section-heading">
      <div><span className="eyebrow">STEP 4 · MEDIA WORKSTATION</span><h2>NLE 製作、背景匯出與發布</h2><p>請依 1 → 10 操作；timeline.json 是來源，FFmpeg 只接受已驗證的 typed effects。</p></div>
      <span className="media-badge">{busy ? `執行中 · ${busy}` : hasActiveJobs ? "背景匯出中" : "就緒"}</span>
    </div>
    {error && <div className="error-banner" role="alert">{error}</div>}
    {message && <div className="success-banner" role="status">{message}</div>}

    <div className="media-step-card">
      <strong>1. 選擇與探勘素材</strong>
      <label>1A. 相容素材<select value={selectedAssetId} onChange={(event) => selectAsset(event.target.value)}><option value="">請選擇素材</option>{timelineAssets.map((asset) => <option key={asset.id} value={asset.id}>{asset.kind} · {asset.display_name ?? asset.relative_path}</option>)}</select></label>
      <button className="secondary" type="button" disabled={!selectedAsset || Boolean(busy)} onClick={() => void probeSelected()}>1B. FFprobe 探勘</button>
    </div>
    {probe && <div className="media-result"><strong>1B. FFprobe metadata</strong><code>{probe.format_name || "unknown"} · {probe.duration_seconds?.toFixed(2) ?? "?"}s · {probe.width ?? "?"}×{probe.height ?? "?"} · {probe.video_codec ?? probe.audio_codec ?? "codec?"}</code></div>}

    <div className="media-step-card">
      <strong>2. 加入相容軌道</strong>
      <label>2A. 目標軌道<select value={selectedTrackId} onChange={(event) => setSelectedTrackId(event.target.value)}><option value="">請選擇相容軌道</option>{compatibleTracks.map((track) => <option key={track.id} value={track.id}>{track.kind} · {track.label}</option>)}</select></label>
      <button className="secondary" type="button" disabled={!canAddSelectedClip || Boolean(busy)} onClick={addSelectedClip}>2B. 加入片段</button>
    </div>

    <div className="timeline-editor">
      <div className="section-heading"><div><h3>3. 剪輯檢查器 · {Math.round(timeline.duration_ms / 1000)} 秒</h3><p>3A 選片段，video 片段可加入 3C typed effect；同一軌道不可重疊。</p></div></div>
      {timeline.tracks.map((track) => <div className="timeline-track" key={track.id}><div className="timeline-track-label"><strong>{track.label}</strong><small>{track.kind}</small></div><div className="timeline-clips">{track.clips.length === 0 ? <span className="timeline-empty">尚無片段；回到 2A 選擇此軌道。</span> : track.clips.map((clip) => <button type="button" className={`timeline-clip ${selectedClipId === clip.id ? "selected" : ""}`} key={clip.id} onClick={() => setSelectedClipId(clip.id)}><strong>{clip.label}</strong><span>{clip.start_ms}ms · {clip.duration_ms}ms{track.kind === "video" ? ` · ${clip.effects.length} 個效果` : ""}</span></button>)}</div></div>)}
    </div>

    {selectedClip && <div className="clip-inspector">
      <div className="section-heading"><div><h3>3A. {selectedClip.label}</h3><p>所有數值會先寫入 portable timeline，再由核心驗證後編譯 filter graph。</p></div></div>
      <div className="timeline-fields"><label>開始 ms<input type="number" min="0" value={selectedClip.start_ms} onChange={(event) => updateClip(selectedClip.id, { start_ms: Number(event.target.value) })} /></label><label>In ms<input type="number" min="0" value={selectedClip.in_ms} onChange={(event) => updateClip(selectedClip.id, { in_ms: Number(event.target.value) })} /></label><label>Out ms<input type="number" min="1" value={selectedClip.out_ms} onChange={(event) => updateClip(selectedClip.id, { out_ms: Number(event.target.value) })} /></label><label>音量<input type="number" min="0" max="4" step="0.05" value={selectedClip.volume} onChange={(event) => updateClip(selectedClip.id, { volume: Number(event.target.value) })} /></label><label className="check-field"><input type="checkbox" checked={selectedClip.muted} onChange={(event) => updateClip(selectedClip.id, { muted: event.target.checked })} />靜音</label></div>
      <div className="transition-fields"><label>3B. 轉場<select value={selectedClip.transition?.kind ?? "none"} onChange={(event) => updateClip(selectedClip.id, { transition: event.target.value === "none" ? null : { kind: event.target.value, duration_ms: selectedClip.transition?.duration_ms ?? 500 } })}><option value="none">無</option><option value="fade_in">淡入</option><option value="fade_out">淡出</option><option value="dissolve">溶解</option><option value="crossfade">交叉淡化</option></select></label><label>轉場長度 ms<input type="number" min="1" max={selectedClip.duration_ms} step="50" disabled={!selectedClip.transition} value={selectedClip.transition?.duration_ms ?? 500} onChange={(event) => updateClip(selectedClip.id, { transition: selectedClip.transition ? { ...selectedClip.transition, duration_ms: Number(event.target.value) } : null })} /></label></div>
      {selectedClipTrack?.kind === "video" ? <>
        <div className="effect-add"><label>3C. 效果種類<select value={effectKind} onChange={(event) => setEffectKind(event.target.value as EffectKind)}>{Object.entries(EFFECT_LABELS).map(([kind, label]) => <option key={kind} value={kind}>{label}</option>)}</select></label><button className="secondary" type="button" onClick={addEffect}>3C. 加入 typed effect</button></div>
        <div className="effect-list">{selectedClip.effects.length === 0 ? <p className="timeline-empty">尚無效果。</p> : selectedClip.effects.map((effect, index) => <EffectControls key={`${effect.kind}-${index}`} effect={effect} onChange={(nextEffect) => updateEffect(selectedClip.id, index, nextEffect)} onRemove={() => removeEffect(selectedClip.id, index)} />)}</div>
      </> : <p className="timeline-empty">此為 {selectedClipTrack?.kind ?? "非 video"} 片段，不顯示視覺效果控制。</p>}
    </div>}

    <div className="subtitle-editor"><div className="section-heading"><div><h3>4. 字幕燒錄樣式</h3><p>先在 1A 選 SRT/VTT、2A 選 subtitle 軌，再按 2B；匯出時會轉 ASS 並燒錄。</p></div></div><div className="subtitle-fields"><label>4A. 字型<input value={timeline.output.subtitle_style.font_family} onChange={(event) => updateSubtitleStyle({ font_family: event.target.value })} /></label><label>4B. 字級<input type="number" min="8" max="300" value={timeline.output.subtitle_style.font_size} onChange={(event) => updateSubtitleStyle({ font_size: Number(event.target.value) })} /></label><label>4C. 字色<input type="color" value={timeline.output.subtitle_style.primary_color.slice(0, 7)} onChange={(event) => updateSubtitleStyle({ primary_color: event.target.value.toUpperCase() })} /></label><label>4D. 外框色<input type="color" value={timeline.output.subtitle_style.outline_color.slice(0, 7)} onChange={(event) => updateSubtitleStyle({ outline_color: event.target.value.toUpperCase() })} /></label><label>4E. 背景色<input type="color" value={timeline.output.subtitle_style.background_color.slice(0, 7)} onChange={(event) => updateSubtitleStyle({ background_color: event.target.value.toUpperCase() })} /></label><label>4F. 外框<input type="number" min="0" max="20" step="0.5" value={timeline.output.subtitle_style.outline_width} onChange={(event) => updateSubtitleStyle({ outline_width: Number(event.target.value) })} /></label><label>4G. 陰影<input type="number" min="0" max="20" step="0.5" value={timeline.output.subtitle_style.shadow_depth} onChange={(event) => updateSubtitleStyle({ shadow_depth: Number(event.target.value) })} /></label><label>4H. 左邊界<input type="number" min="0" max="10000" value={timeline.output.subtitle_style.margin_left} onChange={(event) => updateSubtitleStyle({ margin_left: Number(event.target.value) })} /></label><label>4I. 右邊界<input type="number" min="0" max="10000" value={timeline.output.subtitle_style.margin_right} onChange={(event) => updateSubtitleStyle({ margin_right: Number(event.target.value) })} /></label><label>4J. 垂直邊界<input type="number" min="0" max="10000" value={timeline.output.subtitle_style.margin_vertical} onChange={(event) => updateSubtitleStyle({ margin_vertical: Number(event.target.value) })} /></label><label>4K. 對齊<select value={timeline.output.subtitle_style.alignment} onChange={(event) => updateSubtitleStyle({ alignment: Number(event.target.value) })}><option value="1">左下</option><option value="2">中下</option><option value="3">右下</option><option value="4">左中</option><option value="5">置中</option><option value="6">右中</option><option value="7">左上</option><option value="8">中上</option><option value="9">右上</option></select></label><label className="check-field"><input type="checkbox" checked={timeline.output.subtitle_style.bold} onChange={(event) => updateSubtitleStyle({ bold: event.target.checked })} />4L. 粗體</label><label className="check-field"><input type="checkbox" checked={timeline.output.subtitle_style.italic} onChange={(event) => updateSubtitleStyle({ italic: event.target.checked })} />4M. 斜體</label></div></div>

    <div className="media-step-card"><strong>5. 儲存時間軸</strong><label>5A. 最終 MP4 輸出<input value={timeline.output.output_relative_path} onChange={(event) => mutateTimeline((next) => { next.output.output_relative_path = event.target.value; next.output.format = "mp4"; })} /></label><button className="secondary" type="button" disabled={Boolean(busy)} onClick={() => void saveTimeline()}>5B. 儲存 timeline.json</button></div>
    <div className="media-step-card"><strong>6. 加入背景匯出</strong><p className="media-step-copy">只輸出 MP4；預設最終成品為 <code>{FINAL_OUTPUT_RELATIVE_PATH}</code>。</p><button className="primary" type="button" disabled={Boolean(busy)} onClick={() => void enqueueExport()}>6A. 加入背景匯出佇列</button></div>

    <div className="job-queue"><div className="section-heading"><div><h3>7. 背景工作佇列</h3><p>只顯示目前專案工作；工作依序執行，Queue 只保留於目前 App session。</p></div></div>{jobs.length === 0 ? <p className="timeline-empty">尚無工作。下一步：按 6A. 加入背景匯出佇列。</p> : jobs.map((job) => <div className="job-row" key={job.id}><div><strong>{jobStatusLabel(job.status)} · {job.output_relative_path}</strong><small>{job.message ?? job.id}</small></div><progress aria-label={`${job.output_relative_path} 匯出進度 ${job.progress}%`} max="100" value={job.progress}>{job.progress}%</progress><span>{job.progress}%</span>{!isTerminalMediaJobStatus(job.status) && <button className="danger" type="button" disabled={Boolean(busy)} onClick={() => void cancelJob(job.id)}>7A. 取消</button>}</div>)}</div>

    <div className="publish-workspace"><div className="section-heading"><div><h3>8. YouTube metadata 與 OAuth</h3><p>{config?.oauth_ready ? "OAuth 設定參照已就緒。" : "先完成 OAuth；Token 不會寫入專案或 Library。"}</p></div></div><div className="publish-fields"><label>8A. 標題<input value={metadata.title} onChange={(event) => setMetadata({ ...metadata, title: event.target.value })} /></label><label>8B. 可見度<select value={metadata.visibility} onChange={(event) => setMetadata({ ...metadata, visibility: event.target.value as PublishMetadata["visibility"] })}><option value="private">private</option><option value="unlisted">unlisted</option><option value="public">public</option></select></label><label className="publish-description">8C. 描述<textarea value={metadata.description} onChange={(event) => setMetadata({ ...metadata, description: event.target.value })} /></label></div><div className="publish-actions"><button className="secondary" type="button" disabled={Boolean(busy)} onClick={() => void savePublish()}>8D. 儲存 metadata</button><button className="secondary" type="button" disabled={Boolean(busy)} onClick={() => void startOAuth()}>8E. 開啟 OAuth</button></div>{oauthStart && <div className="media-result oauth-callback"><strong>8F. 完成 OAuth</strong><input value={oauthCallbackUrl} onChange={(event) => setOauthCallbackUrl(event.target.value)} placeholder="貼上 http://127.0.0.1:8765 callback URL" /><button className="secondary" type="button" disabled={Boolean(busy)} onClick={() => void completeOAuth()}>8F. 接續授權</button><code>{oauthStart.redirect_uri} · Token 僅保留於目前 App session。</code></div>}</div>
    <div className="media-step-card"><strong>9. Dry-run 發布檢查</strong><p className="media-step-copy">驗證 metadata 與本機成品，不會上傳。</p><button className="secondary" type="button" disabled={Boolean(busy)} onClick={() => void dryRunPublish()}>9A. Dry-run 檢查</button></div>
    <div className="media-step-card"><strong>10. 明確確認並上傳</strong><p className="media-step-copy">只有按下按鈕並再次確認，才會把 <code>{FINAL_OUTPUT_RELATIVE_PATH}</code> 傳送到 YouTube。</p><button className="primary" type="button" disabled={Boolean(busy)} onClick={() => void uploadPublish()}>10A. 確認並上傳 YouTube</button></div>
    {publishResult && <div className="media-result"><strong>9–10. 發布結果</strong><span>{publishResult.status} · {publishResult.message}{publishResult.video_url ? ` · ${publishResult.video_url}` : ""}</span></div>}
  </section>;
}

function EffectControls({ effect, onChange, onRemove }: { effect: TimelineClipEffect; onChange: (next: TimelineClipEffect) => void; onRemove: () => void }) {
  const number = (label: string, value: number, update: (value: number) => void, options: { min?: number; max?: number; step?: number } = {}) => <label>{label}<input type="number" value={value} min={options.min} max={options.max} step={options.step ?? 0.01} onChange={(event) => update(Number(event.target.value))} /></label>;
  let controls: React.ReactNode;
  switch (effect.kind) {
    case "color_adjust": controls = <>{number("brightness", effect.brightness, (value) => onChange({ ...effect, brightness: value }), { min: -1, max: 1 })}{number("contrast", effect.contrast, (value) => onChange({ ...effect, contrast: value }), { min: 0, max: 4 })}{number("saturation", effect.saturation, (value) => onChange({ ...effect, saturation: value }), { min: 0, max: 4 })}{number("gamma", effect.gamma, (value) => onChange({ ...effect, gamma: value }), { min: 0.1, max: 10 })}</>; break;
    case "blur": controls = number("radius", effect.radius, (value) => onChange({ ...effect, radius: value }), { min: 0, max: 100, step: 0.5 }); break;
    case "sharpen": controls = number("amount", effect.amount, (value) => onChange({ ...effect, amount: value }), { min: 0, max: 10 }); break;
    case "vignette": controls = number("angle", effect.angle, (value) => onChange({ ...effect, angle: value }), { min: 0, max: 3.14 }); break;
    case "chroma_key": controls = <><label>color<input type="color" value={effect.color} onChange={(event) => onChange({ ...effect, color: event.target.value.toUpperCase() })} /></label>{number("similarity", effect.similarity, (value) => onChange({ ...effect, similarity: value }), { min: 0, max: 1 })}{number("blend", effect.blend, (value) => onChange({ ...effect, blend: value }), { min: 0, max: 1 })}</>; break;
    case "fade_in": case "fade_out": controls = number("duration_ms", effect.duration_ms, (value) => onChange({ ...effect, duration_ms: value }), { min: 1, max: 60000, step: 50 }); break;
    case "transform": controls = <>{number("x", effect.x, (value) => onChange({ ...effect, x: value }), { min: -10000, max: 10000, step: 1 })}{number("y", effect.y, (value) => onChange({ ...effect, y: value }), { min: -10000, max: 10000, step: 1 })}{number("scale", effect.scale, (value) => onChange({ ...effect, scale: value }), { min: 0.01, max: 20 })}{number("rotation_degrees", effect.rotation_degrees, (value) => onChange({ ...effect, rotation_degrees: value }), { min: -360, max: 360, step: 1 })}{number("opacity", effect.opacity, (value) => onChange({ ...effect, opacity: value }), { min: 0, max: 1 })}</>; break;
  }
  return <div className="effect-card"><div className="effect-card-heading"><strong>{EFFECT_LABELS[effect.kind]}</strong><button type="button" className="danger" onClick={onRemove}>移除</button></div><div className="effect-fields">{controls}</div></div>;
}
