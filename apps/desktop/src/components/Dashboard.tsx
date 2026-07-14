import { useMemo, useState } from "react";
import { AlertTriangle, CalendarClock, CheckCircle2, Film, Plus } from "lucide-react";
import type { IndexReport, Project, ProjectStatus } from "../types";
import { ProjectCard } from "./ProjectCard";
import { StepGuide } from "./StepGuide";

export type DashboardSort = "recent" | "progress" | "title";
export type DashboardStatusFilter = "all" | ProjectStatus;

export const statusLabels: Record<ProjectStatus, string> = {
  idea: "構想",
  research: "研究中",
  script: "腳本",
  voice: "語音製作",
  visuals: "視覺素材",
  editing: "影片製作",
  subtitles: "字幕",
  thumbnail: "封面",
  review: "待審核",
  scheduled: "待發布",
  published: "已發布",
  archived: "已封存"
};

export const projectStatuses = Object.keys(statusLabels) as ProjectStatus[];

export function filterAndSortProjects(
  projects: Project[],
  options: { query: string; status: DashboardStatusFilter; sort: DashboardSort }
): Project[] {
  const query = options.query.trim().toLocaleLowerCase("zh-TW");
  const filtered = projects.filter((project) => {
    const matchesQuery = !query || [project.title, project.channel ?? "", ...project.tags]
      .join(" ")
      .toLocaleLowerCase("zh-TW")
      .includes(query);
    const matchesStatus = options.status === "all" || project.status === options.status;
    return matchesQuery && matchesStatus;
  });

  return [...filtered].sort((left, right) => {
    if (options.sort === "progress") {
      return right.progress - left.progress || compareRecent(left, right);
    }
    if (options.sort === "title") {
      return left.title.localeCompare(right.title, "zh-TW");
    }
    return compareRecent(left, right);
  });
}

function compareRecent(left: Project, right: Project): number {
  return toTimestamp(right.updated_at) - toTimestamp(left.updated_at);
}

function toTimestamp(value: string): number {
  const timestamp = new Date(value).getTime();
  return Number.isNaN(timestamp) ? 0 : timestamp;
}

export function getProjectMetrics(projects: Project[], now = new Date()) {
  const weekStart = new Date(now);
  weekStart.setHours(0, 0, 0, 0);
  weekStart.setDate(weekStart.getDate() - ((weekStart.getDay() + 6) % 7));
  const weekEnd = new Date(weekStart);
  weekEnd.setDate(weekEnd.getDate() + 7);

  return {
    active: projects.filter((project) => !["published", "archived"].includes(project.status)).length,
    scheduledThisWeek: projects.filter((project) => {
      if (!project.planned_publish_at) return false;
      const planned = new Date(project.planned_publish_at);
      return planned >= weekStart && planned < weekEnd;
    }).length,
    review: projects.filter((project) => project.status === "review").length,
    attention: projects.filter((project) => !["published", "archived"].includes(project.status) && project.progress < 100).length
  };
}

export type DashboardEmptyState = "no-projects" | "no-results" | null;

export function getDashboardEmptyState(totalProjects: number, visibleProjects: number): DashboardEmptyState {
  if (totalProjects === 0) return "no-projects";
  if (visibleProjects === 0) return "no-results";
  return null;
}

type Props = {
  projects: Project[];
  searchQuery: string;
  onSearchChange: (value: string) => void;
  onOpen: (project: Project) => void;
  onNewProject: () => void;
  indexStatus: "idle" | "indexing" | "ready" | "error";
  indexReport: IndexReport | null;
  indexError: string | null;
  onRebuildIndex: () => void;
};

