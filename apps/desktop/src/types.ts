export type ProjectStatus =
  | "idea"
  | "research"
  | "script"
  | "voice"
  | "visuals"
  | "editing"
  | "subtitles"
  | "thumbnail"
  | "review"
  | "scheduled"
  | "published"
  | "archived";

export interface Project {
  schema_version: 2;
  id: string;
  title: string;
  folder_name: string;
  channel: string | null;
  series: string | null;
  status: ProjectStatus;
  archived_from_status: ProjectStatus | null;
  aspect_ratio: string;
  language: string;
  target_duration_seconds: number | null;
  planned_publish_at: string | null;
  published_at: string | null;
  progress: number;
  tags: string[];
  created_at: string;
  updated_at: string;
  app_version: string | null;
}

export interface CreateProjectRequest {
  title: string;
  channel?: string;
  series?: string;
  aspectRatio: string;
  language: string;
  targetDurationSeconds?: number;
  tags: string[];
}

export type ValidationSeverity = "info" | "warning" | "error";

export interface ValidationIssue {
  code: string;
  severity: ValidationSeverity;
  message: string;
  path: string | null;
  suggested_action: string | null;
}

export interface ValidationReport {
  valid: boolean;
  project: Project | null;
  issues: ValidationIssue[];
}

export type TaskStatus = "todo" | "doing" | "review" | "blocked" | "done";
export type TaskPriority = "low" | "normal" | "high" | "urgent";

export interface Task {
  id: string;
  title: string;
  description: string | null;
  status: TaskStatus;
  priority: TaskPriority;
  due_at: string | null;
  completed_at: string | null;
  related_asset_ids: string[];
  acceptance_criteria: string[];
  order_key: number;
  created_at: string;
  updated_at: string;
}

export interface TaskCreateRequest {
  title: string;
  description: string | null;
  priority: TaskPriority;
}

export type TaskUpdatePatch = Partial<Pick<Task, "title" | "description" | "priority" | "due_at" | "acceptance_criteria">>;

export type AssetKind =
  | "research"
  | "script"
  | "voice"
  | "music"
  | "sound_effect"
  | "image"
  | "video"
  | "subtitle"
  | "thumbnail"
  | "metadata"
  | "export"
  | "other";

export type AssetState = "available" | "missing" | "archived" | "processing" | "error";
export type AssetSourceType = "created" | "imported" | "generated" | "linked" | "downloaded";

export interface Asset {
  id: string;
  kind: AssetKind;
  relative_path: string;
  display_name: string | null;
  state: AssetState;
  source_type: AssetSourceType | null;
  generator: string | null;
  model: string | null;
  prompt: string | null;
  sha256: string | null;
  size_bytes: number | null;
  duration_ms: number | null;
  width: number | null;
  height: number | null;
  version_group_id: string | null;
  version_number: number;
  is_adopted: boolean;
  created_at: string;
  updated_at: string;
}

export interface AssetCatalog {
  project_path: string;
  scanned_at: string;
  assets: Asset[];
  total: number;
  available: number;
  missing: number;
  invalid: number;
}

export interface IndexReport {
  db_path: string;
  scanned: number;
  indexed: number;
  invalid: number;
  rebuilt_at?: string | null;
}

export interface RecoveryReport {
  recovered: boolean;
  had_journal: boolean;
  journal_path: string | null;
  message: string | null;
  actions: string[];
}

export type DocumentType = "script" | "publish";

export interface Document {
  relative_path: string;
  content: string;
  saved_at: string | null;
}

export interface DocumentWriteResult {
  saved_at: string;
}
