import { useCallback, useEffect, useMemo, useState, type FormEvent } from "react";
import { Plus, RefreshCw } from "lucide-react";
import { getErrorMessage, taskCreate, taskList, taskMove, taskUpdate } from "../lib/commands";
import type { Task, TaskCreateRequest, TaskPriority, TaskStatus } from "../types";

export const TASK_COLUMNS: ReadonlyArray<{ status: TaskStatus; label: string; helper: string }> = [
  { status: "todo", label: "待處理", helper: "尚未開始" },
  { status: "doing", label: "進行中", helper: "正在製作" },
  { status: "review", label: "待審核", helper: "需要確認" },
  { status: "blocked", label: "已阻塞", helper: "需要排除" },
  { status: "done", label: "已完成", helper: "已驗收" }
];

export const taskStatusLabels: Record<TaskStatus, string> = Object.fromEntries(
  TASK_COLUMNS.map((column) => [column.status, column.label])
) as Record<TaskStatus, string>;

export const taskPriorityLabels: Record<TaskPriority, string> = {
  low: "低",
  normal: "普通",
  high: "高",
  urgent: "緊急"
};

export function groupTasksByStatus(tasks: Task[]): Record<TaskStatus, Task[]> {
  const grouped: Record<TaskStatus, Task[]> = {
    todo: [],
    doing: [],
    review: [],
    blocked: [],
    done: []
  };
  tasks.forEach((task) => grouped[task.status].push(task));
  TASK_COLUMNS.forEach(({ status }) => {
    grouped[status].sort((left, right) => (left.order_key - right.order_key) || left.title.localeCompare(right.title, "zh-TW"));
  });
  return grouped;
}

export function getNextTaskOrder(tasks: Task[], status: TaskStatus): number {
  const orders = tasks.filter((task) => task.status === status).map((task) => task.order_key);
  return orders.length ? Math.max(...orders) + 1 : 0;
}

type Props = { projectPath: string };
type Draft = { title: string; description: string };

export function TaskBoard({ projectPath }: Props) {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [priority, setPriority] = useState<TaskPriority>("normal");
  const [editingTaskId, setEditingTaskId] = useState<string | null>(null);
  const [draft, setDraft] = useState<Draft>({ title: "", description: "" });

  const loadTasks = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setTasks(await taskList(projectPath));
    } catch (reason) {
      setError(getErrorMessage(reason));
    } finally {
      setLoading(false);
    }
  }, [projectPath]);

  useEffect(() => { void loadTasks(); }, [loadTasks]);

  const grouped = useMemo(() => groupTasksByStatus(tasks), [tasks]);

  async function handleCreate(event: FormEvent) {
    event.preventDefault();
    const trimmedTitle = title.trim();
    if (!trimmedTitle) {
      setError("請輸入任務標題，才能新增任務");
      return;
    }
    setBusy("create");
    setError(null);
    const request: TaskCreateRequest = { title: trimmedTitle, description: description.trim() || null, priority };
    try {
      const created = await taskCreate(projectPath, request);
      setTasks((current) => [...current, created]);
      setTitle("");
      setDescription("");
      setPriority("normal");
    } catch (reason) {
      setError(getErrorMessage(reason));
    } finally {
      setBusy(null);
    }
  }

  function beginEdit(task: Task) {
    setEditingTaskId(task.id);
    setDraft({ title: task.title, description: task.description ?? "" });
    setError(null);
  }

  async function saveEdit(task: Task) {
    const nextTitle = draft.title.trim();
    if (!nextTitle) {
      setError("任務標題不可為空");
      return;
    }
    setBusy(`edit:${task.id}`);
    setError(null);
    try {
      const updated = await taskUpdate(projectPath, task.id, {
        title: nextTitle,
        description: draft.description.trim() || null
      });
      setTasks((current) => current.map((item) => item.id === updated.id ? updated : item));
      setEditingTaskId(null);
    } catch (reason) {
      setError(getErrorMessage(reason));
    } finally {
      setBusy(null);
    }
  }

  async function moveTask(task: Task, status: TaskStatus) {
    if (status === task.status) return;
    setBusy(`move:${task.id}`);
    setError(null);
    try {
      const updated = await taskMove(projectPath, task.id, status, getNextTaskOrder(tasks, status));
      setTasks((current) => current.map((item) => item.id === updated.id ? updated : item));
    } catch (reason) {
      setError(getErrorMessage(reason));
    } finally {
      setBusy(null);
    }
  }

  return (
    <section className="panel tab-panel task-panel" role="tabpanel">
      <div className="section-heading">
        <div><span className="eyebrow">STEP 3 · TASKS</span><h2>任務 Kanban</h2><p>新增、編輯並移動任務，狀態會寫回可攜式專案資料。</p></div>
        <button className="secondary" type="button" onClick={() => void loadTasks()} disabled={loading}><RefreshCw size={15} /> Step 3: 重新載入</button>
      </div>

      <form className="task-create-form" onSubmit={(event) => void handleCreate(event)}>
        <div className="task-form-fields">
          <label>任務標題<input value={title} onChange={(event) => setTitle(event.target.value)} placeholder="例如：完成第一版旁白" /></label>
          <label>描述<textarea value={description} onChange={(event) => setDescription(event.target.value)} placeholder="補充完成條件或上下文" rows={2} /></label>
          <label>優先級<select value={priority} onChange={(event) => setPriority(event.target.value as TaskPriority)}>{Object.entries(taskPriorityLabels).map(([value, label]) => <option value={value} key={value}>{label}</option>)}</select></label>
        </div>
        <button className="primary" type="submit" disabled={busy === "create"}><Plus size={15} /> {busy === "create" ? "Step 3: 新增中…" : "Step 3: 新增任務"}</button>
      </form>

      {error && <div className="error-banner task-error" role="alert">{error}</div>}
      {loading ? <p className="inline-status">正在讀取 tasks.json／索引任務…</p> : (
        <div className="task-columns" aria-label="任務 Kanban 欄位">
          {TASK_COLUMNS.map((column) => (
            <section className={`task-column task-column-${column.status}`} key={column.status}>
              <header className="task-column-header"><div><h3>{column.label}</h3><small>{column.helper}</small></div><strong>{grouped[column.status].length}</strong></header>
              <div className="task-list">
                {grouped[column.status].length === 0 && <p className="task-empty">目前沒有任務</p>}
                {grouped[column.status].map((task) => (
                  <TaskCard
                    key={task.id}
                    task={task}
                    editing={editingTaskId === task.id}
                    draft={draft}
                    busy={busy}
                    onBeginEdit={() => beginEdit(task)}
                    onCancelEdit={() => setEditingTaskId(null)}
                    onDraftChange={setDraft}
                    onSave={() => void saveEdit(task)}
                    onMove={(status) => void moveTask(task, status)}
                  />
                ))}
              </div>
            </section>
          ))}
        </div>
      )}
    </section>
  );
}