export function Dashboard({ projects, searchQuery, onSearchChange, onOpen, onNewProject, indexStatus, indexReport, indexError, onRebuildIndex }: Props) {
  const [statusFilter, setStatusFilter] = useState<DashboardStatusFilter>("all");
  const [sort, setSort] = useState<DashboardSort>("recent");
  const visibleProjects = useMemo(
    () => filterAndSortProjects(projects, { query: searchQuery, status: statusFilter, sort }),
    [projects, searchQuery, sort, statusFilter]
  );
  const emptyState = getDashboardEmptyState(projects.length, visibleProjects.length);
  const metrics = getProjectMetrics(projects);
  const hasFilters = Boolean(searchQuery.trim()) || statusFilter !== "all";

  function clearFilters() {
    setStatusFilter("all");
    onSearchChange("");
  }

  return (
    <div className="content">
      <section className="page-heading">
        <div>
          <span className="eyebrow">PRODUCTION OVERVIEW</span>
          <h1>影片製作總覽</h1>
          <p>掌握每支影片的下一步、缺漏與發布時程。</p>
        </div>
        <div className={`index-status index-status-${indexStatus}`} role="status">
          <div><strong>{indexStatus === "indexing" ? "索引建立中…" : indexStatus === "error" ? "索引錯誤" : indexStatus === "ready" ? "索引已就緒" : "尚未建立索引"}</strong><small>{indexReport ? `掃描 ${indexReport.scanned} · 索引 ${indexReport.indexed} · invalid ${indexReport.invalid}` : `${projects.length} 個專案`}</small>{indexError && <small className="index-error">{indexError}</small>}</div>
          <button className="secondary" type="button" onClick={onRebuildIndex} disabled={indexStatus === "indexing"}>Step 1: 重建索引</button>
        </div>
      </section>

      <StepGuide activeStep={2} className="dashboard-guide" />

      <section className="metrics" aria-label="製作統計">
        <Metric icon={<Film />} value={metrics.active} label="進行中" note={`${projects.length} 個專案的實際狀態`} />
        <Metric icon={<CalendarClock />} value={metrics.scheduledThisWeek} label="本週發布" note="依預計發布日期計算" />
        <Metric icon={<CheckCircle2 />} value={metrics.review} label="待審核" note="需要你的確認" />
        <Metric icon={<AlertTriangle />} value={metrics.attention} label="需要處理" note="尚未完成且未封存" danger />
      </section>

      <section className="section-heading">
        <div><h2>接續製作</h2><p>{searchQuery ? `搜尋「${searchQuery}」的結果` : "最近修改的影片專案"}</p></div>
        <button className="secondary" type="button" onClick={onNewProject}><Plus size={15} /> Step 2: 建立影片</button>
      </section>

      <section className="project-controls" aria-label="專案篩選與排序">
        <label>
          狀態
          <select value={statusFilter} onChange={(event) => setStatusFilter(event.target.value as DashboardStatusFilter)}>
            <option value="all">全部狀態</option>
            {projectStatuses.map((status) => <option value={status} key={status}>{statusLabels[status]}</option>)}
          </select>
        </label>
        <label>
          排序
          <select value={sort} onChange={(event) => setSort(event.target.value as DashboardSort)}>
            <option value="recent">最近修改</option>
            <option value="progress">進度最高</option>
            <option value="title">標題 A-Z</option>
          </select>
        </label>
        {hasFilters && <button className="text-button" type="button" onClick={clearFilters}>清除狀態篩選</button>}
      </section>

      {emptyState ? (
        <section className="empty-state compact" aria-live="polite">
          <span className="eyebrow">{emptyState === "no-projects" ? "EMPTY LIBRARY" : "NO MATCHES"}</span>
          <h2>{emptyState === "no-projects" ? "Library 目前還沒有影片" : "找不到符合條件的專案"}</h2>
          <p>{emptyState === "no-projects" ? "現在建立第一支影片，接著就能在工作區追蹤每一步。" : "試試其他標題、頻道、標籤或狀態；目前沒有專案被刪除。"}</p>
          {emptyState === "no-projects" ? (
            <button className="primary" type="button" onClick={onNewProject}>Step 2: 建立第一支影片</button>
          ) : (
            <button className="secondary" type="button" onClick={clearFilters}>清除篩選條件</button>
          )}
        </section>
      ) : (
        <section className="project-grid" aria-label="影片專案列表">
          {visibleProjects.map((project) => <ProjectCard key={project.id} project={project} onOpen={() => onOpen(project)} />)}
        </section>
      )}
    </div>
  );
}

function Metric({ icon, value, label, note, danger = false }: { icon: React.ReactNode; value: number; label: string; note: string; danger?: boolean }) {
  return <article className={`metric ${danger ? "danger" : ""}`}><span className="metric-icon">{icon}</span><div><strong>{value}</strong><span>{label}</span><small>{note}</small></div></article>;
}
