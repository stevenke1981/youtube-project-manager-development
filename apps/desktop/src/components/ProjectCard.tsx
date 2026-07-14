import { Calendar, FolderOpen, MoreHorizontal } from "lucide-react";
import type { Project } from "../types";

const labels: Record<Project["status"], string> = {
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

export function ProjectCard({ project, onOpen }: { project: Project; onOpen: () => void }) {
  return (
    <article className="project-card">
      <div className={`thumbnail status-${project.status}`}>
        <div className="thumbnail-overlay">
          <span>{project.aspect_ratio}</span>
          <button aria-label="更多操作"><MoreHorizontal size={18} /></button>
        </div>
        <strong>{project.title.slice(0, 18)}</strong>
      </div>
      <div className="card-body">
        <div className="status-row">
          <span className={`status-badge status-${project.status}`}>{labels[project.status]}</span>
          <span className="progress-label">{project.progress}%</span>
        </div>
        <h3>{project.title}</h3>
        <p>{project.channel || "未指定頻道"}</p>
        <div className="progress"><span style={{ width: `${project.progress}%` }} /></div>
        <div className="meta-row">
          <span><Calendar size={15} /> {project.planned_publish_at ? new Date(project.planned_publish_at).toLocaleDateString("zh-TW") : "未排程"}</span>
          <button className="text-button" onClick={onOpen}><FolderOpen size={15} /> 開啟</button>
        </div>
      </div>
    </article>
  );
}
