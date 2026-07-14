import { AlertTriangle, CalendarClock, CheckCircle2, Film } from "lucide-react";
import type { Project } from "../types";
import { ProjectCard } from "./ProjectCard";

export function Dashboard({ projects, onOpen }: { projects: Project[]; onOpen: (project: Project) => void }) {
  const review = projects.filter((p) => p.status === "review").length;
  const active = projects.filter((p) => !["published", "archived"].includes(p.status)).length;

  return (
    <div className="content">
      <section className="page-heading">
        <div><span className="eyebrow">PRODUCTION OVERVIEW</span><h1>影片製作總覽</h1><p>掌握每支影片的下一步、缺漏與發布時程。</p></div>
        <span className="date-chip">2026 年 7 月</span>
      </section>
      <section className="metrics" aria-label="製作統計">
        <Metric icon={<Film />} value={active} label="進行中" note="跨所有頻道" />
        <Metric icon={<CalendarClock />} value={3} label="本週發布" note="下一支：7/18" />
        <Metric icon={<CheckCircle2 />} value={review} label="待審核" note="需要你的確認" />
        <Metric icon={<AlertTriangle />} value={4} label="需要處理" note="缺字幕或封面" danger />
      </section>
      <section className="section-heading"><div><h2>接續製作</h2><p>最近修改的影片專案</p></div><button className="secondary">查看全部</button></section>
      <section className="project-grid">
        {projects.map((project) => <ProjectCard key={project.id} project={project} onOpen={() => onOpen(project)} />)}
      </section>
    </div>
  );
}

function Metric({ icon, value, label, note, danger = false }: { icon: React.ReactNode; value: number; label: string; note: string; danger?: boolean }) {
  return <article className={`metric ${danger ? "danger" : ""}`}><span className="metric-icon">{icon}</span><div><strong>{value}</strong><span>{label}</span><small>{note}</small></div></article>;
}