function TaskCard({
  task,
  editing,
  draft,
  busy,
  onBeginEdit,
  onCancelEdit,
  onDraftChange,
  onSave,
  onMove
}: {
  task: Task;
  editing: boolean;
  draft: Draft;
  busy: string | null;
  onBeginEdit: () => void;
  onCancelEdit: () => void;
  onDraftChange: (draft: Draft) => void;
  onSave: () => void;
  onMove: (status: TaskStatus) => void;
}) {
  return <article className="task-card">
    {editing ? (
      <div className="task-edit-form">
        <label>標題<input value={draft.title} onChange={(event) => onDraftChange({ ...draft, title: event.target.value })} /></label>
        <label>描述<textarea value={draft.description} onChange={(event) => onDraftChange({ ...draft, description: event.target.value })} rows={3} /></label>
        <div className="task-card-actions"><button className="primary" type="button" onClick={onSave} disabled={busy === `edit:${task.id}`}>{busy === `edit:${task.id}` ? "Step 3: 儲存中…" : "Step 3: 儲存"}</button><button className="text-button" type="button" onClick={onCancelEdit}>Step 3: 取消</button></div>
      </div>
    ) : <>
      <div className="task-card-header"><span className={`task-priority task-priority-${task.priority}`}>{taskPriorityLabels[task.priority]}</span><button className="text-button" type="button" onClick={onBeginEdit}>Step 3: 編輯</button></div>
      <h4>{task.title}</h4>
      {task.description && <p>{task.description}</p>}
      <div className="task-card-footer">
        <label className="task-move">移動<select aria-label={`移動任務「${task.title}」`} value={task.status} disabled={busy === `move:${task.id}`} onChange={(event) => onMove(event.target.value as TaskStatus)}>{TASK_COLUMNS.map((column) => <option value={column.status} key={column.status}>{column.label}</option>)}</select></label>
        <small>{new Date(task.updated_at).toLocaleDateString("zh-TW")}</small>
      </div>
    </>}
  </article>;
}
