import {
  CalendarDays,
  Clapperboard,
  FolderKanban,
  LayoutDashboard,
  Settings,
  Sparkles,
  Workflow
} from "lucide-react";
import type { ReactNode } from "react";

type Props = {
  children: ReactNode;
  onNewProject: () => void;
  onChooseLibraryRoot: () => void;
  searchQuery: string;
  onSearchChange: (value: string) => void;
};

const nav = [
  [LayoutDashboard, "總覽", true],
  [FolderKanban, "影片專案", false],
  [CalendarDays, "發布日曆", false],
  [Clapperboard, "製作模板", false],
  [Workflow, "自動化", false]
] as const;

export function AppShell({ children, onNewProject, onChooseLibraryRoot, searchQuery, onSearchChange }: Props) {
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark"><Clapperboard size={20} /></span>
          <span><strong>YTPM</strong><small>Production Hub</small></span>
        </div>
        <nav aria-label="主要導覽">
          {nav.map(([Icon, label, active]) => (
            <button className={`nav-item ${active ? "active" : ""}`} key={label} type="button">
              <Icon size={19} /><span>{label}</span>
            </button>
          ))}
        </nav>
        <div className="sidebar-spacer" />
        <button className="nav-item" type="button"><Settings size={19} /><span>設定</span></button>
        <div className="storage-card">
          <Sparkles size={18} />
          <div><strong>離線優先</strong><small>素材保留在你的資料夾</small></div>
        </div>
      </aside>
      <main className="main">
        <header className="topbar">
          <label className="search" role="search">
            <span className="sr-only">搜尋影片、頻道或標籤</span>
            <input
              type="search"
              value={searchQuery}
              onChange={(event) => onSearchChange(event.target.value)}
              placeholder="搜尋影片、頻道或標籤…"
              aria-label="搜尋影片、頻道或標籤"
            />
            <kbd>Ctrl K</kbd>
          </label>
          <div className="topbar-actions">
            <button className="secondary" type="button" onClick={onChooseLibraryRoot}>1. 選擇 Library</button>
            <button className="primary" type="button" onClick={onNewProject}>2. 新增影片</button>
          </div>
        </header>
        {children}
      </main>
    </div>
  );
}
