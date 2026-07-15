import { useEffect, useMemo, useState } from "react";
import {
  assetList,
  getErrorMessage,
  mediaExport,
  mediaProbe,
  publishAuthCallback,
  publishConfigReference,
  publishAuthStart,
  publishDryRun,
  publishMetadataLoad,
  publishMetadataSave,
  publishUpload,
  timelineLoad,
  timelineSave
} from "../lib/commands";
import type { Asset, MediaMetadata, MediaExportResult, Project, PublishConfigReference, PublishMetadata, PublishOAuthStart, PublishResult, Timeline, TimelineClip } from "../types";
import "./MediaWorkspace.css";

type Props = { projectPath: string; project: Project };

export function MediaWorkspace({ projectPath, project }: Props) {
  const [timeline, setTimeline] = useState<Timeline | null>(null);
  const [assets, setAssets] = useState<Asset[]>([]);
  const [metadata, setMetadata] = useState<PublishMetadata | null>(null);
  const [config, setConfig] = useState<PublishConfigReference | null>(null);
  const [oauthStart, setOauthStart] = useState<PublishOAuthStart | null>(null);
  const [oauthCallbackUrl, setOauthCallbackUrl] = useState("");
  const [selectedAssetId, setSelectedAssetId] = useState<string>("");
  const [probe, setProbe] = useState<MediaMetadata | null>(null);
  const [exportResult, setExportResult] = useState<MediaExportResult | null>(null);
  const [publishResult, setPublishResult] = useState<PublishResult | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  async function run<T>(label: string, action: () => Promise<T>): Promise<T | null> {
    setBusy(label);
    setError(null);
    setMessage(null);
    try { return await action(); } catch (reason) { setError(getErrorMessage(reason)); return null; } finally { setBusy(null); }
  }

  useEffect(() => {
    void (async () => {
      const loaded = await run("1", async () => {
        const [nextTimeline, nextAssets, nextMetadata, nextConfig] = await Promise.all([
          timelineLoad(projectPath),
          assetList(projectPath),
          publishMetadataLoad(projectPath, project),
          publishConfigReference(projectPath)
        ]);
        setTimeline(nextTimeline);
        setAssets(nextAssets);
        setMetadata(nextMetadata);
        setConfig(nextConfig);
        setSelectedAssetId(nextAssets.find((asset) => asset.kind === "video")?.id ?? nextAssets[0]?.id ?? "");
      });
      if (loaded !== null) setMessage("1. 已載入 timeline、Asset Catalog 與發布設定參照。");
    })();
  }, [project, projectPath]);

  const selectedAsset = useMemo(() => assets.find((asset) => asset.id === selectedAssetId) ?? null, [assets, selectedAssetId]);
  const videoTrack = timeline?.tracks.find((track) => track.kind === "video") ?? timeline?.tracks[0];

  function updateClip(clipId: string, patch: Partial<TimelineClip>) {
    if (!timeline) return;
    const next = structuredClone(timeline);
    for (const track of next.tracks) {
      const clip = track.clips.find((item) => item.id === clipId);
      if (!clip) continue;
      Object.assign(clip, patch);
      clip.duration_ms = Math.max(1, clip.out_ms - clip.in_ms);
      next.duration_ms = Math.max(next.duration_ms, clip.start_ms + clip.duration_ms);
      setTimeline(next);
      return;
    }
  }

  async function saveTimeline() {
    if (!timeline) return;
    const saved = await run("2", () => timelineSave(projectPath, timeline));
    if (saved) { setTimeline(saved); setMessage("2. timeline.json 已原子儲存；素材檔案沒有被複製或刪除。"); }
  }

  function addSelectedClip() {
    if (!timeline || !videoTrack || !selectedAsset) { setError("請先選擇可用素材與 video track。"); return; }
    const start = timeline.duration_ms;
    const duration = Math.min(selectedAsset.duration_ms ?? 10_000, 60_000);
    const clip: TimelineClip = {
      id: crypto.randomUUID(),
      asset_id: selectedAsset.id,
      relative_path: selectedAsset.relative_path,
      label: selectedAsset.display_name ?? selectedAsset.relative_path,
      start_ms: start,
      in_ms: 0,
      out_ms: duration,
      duration_ms: duration,
      volume: 1,
      muted: false,
      transition: null
    };
    setTimeline({ ...structuredClone(timeline), duration_ms: start + duration, tracks: timeline.tracks.map((track) => track.id === videoTrack.id ? { ...track, clips: [...track.clips, clip] } : track) });
    setMessage("2. 已加入剪輯片段；請調整 trim 後按「2. 儲存時間軸」。");
  }

  async function probeSelected() {
    if (!selectedAsset) { setError("請先選擇要探勘的素材。"); return; }
    const result = await run("3", () => mediaProbe(projectPath, selectedAsset.id, selectedAsset.relative_path));
    if (result) { setProbe(result); setMessage("3. FFprobe 已讀取實際媒體 metadata。"); }
  }

  async function exportVideo() {
    if (!timeline) return;
    if (!window.confirm("確定要執行 4. FFmpeg 匯出嗎？既有輸出檔不會被覆寫。")) return;
    const result = await run("4", () => mediaExport(projectPath, { source_asset_id: selectedAsset?.id ?? null, output_relative_path: timeline.output.output_relative_path, format: timeline.output.format === "webm" ? "webm" : "mp4", timeline }, true));
    if (result) { setExportResult(result); setMessage(result.message ?? "4. FFmpeg 匯出完成。"); }
  }

  async function savePublish() {
    if (!metadata) return;
    const saved = await run("5", () => publishMetadataSave(projectPath, metadata));
    if (saved) { setMetadata(saved); setMessage("5. 發布 metadata 已保存至 08_metadata/publish.json。"); }
  }

  async function startOAuth() {
    const result = await run("5A", publishAuthStart);
    if (result) {
      setOauthStart(result);
      setOauthCallbackUrl("");
      window.open(result.authorize_url, "ytpm-youtube-oauth", "noopener,noreferrer");
      setMessage("5A. 已開啟 Google OAuth；授權完成後請把瀏覽器網址列的 callback URL 貼回 5B，App 會在目前 session 接續上傳。");
    }
  }

  async function completeOAuth() {
    if (!oauthStart || !oauthCallbackUrl.trim()) { setError("請先完成 Google 授權，再貼上 callback URL。"); return; }
    const result = await run("5B", () => publishAuthCallback(oauthCallbackUrl.trim(), oauthStart.state, oauthStart.code_verifier));
    if (result) {
      setConfig(await publishConfigReference(projectPath));
      setMessage(`5B. ${result.message}`);
    }
  }

  async function dryRunPublish() {
    if (!metadata) return;
    const result = await run("6", () => publishDryRun(projectPath, metadata));
    if (result) { setPublishResult(result); setMessage(result.message); }
  }

  async function uploadPublish() {
    if (!metadata || !window.confirm("確定要 7. 連線 YouTube 並上傳 09_exports/final.mp4 嗎？")) return;
    const result = await run("7", () => publishUpload(projectPath, metadata, true));
    if (result) { setPublishResult(result); setMessage(result.message); }
  }

  if (!timeline || !metadata) return <section className="panel media-workspace"><p className="inline-status">1. 正在載入 media workstation…</p>{error && <div className="error-banner">{error}</div>}</section>;

  return <section className="panel media-workspace">
    <div className="section-heading"><div><span className="eyebrow">STEP 4 · MEDIA WORKSTATION</span><h2>完整製作、探勘與發布</h2><p>timeline.json 是來源；SQLite 只做索引，FFmpeg 只接受 argv。</p></div><span className="media-badge">{busy ? `執行中 · Step ${busy}` : "就緒"}</span></div>
    {error && <div className="error-banner" role="alert">{error}</div>}
    {message && <div className="success-banner" role="status">{message}</div>}
    <div className="media-step-grid">
      <div className="media-step-card"><strong>1. 選素材</strong><select value={selectedAssetId} onChange={(event) => setSelectedAssetId(event.target.value)}><option value="">請選擇 Asset Catalog 素材</option>{assets.map((asset) => <option key={asset.id} value={asset.id}>{asset.kind} · {asset.display_name ?? asset.relative_path}</option>)}</select><button className="secondary" type="button" disabled={!selectedAsset || Boolean(busy)} onClick={() => void probeSelected()}>3. FFprobe 探勘</button></div>
      <div className="media-step-card"><strong>2. 編輯時間軸</strong><button className="secondary" type="button" disabled={!selectedAsset || Boolean(busy)} onClick={addSelectedClip}>2A. 加入片段</button><button className="secondary" type="button" disabled={Boolean(busy)} onClick={() => void saveTimeline()}>2B. 儲存時間軸</button><button className="primary" type="button" disabled={Boolean(busy)} onClick={() => void exportVideo()}>4. FFmpeg 匯出</button></div>
    </div>
    <div className="timeline-editor"><div className="section-heading"><div><h3>Timeline · {Math.round(timeline.duration_ms / 1000)} 秒</h3><p>直接編輯 start / in / out；同一 track 不可重疊。</p></div></div>{timeline.tracks.map((track) => <div className="timeline-track" key={track.id}><div className="timeline-track-label"><strong>{track.label}</strong><small>{track.kind}</small></div><div className="timeline-clips">{track.clips.length === 0 ? <span className="timeline-empty">尚無片段</span> : track.clips.map((clip) => <div className="timeline-clip" key={clip.id}><strong>{clip.label}</strong><div className="timeline-fields"><label>開始<input type="number" min="0" value={clip.start_ms} onChange={(event) => updateClip(clip.id, { start_ms: Number(event.target.value) })} /></label><label>In<input type="number" min="0" value={clip.in_ms} onChange={(event) => updateClip(clip.id, { in_ms: Number(event.target.value) })} /></label><label>Out<input type="number" min="1" value={clip.out_ms} onChange={(event) => updateClip(clip.id, { out_ms: Number(event.target.value) })} /></label></div></div>)}</div></div>)}</div>
    {probe && <div className="media-result"><strong>3. FFprobe metadata</strong><code>{probe.format_name || "unknown"} · {probe.duration_seconds?.toFixed(2) ?? "?"}s · {probe.width ?? "?"}×{probe.height ?? "?"} · {probe.video_codec ?? probe.audio_codec ?? "codec?"}</code></div>}
    {exportResult && <div className="media-result"><strong>4. 匯出結果</strong><span>{exportResult.status} · {exportResult.output_relative_path ?? "無輸出"} · {exportResult.message}</span></div>}
    <div className="publish-workspace"><div className="section-heading"><div><h3>5–7. YouTube 發布</h3><p>{config?.oauth_ready ? "OAuth 設定參照已就緒。" : "先完成 OAuth 環境設定；dry-run 不會連線。"}</p></div></div><div className="publish-fields"><label>標題<input value={metadata.title} onChange={(event) => setMetadata({ ...metadata, title: event.target.value })} /></label><label>可見度<select value={metadata.visibility} onChange={(event) => setMetadata({ ...metadata, visibility: event.target.value as PublishMetadata["visibility"] })}><option value="private">private</option><option value="unlisted">unlisted</option><option value="public">public</option></select></label><label className="publish-description">描述<textarea value={metadata.description} onChange={(event) => setMetadata({ ...metadata, description: event.target.value })} /></label></div><div className="publish-actions"><button className="secondary" type="button" disabled={Boolean(busy)} onClick={() => void startOAuth()}>5A. 開啟 OAuth</button><button className="secondary" type="button" disabled={Boolean(busy)} onClick={() => void savePublish()}>5. 儲存 metadata</button><button className="secondary" type="button" disabled={Boolean(busy)} onClick={() => void dryRunPublish()}>6. Dry-run 檢查</button><button className="primary" type="button" disabled={Boolean(busy)} onClick={() => void uploadPublish()}>7. 確認並上傳 YouTube</button></div>{oauthStart && <div className="media-result oauth-callback"><strong>5B. 完成 OAuth</strong><input value={oauthCallbackUrl} onChange={(event) => setOauthCallbackUrl(event.target.value)} placeholder="貼上授權完成後的 http://127.0.0.1:8765 callback URL" /><button className="secondary" type="button" disabled={Boolean(busy)} onClick={() => void completeOAuth()}>5B. 接續授權</button><code>{oauthStart.redirect_uri} · token 僅保留在目前 App session。</code></div>}{publishResult && <div className="media-result"><strong>發布結果</strong><span>{publishResult.status} · {publishResult.message}{publishResult.video_url ? ` · ${publishResult.video_url}` : ""}</span></div>}</div>
  </section>;
}
