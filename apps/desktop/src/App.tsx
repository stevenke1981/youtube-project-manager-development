import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { AppShell } from "./components/AppShell";
import { CreateProjectDialog } from "./components/CreateProjectDialog";
import { Dashboard } from "./components/Dashboard";
import { ProjectWorkspace } from "./components/ProjectWorkspace";
import { createProject, listProjects } from "./lib/commands";
import type { CreateProjectRequest, Project } from "./types";

export default function App() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [selected, setSelected] = useState<Project | null>(null);
  const [rootPath, setRootPath] = useState(() => localStorage.getItem("ytpm.libraryRoot") ?? "");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!rootPath) {
      setProjects([]);
      return;
    }
    listProjects(rootPath).then(setProjects).catch((reason: unknown) => {
      setError(reason instanceof Error ? reason.message : String(reason));
    });
  }, [rootPath]);

  async function handleChooseRoot() {
    setError(null);
    try {
      const selectedRoot = isTauri()
        ? await open({ directory: true, multiple: false, title: "選擇影片 Library" })
        : window.prompt("輸入影片 Library root 路徑", rootPath);
      if (typeof selectedRoot !== "string" || !selectedRoot.trim()) return;
      localStorage.setItem("ytpm.libraryRoot", selectedRoot);
      setRootPath(selectedRoot);
      setSelected(null);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  async function handleCreate(request: CreateProjectRequest) {
    setBusy(true);
    setError(null);
    try {
      const project = await createProject(rootPath, request);
      setProjects((current) => [project, ...current]);
      setDialogOpen(false);
      setSelected(project);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  }

  return (
    <AppShell onNewProject={() => setDialogOpen(true)} onChooseLibraryRoot={handleChooseRoot}>
      {error && <div className="global-error" role="alert">{error}</div>}
      {!rootPath ? (
        <section className="empty-state">
          <span className="eyebrow">OFFLINE LIBRARY</span>
          <h1>先選擇影片 Library</h1>
          <p>專案資料夾會留在你選擇的位置，不會複製到 App 私有資料庫。</p>
          <button className="primary" onClick={handleChooseRoot}>選擇 Library root</button>
        </section>
      ) : selected ? (
        <ProjectWorkspace project={selected} onBack={() => setSelected(null)} />
      ) : (
        <Dashboard projects={projects} onOpen={setSelected} />
      )}
      <CreateProjectDialog open={dialogOpen} rootPath={rootPath} busy={busy} error={error} onClose={() => setDialogOpen(false)} onSubmit={handleCreate} />
    </AppShell>
  );
}

function isTauri(): boolean {
  return "__TAURI_INTERNALS__" in window;
}
