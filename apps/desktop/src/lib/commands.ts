import { invoke } from "@tauri-apps/api/core";
import type {
  Asset,
  AssetCatalog,
  CreateProjectRequest,
  DocumentWriteResult,
  IndexReport,
  MediaExportRequest,
  MediaExportResult,
  MediaMetadata,
  PublishConfigReference,
  PublishMetadata,
  PublishOAuthCallbackResult,
  PublishOAuthStart,
  PublishReadiness,
  PublishResult,
  Project,
  ProjectStatus,
  RecoveryReport,
  Task,
  TaskCreateRequest,
  TaskStatus,
  TaskUpdatePatch,
  Timeline,
  ValidationReport
} from "../types";

export type CommandErrorPayload = {
  code?: string;
  human_message?: string;
  technical_detail?: string | null;
  message?: string;
  recoverable?: boolean;
  suggested_action?: string | null;
};

export function getErrorMessage(reason: unknown): string {
  if (reason instanceof Error && reason.message) return reason.message;
  if (typeof reason === "string" && reason.trim()) return reason;
  if (typeof reason === "object" && reason !== null) {
    const payload = reason as CommandErrorPayload;
    const message = payload.human_message || payload.message;
    if (message) {
      return payload.suggested_action
        ? `${message}（建議：${payload.suggested_action}）`
        : message;
    }
  }
  return "發生未知錯誤，請檢查操作與資料夾權限後重試。";
}

let demoProjects: Project[] = [
  {
    schema_version: 2,
    id: "demo-1",
    title: "大模型為什麼不適合做精確計算",
    folder_name: "2026-07-14_大模型為什麼不適合做精確計算",
    channel: "AI 技術分享",
    series: "AI 基礎",
    status: "voice",
    archived_from_status: null,
    aspect_ratio: "16:9",
    language: "zh-TW",
    target_duration_seconds: 480,
    planned_publish_at: "2026-07-20T12:00:00Z",
    published_at: null,
    progress: 45,
    tags: ["AI", "科普"],
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    app_version: "0.1.0"
  },
  {
    schema_version: 2,
    id: "demo-2",
    title: "OpenWrt 家庭網路防護完整指南",
    folder_name: "2026-07-13_OpenWrt 家庭網路防護完整指南",
    channel: "實用電腦技術",
    series: null,
    status: "review",
    archived_from_status: null,
    aspect_ratio: "16:9",
    language: "zh-TW",
    target_duration_seconds: 720,
    planned_publish_at: "2026-07-18T12:00:00Z",
    published_at: null,
    progress: 86,
    tags: ["OpenWrt", "網路"],
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    app_version: "0.1.0"
  }
];

const demoTasks = new Map<string, Task[]>();
const demoAssets = new Map<string, Asset[]>();
const demoDocuments = new Map<string, string>();
const demoTimelines = new Map<string, Timeline>();
const demoPublishMetadata = new Map<string, PublishMetadata>();

function nowIso(): string {
  return new Date().toISOString();
}

