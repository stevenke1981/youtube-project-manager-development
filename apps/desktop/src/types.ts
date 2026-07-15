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

export type TimelineTrackKind = "video" | "audio" | "subtitle" | "other";

export interface TimelineTransition {
  kind: string;
  duration_ms: number;
}

export type TimelineClipEffect =
  | {
      kind: "color_adjust";
      brightness: number;
      contrast: number;
      saturation: number;
      gamma: number;
    }
  | { kind: "blur"; radius: number }
  | { kind: "sharpen"; amount: number }
  | { kind: "vignette"; angle: number }
  | { kind: "chroma_key"; color: string; similarity: number; blend: number }
  | { kind: "fade_in"; duration_ms: number }
  | { kind: "fade_out"; duration_ms: number }
  | {
      kind: "transform";
      x: number;
      y: number;
      scale: number;
      rotation_degrees: number;
      opacity: number;
    };

export interface TimelineSubtitleStyle {
  font_family: string;
  font_size: number;
  primary_color: string;
  outline_color: string;
  background_color: string;
  bold: boolean;
  italic: boolean;
  outline_width: number;
  shadow_depth: number;
  margin_left: number;
  margin_right: number;
  margin_vertical: number;
  alignment: number;
}

export interface TimelineClip {
  id: string;
  asset_id: string;
  relative_path: string;
  label: string;
  start_ms: number;
  in_ms: number;
  out_ms: number;
  duration_ms: number;
  volume: number;
  muted: boolean;
  transition: TimelineTransition | null;
  effects: TimelineClipEffect[];
}

export interface TimelineTrack {
  id: string;
  label: string;
  kind: TimelineTrackKind;
  clips: TimelineClip[];
}

export interface Timeline {
  schema_version: 1 | 2;
  duration_ms: number;
  tracks: TimelineTrack[];
  output: {
    output_relative_path: string;
    format: "mp4";
    width: number;
    height: number;
    frame_rate: number;
    subtitle_style: TimelineSubtitleStyle;
  };
  updated_at: string;
}

export type TimelineIssueSeverity = "error" | "warning";

export interface TimelineValidationIssue {
  code: string;
  severity: TimelineIssueSeverity;
  message: string;
  clip_id: string | null;
  track_id: string | null;
  suggested_action: string;
}

export interface TimelineValidationReport {
  valid: boolean;
  duration_seconds: number;
  issues: TimelineValidationIssue[];
}

export interface MediaMetadata {
  asset_id: string | null;
  relative_path: string;
  format_name: string;
  duration_seconds: number | null;
  size_bytes: number | null;
  bitrate_bps: number | null;
  width: number | null;
  height: number | null;
  video_codec: string | null;
  audio_codec: string | null;
  frame_rate: string | null;
  sample_rate: number | null;
  channels: number | null;
  probed_at: string;
}

export type MediaOperationKind = "probe" | "export";

export interface MediaExportRequest {
  source_asset_id: string | null;
  output_relative_path: string;
  format: "mp4";
  timeline: Timeline;
}

export interface MediaExportResult {
  operation_id: string;
  status: "completed" | "cancelled" | "failed";
  progress: number;
  output_relative_path: string | null;
  message: string | null;
}

export type MediaJobStatus = "queued" | "running" | "completed" | "failed" | "cancelled";

export interface MediaJob {
  id: string;
  project_path: string;
  kind: "export";
  status: MediaJobStatus;
  progress: number;
  output_relative_path: string;
  message: string | null;
  created_at: string;
  started_at: string | null;
  finished_at: string | null;
}

export type PublishVisibility = "private" | "unlisted" | "public";

export interface PublishMetadata {
  title: string;
  description: string;
  tags: string[];
  visibility: PublishVisibility;
  scheduled_at: string | null;
  channel: string | null;
}

export interface PublishConfigReference {
  provider: string;
  config_path: string;
  oauth_ready: boolean;
  scopes: string[];
  setup_url: string | null;
}

export interface PublishOAuthStart {
  state: string;
  code_verifier: string;
  redirect_uri: string;
  authorize_url: string;
}

export interface PublishOAuthCallbackResult {
  refresh_token_issued: boolean;
  access_token_received: boolean;
  message: string;
}

export interface PublishCheck {
  id: string;
  label: string;
  ok: boolean;
  detail: string;
}

export interface PublishReadiness {
  valid: boolean;
  checks: PublishCheck[];
}

export interface PublishResult {
  operation_id: string;
  status: "completed" | "cancelled" | "failed";
  progress: number;
  dry_run: boolean;
  uploaded: boolean;
  video_url: string | null;
  message: string;
}
