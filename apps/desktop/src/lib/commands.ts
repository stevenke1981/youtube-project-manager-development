import { invoke } from "@tauri-apps/api/core";
import type { CreateProjectRequest, Project } from "../types";

const demoProjects: Project[] = [
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
  return "__TAURI_INTERNALS__" in window;
}

export async function listProjects(rootPath: string): Promise<Project[]> {
  if (!rootPath.trim()) return [];
  if (!isTauri()) return demoProjects;
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