function makeId(): string {
  return typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `demo-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function pathKey(value: string): string {
  return value.trim().replace(/[\\/]+/g, "\\").toLocaleLowerCase();
}

function cloneTask(task: Task): Task {
  return { ...task, related_asset_ids: [...task.related_asset_ids], acceptance_criteria: [...task.acceptance_criteria] };
}

function cloneTimeline(timeline: Timeline): Timeline {
  return {
    ...timeline,
    tracks: timeline.tracks.map((track) => ({
      ...track,
      clips: track.clips.map((clip) => ({ ...clip }))
    }))
  };
}

function clonePublishMetadata(metadata: PublishMetadata): PublishMetadata {
  return { ...metadata, tags: [...metadata.tags] };
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, milliseconds));
}

function getDemoTasks(projectPath: string): Task[] {
  const key = pathKey(projectPath);
  const existing = demoTasks.get(key);
  if (existing) return existing;
  const createdAt = nowIso();
  const initial: Task[] = [{
    id: makeId(),
    title: "確認下一個可交付成果",
    description: "把這支影片目前的阻塞事項拆成可驗收的小步驟。",
    status: "todo",
    priority: "normal",
    due_at: null,
    completed_at: null,
    related_asset_ids: [],
    acceptance_criteria: [],
    order_key: 0,
    created_at: createdAt,
    updated_at: createdAt
  }];
  demoTasks.set(key, initial);
  return initial;
}

function getDemoAssets(projectPath: string): Asset[] {
  const key = pathKey(projectPath);
  const existing = demoAssets.get(key);
  if (existing) return existing;
  const createdAt = nowIso();
  const initial: Asset[] = [
    {
      id: makeId(),
      kind: "script",
      relative_path: "02_script/script.md",
      display_name: "script.md",
      state: "available",
      source_type: "created",
      generator: null,
      model: null,
      prompt: null,
      sha256: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
      size_bytes: 4280,
      duration_ms: null,
      width: null,
      height: null,
      version_group_id: null,
      version_number: 1,
      is_adopted: true,
      created_at: createdAt,
      updated_at: createdAt
    },
    {
      id: makeId(),
      kind: "image",
      relative_path: "04_visuals/storyboard_01.png",
      display_name: "storyboard_01.png",
      state: "missing",
      source_type: "generated",
      generator: "demo",
      model: "demo-image",
      prompt: null,
      sha256: "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd",
      size_bytes: 284000,
      duration_ms: null,
      width: 1280,
      height: 720,
      version_group_id: null,
      version_number: 1,
      is_adopted: false,
      created_at: createdAt,
      updated_at: createdAt
    },
    {
      id: makeId(),
      kind: "subtitle",
      relative_path: "07_subtitles/subtitles.srt",
      display_name: "subtitles.srt",
      state: "available",
      source_type: "created",
      generator: null,
      model: null,
      prompt: null,
      sha256: null,
      size_bytes: 912,
      duration_ms: null,
      width: null,
      height: null,
      version_group_id: null,
      version_number: 1,
      is_adopted: false,
      created_at: createdAt,
      updated_at: createdAt
    },
    {
      id: makeId(),
      kind: "video",
      relative_path: "05_video/rough-cut.mp4",
      display_name: "rough-cut.mp4",
      state: "available",
      source_type: "created",
      generator: null,
      model: null,
      prompt: null,
      sha256: null,
      size_bytes: 48_000_000,
      duration_ms: 420_000,
      width: 1920,
      height: 1080,
      version_group_id: null,
      version_number: 1,
      is_adopted: true,
      created_at: createdAt,
      updated_at: createdAt
    },
    {
      id: makeId(),
      kind: "voice",
      relative_path: "03_voice/narration.wav",
      display_name: "narration.wav",
      state: "available",
      source_type: "created",
      generator: null,
      model: null,
      prompt: null,
      sha256: null,
      size_bytes: 12_000_000,
      duration_ms: 420_000,
      width: null,
      height: null,
      version_group_id: null,
      version_number: 1,
      is_adopted: true,
      created_at: createdAt,
      updated_at: createdAt
    }
  ];
  demoAssets.set(key, initial);
  return initial;
}

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function listProjects(rootPath: string): Promise<Project[]> {
  if (!rootPath.trim()) return [];
  return projectIndexSearch(rootPath);
}

export async function projectIndexRebuild(rootPath: string): Promise<IndexReport> {
  if (!rootPath.trim()) throw new Error("請先選擇影片 Library root，才能重建索引");
  if (!isTauri()) {
    return {
      db_path: `${rootPath.replace(/[\\/]+$/, "")}\\.ytpm\\index.db`,
      scanned: demoProjects.length,
      indexed: demoProjects.length,
      invalid: 0,
      rebuilt_at: nowIso()
    };
  }
  return invoke<IndexReport>("project_index_rebuild", { rootPath });
}

export async function projectIndexSearch(rootPath: string, query?: string, status?: ProjectStatus): Promise<Project[]> {
  if (!rootPath.trim()) return [];
  if (!isTauri()) {
    const normalizedQuery = query?.trim().toLocaleLowerCase("zh-TW") ?? "";
    return demoProjects.filter((project) => {
      const searchable = [project.title, project.channel ?? "", project.series ?? "", ...project.tags]
        .join(" ")
        .toLocaleLowerCase("zh-TW");
      return (!normalizedQuery || searchable.includes(normalizedQuery)) && (!status || project.status === status);
    }).map((project) => ({ ...project }));
  }
  await projectRecoverJournal(rootPath);
  return invoke<Project[]>("project_index_search", {
    rootPath,
    ...(query?.trim() ? { query: query.trim() } : {}),
    ...(status ? { status } : {})
  });
}

export async function createProject(
  rootPath: string,
  request: CreateProjectRequest
): Promise<Project> {
  if (!rootPath.trim() && isTauri()) {
    throw new Error("請先選擇影片 Library root");
  }
  if (!isTauri()) {
    const createdAt = nowIso();
    const project = {
      ...demoProjects[0],
      id: makeId(),
      title: request.title,
      folder_name: `${createdAt.slice(0, 10)}_${request.title}`,
      channel: request.channel ?? null,
      progress: 0,
      status: "idea" as const,
      created_at: createdAt,
      updated_at: createdAt
    };
    demoProjects = [project, ...demoProjects];
    return project;
  }
  return invoke<Project>("project_create", { rootPath, request });
}

export async function validateProject(projectPath: string): Promise<ValidationReport> {
  if (!projectPath.trim()) {
    throw new Error("找不到專案路徑，請先選擇一個影片專案");
  }
  if (!isTauri()) {
    const project = demoProjects.find((item) => projectPath.includes(item.folder_name)) ?? null;
    return { valid: true, project, issues: [] };
  }
  return invoke<ValidationReport>("project_validate", { projectPath });
}

export async function updateProjectStatus(
  projectPath: string,
  status: ProjectStatus
): Promise<Project> {
  if (!projectPath.trim()) {
    throw new Error("找不到專案路徑，無法更新影片狀態");
  }
  if (!isTauri()) {
    const project = demoProjects.find((item) => projectPath.includes(item.folder_name));
    if (!project) throw new Error("示範模式找不到這個影片專案");
    const updated = { ...project, status, updated_at: new Date().toISOString() };
    demoProjects = demoProjects.map((item) => item.id === updated.id ? updated : item);
    return updated;
  }
  return invoke<Project>("project_update_status", { projectPath, status });
}

export async function archiveProject(projectPath: string): Promise<Project> {
  if (!projectPath.trim()) {
    throw new Error("找不到專案路徑，無法封存影片");
  }
  if (!isTauri()) {
    const project = demoProjects.find((item) => projectPath.includes(item.folder_name));
    if (!project) throw new Error("示範模式找不到這個影片專案");
    const archived = {
      ...project,
      status: "archived" as const,
      archived_from_status: project.status,
      updated_at: new Date().toISOString()
    };
    demoProjects = demoProjects.filter((item) => item.id !== project.id);
    return archived;
  }
  return invoke<Project>("project_archive", { projectPath });
}

export async function taskList(projectPath: string): Promise<Task[]> {
  if (!projectPath.trim()) throw new Error("找不到專案路徑，無法讀取任務");
  if (!isTauri()) return getDemoTasks(projectPath).map(cloneTask);
  return invoke<Task[]>("task_list", { projectPath });
}

export async function taskCreate(projectPath: string, request: TaskCreateRequest): Promise<Task> {
  if (!projectPath.trim()) throw new Error("找不到專案路徑，無法建立任務");
  if (!isTauri()) {
    const tasks = getDemoTasks(projectPath);
    const createdAt = nowIso();
    const task: Task = {
      id: makeId(),
      title: request.title,
      description: request.description,
      status: "todo",
      priority: request.priority,
      due_at: null,
      completed_at: null,
      related_asset_ids: [],
      acceptance_criteria: [],
      order_key: tasks.length ? Math.max(...tasks.map((item) => item.order_key)) + 1 : 0,
      created_at: createdAt,
      updated_at: createdAt
    };
    tasks.push(task);
    return cloneTask(task);
  }
  return invoke<Task>("task_create", { projectPath, request });
}

export async function taskUpdate(projectPath: string, taskId: string, patch: TaskUpdatePatch): Promise<Task> {
  if (!projectPath.trim()) throw new Error("找不到專案路徑，無法更新任務");
  if (!isTauri()) {
    const tasks = getDemoTasks(projectPath);
    const task = tasks.find((item) => item.id === taskId);
    if (!task) throw new Error("示範模式找不到這個任務");
    Object.assign(task, patch, { updated_at: nowIso() });
    return cloneTask(task);
  }
  return invoke<Task>("task_update", { projectPath, taskId, patch });
}

export async function taskMove(projectPath: string, taskId: string, status: TaskStatus, orderKey: number): Promise<Task> {
  if (!projectPath.trim()) throw new Error("找不到專案路徑，無法移動任務");
  if (!isTauri()) {
    const tasks = getDemoTasks(projectPath);
    const task = tasks.find((item) => item.id === taskId);
    if (!task) throw new Error("示範模式找不到這個任務");
    Object.assign(task, {
      status,
      order_key: orderKey,
      completed_at: status === "done" ? nowIso() : null,
      updated_at: nowIso()
    });
    return cloneTask(task);
  }
  return invoke<Task>("task_move", { projectPath, taskId, status, orderKey });
}

export async function assetScan(projectPath: string): Promise<AssetCatalog> {
  if (!projectPath.trim()) throw new Error("找不到專案路徑，無法掃描素材");
  if (!isTauri()) {
    const assets = getDemoAssets(projectPath);
    return {
      project_path: projectPath,
      scanned_at: nowIso(),
      assets: assets.map((asset) => ({ ...asset })),
      total: assets.length,
      available: assets.filter((asset) => asset.state === "available").length,
      missing: assets.filter((asset) => asset.state === "missing").length,
      invalid: assets.filter((asset) => asset.state === "error").length
    };
  }
  return invoke<AssetCatalog>("asset_scan", { projectPath });
}

export async function assetList(projectPath: string): Promise<Asset[]> {
  if (!projectPath.trim()) throw new Error("找不到專案路徑，無法讀取素材");
  if (!isTauri()) return getDemoAssets(projectPath).map((asset) => ({ ...asset }));
  return invoke<Asset[]>("asset_list", { projectPath });
}

export async function documentRead(projectPath: string, relativePath: string): Promise<string> {
  if (!projectPath.trim()) throw new Error("找不到專案路徑，無法讀取文件");
  if (!relativePath.trim()) throw new Error("文件路徑不可為空");
  if (!isTauri()) {
    const key = `${pathKey(projectPath)}::${relativePath}`;
    const existing = demoDocuments.get(key);
    if (existing !== undefined) return existing;
    const project = demoProjects.find((item) => projectPath.includes(item.folder_name));
    const initial = relativePath === "02_script/script.md"
      ? `# ${project?.title ?? "影片腳本"}\n\n## 開場\n\n在這裡整理第一版腳本。\n`
      : `# ${project?.title ?? "發布描述"}\n\n在這裡撰寫 YouTube 描述與發布資訊。\n`;
    demoDocuments.set(key, initial);
    return initial;
  }
  return invoke<string>("document_read", { projectPath, relativePath });
}

