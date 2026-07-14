import { describe, expect, it } from "vitest";
import { filterAndSortProjects, getDashboardEmptyState, getProjectMetrics } from "./components/Dashboard";
import { getStepState, WORKFLOW_STEPS } from "./components/StepGuide";
import { getErrorMessage } from "./lib/commands";
import type { Project } from "./types";

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
});
