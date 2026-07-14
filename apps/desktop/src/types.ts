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
