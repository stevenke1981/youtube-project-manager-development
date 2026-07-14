import { invoke } from "@tauri-apps/api/core";
import type {
  CreateProjectRequest,
  Project,
  ProjectStatus,
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

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function listProjects(rootPath: string): Promise<Project[]> {
  if (!rootPath.trim()) return [];
  if (!isTauri()) return [...demoProjects];
  return invoke<Project[]>("project_list", { rootPath });
}

export async function createProject(
  rootPath: string,
  request: CreateProjectRequest
): Promise<Project> {
  if (!rootPath.trim() && isTauri()) {
    throw new Error("請先選擇影片 Library root");
  }
  if (!isTauri()) {
    return {
      ...demoProjects[0],
      id: crypto.randomUUID(),
      title: request.title,
      folder_name: `${new Date().toISOString().slice(0, 10)}_${request.title}`,
      channel: request.channel ?? null,
      progress: 0,
      status: "idea"
    };
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
