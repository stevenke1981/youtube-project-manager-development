import { describe, expect, it } from "vitest";
import { AssetCatalogPanel, filterAssets, formatBytes, summarizeAssetCatalog } from "./components/AssetCatalogPanel";
import { filterAndSortProjects, getDashboardEmptyState, getProjectMetrics } from "./components/Dashboard";
import { getDocumentPath, getEditorNextStep, getSaveStatusLabel } from "./components/DocumentEditor";
import { getNextTaskOrder, groupTasksByStatus } from "./components/TaskBoard";
import { getStepState, WORKFLOW_STEPS } from "./components/StepGuide";
import { getErrorMessage } from "./lib/commands";
import type { Asset, Project, Task } from "./types";

function project(overrides: Partial<Project> = {}): Project {
  return {
    schema_version: 2,
    id: "project-id",
    title: "示範影片",
    folder_name: "2026-07-15_示範影片",
    channel: "測試頻道",
    series: null,
    status: "idea",
    archived_from_status: null,
    aspect_ratio: "16:9",
    language: "zh-TW",
    target_duration_seconds: 300,
    planned_publish_at: null,
    published_at: null,
    progress: 0,
    tags: ["AI", "教學"],
    created_at: "2026-07-15T08:00:00.000Z",
    updated_at: "2026-07-15T08:00:00.000Z",
    app_version: "0.1.0",
    ...overrides
  };
}

function task(overrides: Partial<Task> = {}): Task {
  return {
    id: "task-id",
    title: "任務",
    description: null,
    status: "todo",
    priority: "normal",
    due_at: null,
    completed_at: null,
    related_asset_ids: [],
    acceptance_criteria: [],
    order_key: 0,
    created_at: "2026-07-15T08:00:00.000Z",
    updated_at: "2026-07-15T08:00:00.000Z",
    ...overrides
  };
}

function asset(overrides: Partial<Asset> = {}): Asset {
  return {
    id: "asset-id",
    kind: "image",
    relative_path: "04_visuals/scene.png",
    display_name: "scene.png",
    state: "available",
    source_type: "generated",
    generator: null,
    model: null,
    prompt: null,
    sha256: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    size_bytes: 2048,
    duration_ms: null,
    width: 1280,
    height: 720,
    version_group_id: null,
    version_number: 1,
    is_adopted: false,
    created_at: "2026-07-15T08:00:00.000Z",
    updated_at: "2026-07-15T08:00:00.000Z",
    ...overrides
  };
}

