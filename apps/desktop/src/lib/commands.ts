import { invoke } from "@tauri-apps/api/core";
import type {
  Asset,
  AssetCatalog,
  CreateProjectRequest,
  DocumentWriteResult,
  IndexReport,
  Project,
  ProjectStatus,
  RecoveryReport,
  Task,
  TaskCreateRequest,
  TaskStatus,
  TaskUpdatePatch,
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
