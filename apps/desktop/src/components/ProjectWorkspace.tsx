import { ArrowLeft, CheckCircle2, Circle, FolderOpen, Mic2, Image, Captions, Video } from "lucide-react";
import type { Project } from "../types";

const tabs = ["總覽", "任務", "研究", "腳本", "語音", "圖片", "影片", "字幕", "封面", "發布", "歷史", "設定"];

export function ProjectWorkspace({ project, onBack }: { project: Project; onBack: () => void }) {
  return (
    <div className="workspace">
      <div className="workspace-header">
        <button className="text-button" onClick={onBack}><ArrowLeft size={17} /> 返回總覽</button>
        <div className="workspace-title"><div><span className="eyebrow">{project.channel || "未指定頻道"}</span><h1>{project.title}</h1></div><button className="secondary"><FolderOpen size={16} /> 開啟資料夾</button></div>
        <div className="workspace-tabs">{tabs.map((tab, index) => <button className={index === 0 ? "active" : ""} key={tab}>{tab}</button>)}</div>
      </div>
      <div className="workspace-content">
        <section className="next-action"><div><span>建議下一步</span><h2>完成旁白語音並確認段落長度</h2><p>腳本已完成，語音資料夾目前有 3 個草稿。</p></div><button className="primary">前往語音</button></section>
        <div className="workspace-columns">
          <section className="panel"><div className="section-heading"><div><h2>製作進度</h2><p>{project.progress}% 已完成</p></div></div><div className="large-progress"><span style={{ width: `${project.progress}%` }} /></div><ul className="deliverables"><Deliverable done label="研究資料" /><Deliverable done label="完整腳本" /><Deliverable icon={<Mic2 />} label="旁白語音" active /><Deliverable icon={<Image />} label="圖片／分鏡" /><Deliverable icon={<Video />} label="最終影片" /><Deliverable icon={<Captions />} label="字幕" /></ul></section>
          <section className="panel"><div className="section-heading"><div><h2>發布檢查</h2><p>必要成果與缺漏</p></div></div><div className="check-row ok"><CheckCircle2 /> 腳本已選定最終版本</div><div className="check-row"><Circle /> 尚未指定最終影片</div><div className="check-row"><Circle /> 尚未指定封面</div><div className="check-row"><Circle /> 字幕檔不存在</div></section>
        </div>
      </div>
    </div>
  );
}

function Deliverable({ label, done = false, active = false, icon }: { label: string; done?: boolean; active?: boolean; icon?: React.ReactNode }) {
  return <li className={active ? "active" : ""}>{done ? <CheckCircle2 /> : icon ?? <Circle />}<span>{label}</span><small>{done ? "完成" : active ? "進行中" : "待處理"}</small></li>;
}