describe("YTPM desktop workflow UI", () => {
  it("keeps the main workflow numbered and ordered", () => {
    expect(WORKFLOW_STEPS.map((step) => `${step.number}. ${step.title}`)).toEqual([
      "1. 選擇 Library",
      "2. 建立影片專案",
      "3. 完成下一個成果"
    ]);
    expect(getStepState(1, 3)).toBe("complete");
    expect(getStepState(3, 3)).toBe("current");
    expect(getStepState(2, 1)).toBe("upcoming");
  });

  it("searches title, channel, and tags and applies status filters", () => {
    const projects = [
      project({ id: "ai", title: "AI 旁白工作流", tags: ["語音", "AI"] }),
      project({ id: "network", title: "家庭網路設定", channel: "網路頻道", status: "review", tags: ["OpenWrt"] })
    ];
    expect(filterAndSortProjects(projects, { query: "openwrt", status: "all", sort: "recent" }).map((item) => item.id)).toEqual(["network"]);
    expect(filterAndSortProjects(projects, { query: "", status: "review", sort: "recent" }).map((item) => item.id)).toEqual(["network"]);
  });

  it("sorts projects by progress and exposes a clear no-result state", () => {
    const projects = [
      project({ id: "low", title: "低進度", progress: 20, updated_at: "2026-07-15T10:00:00.000Z" }),
      project({ id: "high", title: "高進度", progress: 80, updated_at: "2026-07-15T09:00:00.000Z" })
    ];
    expect(filterAndSortProjects(projects, { query: "", status: "all", sort: "progress" }).map((item) => item.id)).toEqual(["high", "low"]);
    expect(getDashboardEmptyState(projects.length, 0)).toBe("no-results");
    expect(getDashboardEmptyState(0, 0)).toBe("no-projects");
  });

  it("calculates dashboard metrics from project data rather than fixed values", () => {
    const projects = [
      project({ id: "active", status: "script", progress: 35, planned_publish_at: "2026-07-14T12:00:00.000Z" }),
      project({ id: "review", status: "review", progress: 90, planned_publish_at: "2026-07-18T12:00:00.000Z" }),
      project({ id: "published", status: "published", progress: 100 }),
      project({ id: "archived", status: "archived", progress: 100 })
    ];
    expect(getProjectMetrics(projects, new Date("2026-07-15T12:00:00.000Z"))).toEqual({
      active: 2,
      scheduledThisWeek: 2,
      review: 1,
      attention: 2
    });
  });

  it("treats a fully complete project as no longer needing attention", () => {
    expect(getProjectMetrics([project({ progress: 100, status: "review" })]).attention).toBe(0);
  });

  it("turns structured Tauri errors into actionable human-readable text", () => {
    expect(getErrorMessage({
      code: "FILESYSTEM_ERROR",
      human_message: "無法建立專案",
      suggested_action: "選擇可寫入的 Library root"
    })).toBe("無法建立專案（建議：選擇可寫入的 Library root）");
  });

  it("groups all five task columns and preserves order_key", () => {
    const grouped = groupTasksByStatus([
      task({ id: "b", status: "doing", order_key: 2 }),
      task({ id: "a", status: "doing", order_key: 1 }),
      task({ id: "done", status: "done", order_key: 0 })
    ]);
    expect(Object.keys(grouped)).toEqual(["todo", "doing", "review", "blocked", "done"]);
    expect(grouped.doing.map((item) => item.id)).toEqual(["a", "b"]);
    expect(grouped.blocked).toEqual([]);
  });

  it("calculates the next task order inside a target column", () => {
    expect(getNextTaskOrder([task({ status: "review", order_key: 4 }), task({ status: "review", order_key: 9 })], "review")).toBe(10);
    expect(getNextTaskOrder([task({ status: "todo", order_key: 4 })], "done")).toBe(0);
  });

  it("maps editor tabs to the portable document paths", () => {
    expect(getDocumentPath("script")).toBe("02_script/script.md");
    expect(getDocumentPath("publish")).toBe("08_metadata/description.md");
  });

  it("exposes explicit autosave status labels", () => {
    expect(getSaveStatusLabel("saving")).toBe("Saving…");
    expect(getSaveStatusLabel("saved")).toBe("Saved");
    expect(getSaveStatusLabel("error")).toBe("Error");
  });

  it("numbers editor next steps for both script and publish flows", () => {
    expect(getEditorNextStep("script")).toMatch(/^Next step:/);
    expect(getEditorNextStep("publish")).toMatch(/^Next step:/);
  });

  it("filters assets by kind, query, and missing state", () => {
    const assets = [
      asset({ id: "image", kind: "image", relative_path: "image/scene.png" }),
      asset({ id: "missing", kind: "image", state: "missing", display_name: "missing.png", relative_path: "image/missing.png" }),
      asset({ id: "voice", kind: "voice", relative_path: "03_audio/voice.wav" })
    ];
    expect(filterAssets(assets, { kind: "image", query: "scene" }).map((item) => item.id)).toEqual(["image"]);
    expect(filterAssets(assets, { kind: "image", includeMissing: false }).map((item) => item.id)).toEqual(["image"]);
  });

  it("summarizes asset catalog availability and missing counts", () => {
    const catalog = summarizeAssetCatalog("D:\\demo", [asset({ id: "ok" }), asset({ id: "missing", state: "missing" }), asset({ id: "error", state: "error" })], "2026-07-15T08:00:00.000Z");
    expect(catalog).toMatchObject({ total: 3, available: 1, missing: 1, invalid: 1, project_path: "D:\\demo" });
  });

  it("formats asset sizes for a readable catalog table", () => {
    expect(formatBytes(null)).toBe("—");
    expect(formatBytes(2048)).toBe("2.0 KB");
    expect(formatBytes(1024 * 1024)).toBe("1.0 MB");
  });

  it("keeps the asset catalog component export available for the workspace surface", () => {
    expect(AssetCatalogPanel).toBeTypeOf("function");
  });
});