export async function documentWrite(projectPath: string, relativePath: string, content: string): Promise<DocumentWriteResult> {
  if (!projectPath.trim()) throw new Error("找不到專案路徑，無法儲存文件");
  if (!relativePath.trim()) throw new Error("文件路徑不可為空");
  if (!isTauri()) {
    demoDocuments.set(`${pathKey(projectPath)}::${relativePath}`, content);
    return { saved_at: nowIso() };
  }
  return invoke<DocumentWriteResult>("document_write", { projectPath, relativePath, content });
}

export async function projectRecoverJournal(rootPath: string): Promise<RecoveryReport> {
  if (!rootPath.trim()) throw new Error("請先選擇影片 Library root，才能檢查 journal");
  if (!isTauri()) {
    return {
      recovered: false,
      had_journal: false,
      journal_path: null,
      message: "沒有待恢復的操作 journal。",
      actions: []
    };
  }
  return invoke<RecoveryReport>("project_recover_journal", { rootPath });
}

export function isTauriRuntime(): boolean {
  return isTauri();
}

function createDemoTimeline(projectPath: string): Timeline {
  const assets = getDemoAssets(projectPath);
  const video = assets.find((asset) => asset.kind === "video") ?? assets[0];
  const voice = assets.find((asset) => asset.kind === "voice") ?? assets[0];
  const now = nowIso();
  const clip = (asset: Asset | undefined, label: string, start_ms: number, in_ms: number, out_ms: number) => ({
    id: makeId(),
    asset_id: asset?.id ?? makeId(),
    relative_path: asset?.relative_path ?? "05_video/rough-cut.mp4",
    label,
    start_ms,
    in_ms,
    out_ms,
    duration_ms: out_ms - in_ms,
    volume: 1,
    muted: false,
    transition: null
  });
  return {
    schema_version: 1,
    duration_ms: 420_000,
    updated_at: now,
    tracks: [
      {
        id: "00000000-0000-0000-0000-000000000001",
        label: "V1 · 主畫面",
        kind: "video",
        clips: [
          clip(video, "開場與主畫面", 0, 0, 120_000),
          clip(video, "重點示範", 130_000, 120_000, 290_000)
        ]
      },
      {
        id: "00000000-0000-0000-0000-000000000002",
        label: "A1 · 旁白",
        kind: "audio",
        clips: [clip(voice, "旁白主軌", 0, 0, 300_000)]
      },
      {
        id: "00000000-0000-0000-0000-000000000003",
        label: "S1 · 字幕",
        kind: "subtitle",
        clips: []
      }
    ],
    output: {
      output_relative_path: "09_exports/timeline.mp4",
      format: "mp4",
      width: 1920,
      height: 1080,
      frame_rate: 30
    }
  };
}

