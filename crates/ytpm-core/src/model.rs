use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    #[default]
    Idea,
    Research,
    Script,
    Voice,
    Visuals,
    Editing,
    Subtitles,
    Thumbnail,
    Review,
    Scheduled,
    Published,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub folder_name: String,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub series: Option<String>,
    pub status: ProjectStatus,
    #[serde(default)]
    pub archived_from_status: Option<ProjectStatus>,
    pub aspect_ratio: String,
    pub language: String,
    #[serde(default)]
    pub target_duration_seconds: Option<u32>,
    #[serde(default)]
    pub planned_publish_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub published_at: Option<DateTime<Utc>>,
    pub progress: u8,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub app_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub title: String,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub series: Option<String>,
    #[serde(default = "default_aspect_ratio")]
    pub aspect_ratio: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub target_duration_seconds: Option<u32>,
    #[serde(default)]
    pub planned_publish_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_aspect_ratio() -> String {
    "16:9".to_string()
}

fn default_language() -> String {
    "zh-TW".to_string()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub code: String,
    pub severity: ValidationSeverity,
    pub message: String,
    pub path: Option<String>,
    pub suggested_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub project: Option<Project>,
    pub issues: Vec<ValidationIssue>,
}
