import { useCallback, useEffect, useState } from "react";
import { ArrowLeft, CheckCircle2, Circle, Clipboard, FolderOpen, Mic2, Image, Captions, Video } from "lucide-react";
import { archiveProject, getErrorMessage, updateProjectStatus, validateProject } from "../lib/commands";
import type { Project, ProjectStatus, ValidationReport } from "../types";
import { StepGuide } from "./StepGuide";

const tabs = ["總覽", "任務", "研究", "腳本", "語音", "圖片", "影片", "字幕", "封面", "發布", "歷史", "設定"] as const;
type WorkspaceTab = typeof tabs[number];

const statusLabels: Record<ProjectStatus, string> = {
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

const statusOptions = (Object.keys(statusLabels) as ProjectStatus[]).filter((status) => status !== "archived");

const tabDescriptions: Record<WorkspaceTab, string> = {
  總覽: "查看進度、validation 結果與目前建議下一步。",
  任務: "把目前成果拆成可以逐項完成的工作。",
  研究: "整理研究來源與可供腳本使用的重點。",
  腳本: "確認腳本版本與下一個可交付段落。",
  語音: "管理旁白草稿與語音製作狀態。",
  圖片: "整理圖片、分鏡與視覺素材。",
  影片: "查看剪輯輸出與最終影片檔案。",
  字幕: "檢查字幕檔與時間軸成果。",
  封面: "準備縮圖與發布用封面。",
  發布: "確認發布資訊與上架前檢查。",
  歷史: "查看專案狀態與檔案變更紀錄。",
  設定: "查看這支影片的基本設定。"
};

type NextAction = {
  title: string;
  detail: string;
  buttonLabel: string;
  tab: WorkspaceTab;
  refreshValidation?: boolean;
};

export function getNextAction(project: Project, report: ValidationReport | null): NextAction {
  if (!report) {
    return {
      title: "先完成專案 validation",
      detail: "確認 project.json 與標準資料夾狀態，再開始下一個成果。",
      buttonLabel: "Step 3: 執行驗證",
      tab: "總覽",
      refreshValidation: true
    };
  }

  const issue = report.issues.find((item) => item.severity === "error" || item.severity === "warning");
  if (issue) {
    return {
      title: `先處理 ${report.issues.length} 個 validation 缺漏`,
      detail: issue.suggested_action || issue.message,
      buttonLabel: "Step 3: 查看缺漏",
      tab: "總覽"
    };
  }

  const actions: Partial<Record<ProjectStatus, Omit<NextAction, "refreshValidation">>> = {
    idea: { title: "開始研究影片主題", detail: "先整理研究資料，建立腳本可以依循的素材。", buttonLabel: "Step 3: 前往研究", tab: "研究" },
    research: { title: "完成第一版腳本", detail: "把研究重點整理成可錄製的段落。", buttonLabel: "Step 3: 前往腳本", tab: "腳本" },
    script: { title: "準備旁白語音", detail: "確認腳本版本後，進入語音製作。", buttonLabel: "Step 3: 前往語音", tab: "語音" },
    voice: { title: "補齊圖片與分鏡", detail: "旁白完成後，接著準備能支撐內容的視覺素材。", buttonLabel: "Step 3: 前往圖片", tab: "圖片" },
    visuals: { title: "組合影片初剪", detail: "素材齊備後，建立第一版影片輸出。", buttonLabel: "Step 3: 前往影片", tab: "影片" },
    editing: { title: "完成字幕成果", detail: "影片初剪完成後，檢查字幕是否與內容同步。", buttonLabel: "Step 3: 前往字幕", tab: "字幕" },
    subtitles: { title: "準備發布封面", detail: "字幕完成後，補齊縮圖與發布素材。", buttonLabel: "Step 3: 前往封面", tab: "封面" },
    thumbnail: { title: "進行發布前審核", detail: "確認影片、字幕與封面，再送進審核。", buttonLabel: "Step 3: 前往發布", tab: "發布" },
    review: { title: "完成發布前確認", detail: "逐項確認成果，決定是否進入待發布狀態。", buttonLabel: "Step 3: 前往發布", tab: "發布" }
  };

  return actions[project.status] ?? {
    title: "查看專案總覽",
    detail: "這支影片目前沒有待處理的下一步。",
    buttonLabel: "Step 3: 查看總覽",
    tab: "總覽"
  };
}

type Props = {
  project: Project;
  projectPath: string;
  onBack: () => void;
  onProjectUpdated: (project: Project) => void;
  onArchived: () => void;
};

export function ProjectWorkspace({ project, projectPath, onBack, onProjectUpdated, onArchived }: Props) {
  const [activeTab, setActiveTab] = useState<WorkspaceTab>("總覽");
  const [validationReport, setValidationReport] = useState<ValidationReport | null>(null);
  const [validationLoading, setValidationLoading] = useState(true);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [statusUpdating, setStatusUpdating] = useState(false);
  const [archiving, setArchiving] = useState(false);
  const [statusError, setStatusError] = useState<string | null>(null);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);

  const loadValidation = useCallback(async () => {
    setValidationLoading(true);
    setValidationError(null);
    try {
      const report = await validateProject(projectPath);
      setValidationReport(report);
      if (report.project) onProjectUpdated(report.project);
    } catch (reason) {
      setValidationError(getErrorMessage(reason));
    } finally {
      setValidationLoading(false);
    }
  }, [onProjectUpdated, projectPath]);

  useEffect(() => { void loadValidation(); }, [loadValidation]);

  async function handleStatusChange(nextStatus: ProjectStatus) {
    if (nextStatus === project.status) return;
    setStatusUpdating(true);
    setStatusError(null);
    setStatusMessage(null);
    try {
      const updated = await updateProjectStatus(projectPath, nextStatus);
      onProjectUpdated(updated);
      setStatusMessage(`狀態已更新為「${statusLabels[updated.status]}」`);
      await loadValidation();
    } catch (reason) {
      setStatusError(getErrorMessage(reason));
    } finally {
      setStatusUpdating(false);
    }
  }

  async function copyProjectPath() {
    try {
      await navigator.clipboard.writeText(projectPath);
      setStatusMessage("已複製專案路徑");
      setStatusError(null);
    } catch {
      setStatusError("無法複製路徑；請直接使用下方顯示的專案位置");
    }
  }

  async function handleArchive() {
    if (!window.confirm(`確定要封存「${project.title}」嗎？專案會移入 Library 的 _archive，不會刪除素材。`)) return;
    setArchiving(true);
    setStatusError(null);
    try {
      await archiveProject(projectPath);
      onArchived();
    } catch (reason) {
      setStatusError(getErrorMessage(reason));
    } finally {
      setArchiving(false);
    }
  }

  const nextAction = getNextAction(project, validationReport);
  const progress = Math.min(100, Math.max(0, project.progress));

  function handleNextAction() {
    setActiveTab(nextAction.tab);
    if (nextAction.refreshValidation) void loadValidation();
  }

  return (
    <div className="workspace">
      <div className="workspace-header">
        <button className="text-button" type="button" onClick={onBack}><ArrowLeft size={17} /> Step 2: 返回總覽</button>
        <div className="workspace-title">
          <div><span className="eyebrow">{project.channel || "未指定頻道"}</span><h1>{project.title}</h1><code className="project-path">{projectPath}</code></div>
          <div className="workspace-actions">
            <button className="secondary" type="button" onClick={copyProjectPath}><Clipboard size={16} /> 複製路徑</button>
            <button className="secondary danger-button" type="button" disabled={archiving} onClick={() => void handleArchive}>{archiving ? "封存中…" : "Step 3: 封存"}</button>
            <label className="status-control">目前狀態
              <select value={project.status} disabled={statusUpdating} onChange={(event) => void handleStatusChange(event.target.value as ProjectStatus)}>
                {statusOptions.map((status) => <option value={status} key={status}>{statusLabels[status]}</option>)}
              </select>
            </label>
          </div>
        </div>
        <nav className="workspace-tabs" aria-label="專案工作區分頁" role="tablist">
          {tabs.map((tab) => <button type="button" role="tab" aria-selected={activeTab === tab} className={activeTab === tab ? "active" : ""} key={tab} onClick={() => setActiveTab(tab)}>{tab}</button>)}
        </nav>
      </div>

      <div className="workspace-content">
        <StepGuide activeStep={3} className="workspace-guide" />
        {(statusError || statusMessage) && <div className={statusError ? "workspace-feedback error-banner" : "workspace-feedback success-banner"} role={statusError ? "alert" : "status"}>{statusError || statusMessage}</div>}
        <section className="next-action"><div><span>建議下一步</span><h2>{nextAction.title}</h2><p>{nextAction.detail}</p></div><button className="primary" type="button" onClick={handleNextAction}>{nextAction.buttonLabel}</button></section>

        {activeTab === "總覽" ? (
          <div className="workspace-columns">
            <section className="panel"><div className="section-heading"><div><h2>製作進度</h2><p>{progress}% 已完成 · 目前階段：{statusLabels[project.status]}</p></div></div><div className="large-progress"><span style={{ width: `${progress}%` }} /></div><Milestones progress={progress} /></section>
            <ValidationPanel report={validationReport} loading={validationLoading} error={validationError} onRetry={() => void loadValidation()} />
          </div>
        ) : (
          <section className="panel tab-panel" role="tabpanel">
            <span className="eyebrow">STEP 3 · {activeTab}</span>
            <h2>{activeTab}</h2>
            <p>{tabDescriptions[activeTab]}</p>
            <div className="tab-placeholder"><FolderOpen size={18} /><span>這個工作區已切換完成；素材仍留在專案資料夾，不會被複製到私有資料庫。</span></div>
            <button className="secondary" type="button" onClick={() => setActiveTab("總覽")}>Step 3: 回到總覽</button>
          </section>
        )}
      </div>
    </div>
  );
}