export async function timelineLoad(projectPath: string): Promise<Timeline> {
  if (!projectPath.trim()) throw new Error("找不到專案路徑，無法載入 timeline");
  if (!isTauri()) {
    const key = pathKey(projectPath);
    const existing = demoTimelines.get(key);
    if (existing) return cloneTimeline(existing);
    const created = createDemoTimeline(projectPath);
    demoTimelines.set(key, created);
    return cloneTimeline(created);
  }
  return invoke<Timeline>("timeline_load", { projectPath });
}

export async function timelineSave(projectPath: string, timeline: Timeline): Promise<Timeline> {
  if (!projectPath.trim()) throw new Error("找不到專案路徑，無法儲存 timeline");
  if (!isTauri()) {
    const saved = { ...cloneTimeline(timeline), updated_at: nowIso() };
    demoTimelines.set(pathKey(projectPath), saved);
    return cloneTimeline(saved);
  }
  return invoke<Timeline>("timeline_save", { projectPath, timeline });
}

export async function mediaProbe(
  projectPath: string,
  assetId: string | null,
  relativePath: string
): Promise<MediaMetadata> {
  if (!projectPath.trim()) throw new Error("找不到專案路徑，無法 probe 素材");
  if (!relativePath.trim()) throw new Error("請先選擇要 probe 的素材");
  if (!isTauri()) {
    const asset = getDemoAssets(projectPath).find((item) => item.id === assetId || item.relative_path === relativePath);
    await delay(350);
    return {
      asset_id: asset?.id ?? assetId,
      relative_path: relativePath,
      format_name: getExtension(relativePath) === "wav" ? "wav" : "mov,mp4,m4a,3gp,3g2,mj2",
      duration_seconds: asset?.duration_ms ? asset.duration_ms / 1000 : 420,
      size_bytes: asset?.size_bytes ?? null,
      bitrate_bps: 2_400_000,
      width: asset?.width ?? (asset?.kind === "voice" ? null : 1920),
      height: asset?.height ?? (asset?.kind === "voice" ? null : 1080),
      video_codec: asset?.kind === "voice" ? null : "h264",
      audio_codec: asset?.kind === "voice" ? "pcm_s16le" : "aac",
      frame_rate: asset?.kind === "voice" ? null : "30/1",
      sample_rate: asset?.kind === "voice" ? 48000 : 48000,
      channels: asset?.kind === "voice" ? 2 : 2,
      probed_at: nowIso()
    };
  }
  return invoke<MediaMetadata>("media_probe", { projectPath, assetId, relativePath });
}

