import { useCallback, useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { AppShell } from "./components/AppShell";
import { CreateProjectDialog } from "./components/CreateProjectDialog";
import { Dashboard } from "./components/Dashboard";
import { ProjectWorkspace } from "./components/ProjectWorkspace";
import { StepGuide } from "./components/StepGuide";
import { createProject, getErrorMessage, listProjects, projectIndexRebuild } from "./lib/commands";
import type { CreateProjectRequest, IndexReport, Project } from "./types";

type IndexStatus = "idle" | "indexing" | "ready" | "error";

export default function App() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [selected, setSelected] = useState<Project | null>(null);
  const [rootPath, setRootPath] = useState(() => localStorage.getItem("ytpm.libraryRoot") ?? "");
  const [searchQuery, setSearchQuery] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [indexStatus, setIndexStatus] = useState<IndexStatus>("idle");
  const [indexReport, setIndexReport] = useState<IndexReport | null>(null);
  const [indexError, setIndexError] = useState<string | null>(null);
  const indexRequestRef = useRef(0);

  const loadProjects = useCallback(async () => {
    if (!rootPath.trim()) {
      setProjects([]);
      setLoading(false);
      setIndexStatus("idle");
      setIndexReport(null);
      return;
    }
    const requestId = indexRequestRef.current + 1;
    indexRequestRef.current = requestId;
    setLoading(true);
    setError(null);
    setIndexError(null);
    setIndexStatus("indexing");
    try {
      const report = await projectIndexRebuild(rootPath);
      if (requestId !== indexRequestRef.current) return;
      setIndexReport(report);
      setIndexStatus("ready");
      const nextProjects = await listProjects(rootPath);
      if (requestId !== indexRequestRef.current) return;
      setProjects(nextProjects);
      setSelected((current) => current ? nextProjects.find((project) => project.id === current.id) ?? null : null);
    } catch (reason) {
      if (requestId !== indexRequestRef.current) return;
      setIndexStatus("error");
      setIndexError(getErrorMessage(reason));
      setError(getErrorMessage(reason));
    } finally {
      if (requestId === indexRequestRef.current) setLoading(false);
    }
  }, [rootPath]);

  useEffect(() => { void loadProjects(); }, [loadProjects]);

  async function handleChooseRoot() {
    setError(null);
    try {
      const selectedRoot = isTauri()
        ? await open({ directory: true, multiple: false, title: "選擇影片 Library" })
        : rootPath.trim() || "demo-library";
      if (typeof selectedRoot !== "string" || !selectedRoot.trim()) return;
      localStorage.setItem("ytpm.libraryRoot", selectedRoot);
      setRootPath(selectedRoot.trim());
      setSelected(null);
      setSearchQuery("");
    } catch (reason) {
      setError(getErrorMessage(reason));
    }
  }

  function handleNewProject() {
    if (!rootPath.trim()) {
      setError("請先選擇影片 Library，完成 Step 1 後才能建立專案");
      void handleChooseRoot();
      return;
    }
    setError(null);
    setDialogOpen(true);
  }

  async function handleCreate(request: CreateProjectRequest) {
    setBusy(true);
    setError(null);
    try {
      const project = await createProject(rootPath, request);
      setProjects((current) => [project, ...current.filter((item) => item.id !== project.id)]);
      setDialogOpen(false);
      setSelected(project);
    } catch (reason) {
      setError(getErrorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  const handleProjectUpdated = useCallback((project: Project) => {
    setProjects((current) => current.map((item) => item.id === project.id ? project : item));
    setSelected((current) => current?.id === project.id ? project : current);
  }, []);

  const projectPath = selected ? joinProjectPath(rootPath, selected.folder_name) : "";

  return (
    <AppShell
      searchQuery={searchQuery}
      onSearchChange={setSearchQuery}
      onNewProject={handleNewProject}
      onChooseLibraryRoot={handleChooseRoot}
    >
      {error && <div className="global-error" role="alert"><span>{error}</span>{rootPath && <button className="secondary" type="button" onClick={() => void loadProjects()}>Step 1: 重新載入 Library</button>}</div>}
      {!rootPath ? (
        <section className="empty-state">
          <StepGuide activeStep={1} className="empty-guide" />
          <span className="eyebrow">OFFLINE LIBRARY</span>
          <h1>Step 1: 先選擇影片 Library</h1>
          <p>專案資料夾會留在你選擇的位置，不會複製到 App 私有資料庫。</p>
          <button className="primary" type="button" onClick={handleChooseRoot}>1. 選擇 Library root</button>
        </section>
      ) : loading && !selected ? (
        <section className="status-state" role="status"><span className="eyebrow">LOADING OFFLINE INDEX</span><h1>正在載入 Library</h1><p>正在讀取可攜式 project.json；資料仍留在你的資料夾。</p></section>
      ) : selected ? (
        <ProjectWorkspace rootPath={rootPath} project={selected} projectPath={projectPath} onBack={() => setSelected(null)} onProjectUpdated={handleProjectUpdated} onArchived={() => { setSelected(null); void loadProjects(); }} />
      ) : (
        <Dashboard projects={projects} searchQuery={searchQuery} onSearchChange={setSearchQuery} onOpen={setSelected} onNewProject={handleNewProject} indexStatus={indexStatus} indexReport={indexReport} indexError={indexError} onRebuildIndex={() => void loadProjects()} />
      )}
      <CreateProjectDialog open={dialogOpen} rootPath={rootPath} busy={busy} error={error} onClose={() => setDialogOpen(false)} onSubmit={handleCreate} />
    </AppShell>
  );
}

function joinProjectPath(rootPath: string, folderName: string): string {
  return `${rootPath.replace(/[\\/]+$/, "")}\\${folderName}`;
}

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