function Milestones({ progress }: { progress: number }) {
  const milestones = [
    { label: "研究資料", threshold: 15, icon: <Circle /> },
    { label: "腳本內容", threshold: 30, icon: <Circle /> },
    { label: "旁白語音", threshold: 45, icon: <Mic2 /> },
    { label: "圖片／分鏡", threshold: 60, icon: <Image /> },
    { label: "最終影片", threshold: 80, icon: <Video /> },
    { label: "字幕與發布素材", threshold: 100, icon: <Captions /> }
  ];

  return <ul className="deliverables">{milestones.map((milestone) => {
    const done = progress >= milestone.threshold;
    const active = !done && progress >= milestone.threshold - 15;
    return <li className={active ? "active" : ""} key={milestone.label}>{done ? <CheckCircle2 /> : milestone.icon}<span>{milestone.label}</span><small>{done ? "進度已達" : active ? "下一個里程碑" : "待處理"}</small></li>;
  })}</ul>;
}

function ValidationPanel({ report, loading, error, onRetry }: { report: ValidationReport | null; loading: boolean; error: string | null; onRetry: () => void }) {
  return <section className="panel validation-panel"><div className="section-heading"><div><h2>專案驗證</h2><p>project_validate 的實際結果</p></div><button className="text-button" type="button" onClick={onRetry}>Step 3: 重新驗證</button></div>
    {loading && <p className="inline-status">正在檢查 project.json 與標準資料夾…</p>}
    {error && <div className="error-banner" role="alert">{error}<button className="secondary" type="button" onClick={onRetry}>Step 3: 重試驗證</button></div>}
    {report && <>
      <div className={`check-row ${report.valid ? "ok" : "problem"}`}><CheckCircle2 /> {report.valid ? "project.json 與必要結構可讀取" : "validation 發現錯誤，請先修正"}</div>
      {report.issues.length === 0 ? <p className="validation-clear">目前沒有缺漏或警告。</p> : <ul className="validation-issues">{report.issues.map((issue) => <li key={`${issue.code}-${issue.path ?? "root"}`}><span className={`issue-severity ${issue.severity}`}>{issue.severity}</span><div><strong>{issue.message}</strong>{issue.path && <code>{issue.path}</code>}{issue.suggested_action && <small>下一步：{issue.suggested_action}</small>}</div></li>)}</ul>}
    </>}
  </section>;
}