export async function mediaExport(projectPath: string, request: MediaExportRequest, confirm = false): Promise<MediaExportResult> {
  if (!projectPath.trim()) throw new Error("找不到專案路徑，無法匯出媒體");
  if (!request.output_relative_path.trim()) throw new Error("輸出路徑不可為空");
  if (!isTauri()) {
    await delay(650);
    return {
      operation_id: makeId(),
      status: "completed",
      progress: 100,
      output_relative_path: request.output_relative_path,
      message: "Web demo 已完成本地匯出流程預覽，未執行 FFmpeg。"
    };
  }
  return invoke<MediaExportResult>("media_export", { projectPath, request, confirm });
}

export async function mediaOperationCancel(
  projectPath: string,
  operationId: string,
  kind: "probe" | "export"
): Promise<void> {
  if (!projectPath.trim()) throw new Error("找不到專案路徑，無法取消媒體操作");
  if (!isTauri()) return;
  await invoke<void>("media_operation_cancel", { projectPath, operationId, kind });
}

function getExtension(relativePath: string): string {
  return relativePath.split(/[\\.]/).pop()?.toLocaleLowerCase() ?? "";
}

export async function publishConfigReference(projectPath: string): Promise<PublishConfigReference> {
  if (!projectPath.trim()) throw new Error("找不到專案路徑，無法讀取 OAuth 設定參照");
  if (!isTauri()) {
    return {
      provider: "YouTube Data API v3",
      config_path: ".ytpm/publish/oauth.json",
      oauth_ready: false,
      scopes: ["youtube.upload", "youtube.readonly"],
      setup_url: "https://console.cloud.google.com/apis/credentials"
    };
  }
  return invoke<PublishConfigReference>("publish_config_reference", { projectPath });
}

