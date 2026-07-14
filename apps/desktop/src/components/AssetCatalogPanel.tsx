import { useCallback, useEffect, useMemo, useState } from "react";
import { RefreshCw, Search } from "lucide-react";
import { assetList, assetScan, getErrorMessage } from "../lib/commands";
import type { Asset, AssetCatalog, AssetKind } from "../types";

export const assetKindLabels: Record<AssetKind, string> = {
  research: "研究",
  script: "腳本",
  voice: "語音",
  music: "音樂",
  sound_effect: "音效",
  image: "圖片",
  video: "影片",
  subtitle: "字幕",
  thumbnail: "封面",
  metadata: "發布資料",
  export: "輸出",
  other: "其他"
};

export function filterAssets(
  assets: Asset[],
  options: { kind?: AssetKind | "all"; query?: string; includeMissing?: boolean } = {}
): Asset[] {
  const query = options.query?.trim().toLocaleLowerCase("zh-TW") ?? "";
  return assets.filter((asset) => {
    const matchesKind = !options.kind || options.kind === "all" || asset.kind === options.kind;
    const matchesQuery = !query || [asset.relative_path, asset.display_name ?? "", asset.kind, asset.state]
      .join(" ")
      .toLocaleLowerCase("zh-TW")
      .includes(query);
    const matchesMissing = options.includeMissing !== false || asset.state !== "missing";
    return matchesKind && matchesQuery && matchesMissing;
  });
}

export function summarizeAssetCatalog(projectPath: string, assets: Asset[], scannedAt = ""): AssetCatalog {
  return {
    project_path: projectPath,
    scanned_at: scannedAt,
    assets,
    total: assets.length,
    available: assets.filter((asset) => asset.state === "available").length,
    missing: assets.filter((asset) => asset.state === "missing").length,
    invalid: assets.filter((asset) => asset.state === "error").length
  };
}

type Props = { projectPath: string; kindFilter?: AssetKind };

export function AssetCatalogPanel({ projectPath, kindFilter }: Props) {
  const [assets, setAssets] = useState<Asset[]>([]);
  const [catalog, setCatalog] = useState<AssetCatalog | null>(null);
  const [query, setQuery] = useState("");
  const [includeMissing, setIncludeMissing] = useState(true);
  const [loading, setLoading] = useState(true);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadAssets = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setAssets(await assetList(projectPath));
    } catch (reason) {
      setError(getErrorMessage(reason));
    } finally {
      setLoading(false);
    }
  }, [projectPath]);

  useEffect(() => { void loadAssets(); }, [loadAssets]);

  async function handleScan() {
    setScanning(true);
    setError(null);
    try {
      const report = await assetScan(projectPath);
      setCatalog(report);
      setAssets(await assetList(projectPath));
    } catch (reason) {
      setError(getErrorMessage(reason));
    } finally {
      setScanning(false);
    }
  }

  const visibleAssets = useMemo(
    () => filterAssets(assets, { kind: kindFilter ?? "all", query, includeMissing }),
    [assets, includeMissing, kindFilter, query]
  );
  const totals = catalog ?? summarizeAssetCatalog(projectPath, assets);
  const heading = kindFilter ? `${assetKindLabels[kindFilter]}素材` : "Asset Catalog";

  return <section className="panel tab-panel asset-panel" role="tabpanel">
    <div className="section-heading">
      <div><span className="eyebrow">STEP 3 · ASSET CATALOG</span><h2>{heading}</h2><p>以專案相對路徑管理素材，不把檔案複製進私有資料庫。</p></div>
      <button className="primary" type="button" onClick={() => void handleScan()} disabled={scanning}><RefreshCw size={15} /> {scanning ? "Step 3: 掃描中…" : "Step 3: 掃描素材"}</button>
    </div>
    <div className="asset-summary" aria-live="polite"><span>總數 <strong>{totals.total}</strong></span><span>可用 <strong>{totals.available}</strong></span><span className={totals.missing ? "has-warning" : ""}>缺漏 <strong>{totals.missing}</strong></span><span>{catalog ? `最近掃描：${new Date(catalog.scanned_at).toLocaleString("zh-TW")}` : "尚未執行掃描"}</span></div>
    <div className="asset-controls">
      <label className="asset-search"><Search size={15} /><span className="sr-only">搜尋素材</span><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜尋檔名或相對路徑" /></label>
      <label className="asset-checkbox"><input type="checkbox" checked={includeMissing} onChange={(event) => setIncludeMissing(event.target.checked)} /> 顯示 missing</label>
      <button className="text-button" type="button" onClick={() => void loadAssets()} disabled={loading}>Step 3: 重新載入列表</button>
    </div>
    {error && <div className="error-banner" role="alert">{error}</div>}
    {loading ? <p className="inline-status">正在讀取 asset_list…</p> : visibleAssets.length === 0 ? <div className="asset-empty"><h3>找不到符合條件的素材</h3><p>可以先執行 Step 3 掃描，或清除搜尋／missing 篩選。</p></div> : <div className="asset-table-wrap">
      <table className="asset-table"><thead><tr><th>檔名</th><th>kind</th><th>size</th><th>hash 前 12 碼</th><th>state</th></tr></thead><tbody>{visibleAssets.map((asset) => <tr key={asset.id}><td><strong>{asset.display_name || getFileName(asset.relative_path)}</strong><small>{asset.relative_path}</small></td><td><span className="asset-kind">{assetKindLabels[asset.kind]}</span></td><td>{formatBytes(asset.size_bytes)}</td><td><code>{asset.sha256?.slice(0, 12) || "—"}</code></td><td><span className={`asset-state asset-state-${asset.state}`}>{asset.state}</span></td></tr>)}</tbody></table>
    </div>}
  </section>;
}

function getFileName(relativePath: string): string {
  return relativePath.split(/[\\/]/).pop() || relativePath;
}

export function formatBytes(value: number | null): string {
  if (value === null || value < 0) return "—";
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  if (value < 1024 * 1024 * 1024) return `${(value / (1024 * 1024)).toFixed(1)} MB`;
  return `${(value / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}