export async function publishAuthStart(): Promise<PublishOAuthStart> {
  if (!isTauri()) throw new Error("OAuth loopback 需要在 Tauri desktop host 執行");
  return invoke<PublishOAuthStart>("publish_auth_start");
}

export async function publishAuthCallback(callbackUrl: string, expectedState: string, codeVerifier: string): Promise<PublishOAuthCallbackResult> {
  if (!isTauri()) throw new Error("OAuth callback 需要在 Tauri desktop host 執行");
  return invoke<PublishOAuthCallbackResult>("publish_auth_callback", { request: { callbackUrl, expectedState, codeVerifier } });
}

export async function publishMetadataLoad(projectPath: string, project: Project): Promise<PublishMetadata> {
  if (!projectPath.trim()) throw new Error("找不到專案路徑，無法讀取發布 metadata");
  if (!isTauri()) {
    const key = pathKey(projectPath);
    const existing = demoPublishMetadata.get(key);
    if (existing) return clonePublishMetadata(existing);
    const description = await documentRead(projectPath, "08_metadata/description.md");
    const created: PublishMetadata = {
      title: project.title,
      description,
      tags: [...project.tags],
      visibility: "private",
      scheduled_at: project.planned_publish_at,
      channel: project.channel
    };
    demoPublishMetadata.set(key, created);
    return clonePublishMetadata(created);
  }
  return invoke<PublishMetadata>("publish_metadata_load", { projectPath });
}

export async function publishMetadataSave(projectPath: string, metadata: PublishMetadata): Promise<PublishMetadata> {
  if (!projectPath.trim()) throw new Error("找不到專案路徑，無法儲存發布 metadata");
  if (!metadata.title.trim()) throw new Error("發布標題不可為空");
  if (!isTauri()) {
    const saved = clonePublishMetadata(metadata);
    demoPublishMetadata.set(pathKey(projectPath), saved);
    await documentWrite(projectPath, "08_metadata/description.md", metadata.description);
    return clonePublishMetadata(saved);
  }
  return invoke<PublishMetadata>("publish_metadata_save", { projectPath, metadata });
}

export async function publishDryRun(projectPath: string, metadata: PublishMetadata): Promise<PublishResult> {
  if (!projectPath.trim()) throw new Error("找不到專案路徑，無法執行發布 dry-run");
  if (!isTauri()) {
    await delay(450);
    return {
      operation_id: makeId(),
      status: "completed",
      progress: 100,
      dry_run: true,
      uploaded: false,
      video_url: null,
      message: `Dry-run 完成：已檢查「${metadata.title || "未命名影片"}」，未連線、未上傳。`
    };
  }
  return invoke<PublishResult>("publish_dry_run", { projectPath, metadata });
}

export async function publishUpload(projectPath: string, metadata: PublishMetadata, confirm = false): Promise<PublishResult> {
  if (!projectPath.trim()) throw new Error("找不到專案路徑，無法發布影片");
  if (!isTauri()) {
    return {
      operation_id: makeId(),
      status: "failed",
      progress: 0,
      dry_run: false,
      uploaded: false,
      video_url: null,
      message: "Web demo fallback 不會真的上傳影片；請在 Tauri host 完成 OAuth 與 publish_upload command。"
    };
  }
  return invoke<PublishResult>("publish_upload", { projectPath, metadata, confirm });
}

export async function publishCancel(projectPath: string, operationId: string): Promise<void> {
  if (!projectPath.trim()) throw new Error("找不到專案路徑，無法取消發布");
  if (!isTauri()) return;
  await invoke<void>("publish_cancel", { projectPath, operationId });
}
