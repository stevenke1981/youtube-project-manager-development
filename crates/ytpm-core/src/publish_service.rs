//! YouTube Data API v3 publishing adapter.
//!
//! Credentials are process configuration (`YTPM_YOUTUBE_*`) and are never
//! written into a project folder or SQLite. Publishing is opt-in: callers must
//! complete a dry-run and pass an explicit confirmation before upload.

use crate::{Result, YtpmError};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Utc};
use reqwest::blocking::{Body, Client};
use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE, LOCATION};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use url::Url;
use uuid::Uuid;

const OAUTH_AUTHORIZE_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const OAUTH_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const YOUTUBE_UPLOAD_ENDPOINT: &str = "https://www.googleapis.com/upload/youtube/v3/videos";
const YOUTUBE_SCOPE: &str = "https://www.googleapis.com/auth/youtube.upload";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PublishVisibility {
    Private,
    Unlisted,
    Public,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PublishMetadata {
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub visibility: PublishVisibility,
    pub scheduled_at: Option<String>,
    pub channel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishConfigReference {
    pub provider: String,
    pub config_path: String,
    pub oauth_ready: bool,
    pub scopes: Vec<String>,
    pub setup_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishCheck {
    pub id: String,
    pub label: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishReadiness {
    pub valid: bool,
    pub checks: Vec<PublishCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PublishJobStatus {
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PublishResult {
    pub operation_id: String,
    pub status: PublishJobStatus,
    pub progress: u8,
    pub dry_run: bool,
    pub uploaded: bool,
    pub video_url: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthStart {
    pub state: String,
    pub code_verifier: String,
    pub redirect_uri: String,
    pub authorize_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthCallbackResult {
    pub refresh_token_issued: bool,
    pub access_token_received: bool,
    pub message: String,
}

pub fn config_reference() -> PublishConfigReference {
    PublishConfigReference {
        provider: "YouTube Data API v3".into(),
        config_path: ".ytpm/publish/oauth.json".into(),
        oauth_ready: env_value("YTPM_YOUTUBE_CLIENT_ID").is_some()
            && env_value("YTPM_YOUTUBE_CLIENT_SECRET").is_some()
            && env_value("YTPM_YOUTUBE_REFRESH_TOKEN").is_some(),
        scopes: vec![YOUTUBE_SCOPE.into()],
        setup_url: Some("https://console.cloud.google.com/apis/credentials".into()),
    }
}

pub fn start_oauth() -> Result<OAuthStart> {
    let client_id = env_value("YTPM_YOUTUBE_CLIENT_ID").ok_or_else(|| {
        YtpmError::InvalidInput(
            "缺少 YTPM_YOUTUBE_CLIENT_ID；請先在 Google Cloud 建立 Desktop OAuth client".into(),
        )
    })?;
    let redirect_uri = env_value("YTPM_YOUTUBE_REDIRECT_URI")
        .unwrap_or_else(|| "http://127.0.0.1:8765/oauth2/callback".into());
    let state = Uuid::new_v4().to_string();
    let verifier = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut url = Url::parse(OAUTH_AUTHORIZE_ENDPOINT)
        .map_err(|error| YtpmError::InvalidInput(format!("OAuth endpoint 無效：{error}")))?;
    url.query_pairs_mut()
        .append_pair("client_id", &client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", YOUTUBE_SCOPE)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent")
        .append_pair("state", &state)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256");
    Ok(OAuthStart {
        state,
        code_verifier: verifier,
        redirect_uri,
        authorize_url: url.to_string(),
    })
}

pub fn complete_oauth(
    callback_url: &str,
    expected_state: &str,
    code_verifier: &str,
) -> Result<OAuthCallbackResult> {
    if expected_state.trim().is_empty() || code_verifier.trim().is_empty() {
        return Err(YtpmError::InvalidInput(
            "OAuth state 與 PKCE verifier 不可為空".into(),
        ));
    }
    let callback = Url::parse(callback_url)
        .map_err(|error| YtpmError::InvalidInput(format!("OAuth callback URL 無效：{error}")))?;
    let params: std::collections::HashMap<_, _> = callback.query_pairs().into_owned().collect();
    if params.get("state").map(String::as_str) != Some(expected_state) {
        return Err(YtpmError::InvalidInput(
            "OAuth state 不符合，拒絕 callback".into(),
        ));
    }
    if let Some(error) = params.get("error") {
        return Err(YtpmError::InvalidInput(format!(
            "YouTube OAuth 被拒絕：{error}"
        )));
    }
    let code = params
        .get("code")
        .ok_or_else(|| YtpmError::InvalidInput("OAuth callback 缺少 code".into()))?;
    let client_id = env_value("YTPM_YOUTUBE_CLIENT_ID")
        .ok_or_else(|| YtpmError::InvalidInput("缺少 YTPM_YOUTUBE_CLIENT_ID".into()))?;
    let client_secret = env_value("YTPM_YOUTUBE_CLIENT_SECRET")
        .ok_or_else(|| YtpmError::InvalidInput("缺少 YTPM_YOUTUBE_CLIENT_SECRET".into()))?;
    let redirect_uri = env_value("YTPM_YOUTUBE_REDIRECT_URI")
        .unwrap_or_else(|| "http://127.0.0.1:8765/oauth2/callback".into());
    let response = Client::new()
        .post(OAUTH_TOKEN_ENDPOINT)
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code.as_str()),
            ("code_verifier", code_verifier),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri.as_str()),
        ])
        .send()
        .map_err(|error| {
            YtpmError::InvalidInput(format!("OAuth token endpoint 連線失敗：{error}"))
        })?
        .error_for_status()
        .map_err(|error| YtpmError::InvalidInput(format!("OAuth token exchange 失敗：{error}")))?;
    let token: TokenResponse = response.json().map_err(|error| {
        YtpmError::InvalidInput(format!("OAuth token response 無法解析：{error}"))
    })?;
    if let Some(refresh_token) = token.refresh_token.as_deref() {
        // Keep the token only in the current desktop process. The portable
        // project and SQLite remain credential-free by design.
        std::env::set_var("YTPM_YOUTUBE_REFRESH_TOKEN", refresh_token);
    }
    Ok(OAuthCallbackResult {
        refresh_token_issued: token.refresh_token.is_some(),
        access_token_received: !token.access_token.is_empty(),
        message: "OAuth 完成。refresh token 僅保留在目前 App session，不會寫入專案或 SQLite；可直接執行 YouTube 上傳。".into(),
    })
}

pub fn load_metadata(project_dir: &Path) -> Result<PublishMetadata> {
    let path = metadata_path(project_dir)?;
    if !path.exists() {
        return Ok(PublishMetadata {
            title: "未命名影片".into(),
            description: String::new(),
            tags: Vec::new(),
            visibility: PublishVisibility::Private,
            scheduled_at: None,
            channel: None,
        });
    }
    let content = fs::read_to_string(&path).map_err(|error| YtpmError::io(&path, error))?;
    Ok(serde_json::from_str(&content)?)
}

pub fn save_metadata(project_dir: &Path, metadata: &PublishMetadata) -> Result<PublishMetadata> {
    let readiness = validate_metadata(metadata, project_dir);
    let invalid_metadata = readiness
        .checks
        .iter()
        .filter(|check| {
            matches!(
                check.id.as_str(),
                "title" | "description" | "tags" | "schedule"
            ) && !check.ok
        })
        .map(|check| check.detail.clone())
        .collect::<Vec<_>>();
    if !invalid_metadata.is_empty() {
        return Err(YtpmError::InvalidInput(invalid_metadata.join("；")));
    }
    let path = metadata_path(project_dir)?;
    let parent = path
        .parent()
        .ok_or_else(|| YtpmError::InvalidInput("metadata 缺少 parent".into()))?;
    fs::create_dir_all(parent).map_err(|error| YtpmError::io(parent, error))?;
    reject_reparse_points(parent)?;
    let temporary = parent.join(format!(".publish-{}.tmp", Uuid::new_v4().simple()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| YtpmError::io(&temporary, error))?;
        file.write_all(serde_json::to_string_pretty(metadata)?.as_bytes())
            .map_err(|error| YtpmError::io(&temporary, error))?;
        file.write_all(b"\n")
            .map_err(|error| YtpmError::io(&temporary, error))?;
        file.sync_all()
            .map_err(|error| YtpmError::io(&temporary, error))?;
        fs::rename(&temporary, &path).map_err(|error| YtpmError::io(&path, error))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map(|()| metadata.clone())
}

pub fn validate_metadata(metadata: &PublishMetadata, project_dir: &Path) -> PublishReadiness {
    let mut checks = Vec::new();
    checks.push(check(
        "title",
        "標題",
        !metadata.title.trim().is_empty() && metadata.title.chars().count() <= 100,
        "標題必須為 1–100 個字元。",
    ));
    checks.push(check(
        "description",
        "描述",
        metadata.description.chars().count() <= 5000,
        "描述不可超過 5000 個字元。",
    ));
    checks.push(check(
        "tags",
        "標籤",
        metadata.tags.len() <= 500 && metadata.tags.iter().all(|tag| tag.chars().count() <= 30),
        "標籤最多 500 個且單一標籤不可超過 30 個字元。",
    ));
    let schedule_is_private = metadata.scheduled_at.is_none()
        || matches!(metadata.visibility, PublishVisibility::Private);
    let schedule_ok = metadata
        .scheduled_at
        .as_deref()
        .map(|value| {
            DateTime::parse_from_rfc3339(value)
                .map(|date| date.with_timezone(&Utc) > Utc::now())
                .unwrap_or(false)
        })
        .unwrap_or(true)
        && schedule_is_private;
    let schedule_detail = if !schedule_is_private {
        "YouTube 排程影片必須使用 private visibility。"
    } else {
        "排程時間必須是有效且未來的 RFC3339 時間。"
    };
    checks.push(check("schedule", "排程", schedule_ok, schedule_detail));
    let video = project_dir.join("09_exports").join("final.mp4");
    let project_path_safe = reject_reparse_points(project_dir).is_ok();
    let video_path_safe = project_path_safe && reject_reparse_points(&video).is_ok();
    checks.push(check(
        "video",
        "影片檔",
        video_path_safe && video.is_file(),
        if project_path_safe {
            "請先將完成影片輸出至 09_exports/final.mp4。"
        } else {
            "專案路徑含有 symlink/junction/reparse path，為安全起見拒絕發布。"
        },
    ));
    let config = config_reference();
    checks.push(check(
        "oauth",
        "OAuth",
        config.oauth_ready,
        "請設定 YTPM_YOUTUBE_CLIENT_ID、YTPM_YOUTUBE_CLIENT_SECRET 與 YTPM_YOUTUBE_REFRESH_TOKEN；dry-run 不需要此項。",
    ));
    PublishReadiness {
        valid: checks.iter().all(|check| check.ok),
        checks,
    }
}

pub fn dry_run(project_dir: &Path, metadata: &PublishMetadata) -> Result<PublishResult> {
    let operation_id = Uuid::new_v4().to_string();
    let readiness = validate_metadata(metadata, project_dir);
    let required_failures = readiness
        .checks
        .iter()
        .filter(|check| check.id != "oauth" && !check.ok)
        .map(|check| check.detail.clone())
        .collect::<Vec<_>>();
    if !required_failures.is_empty() {
        return Ok(PublishResult {
            operation_id,
            status: PublishJobStatus::Failed,
            progress: 0,
            dry_run: true,
            uploaded: false,
            video_url: None,
            message: required_failures.join("；"),
        });
    }
    Ok(PublishResult {
        operation_id,
        status: PublishJobStatus::Completed,
        progress: 100,
        dry_run: true,
        uploaded: false,
        video_url: None,
        message: "發布 dry-run 完成：metadata、影片檔與路徑已檢查，未連線、未上傳。".into(),
    })
}

pub fn upload_video(
    project_dir: &Path,
    metadata: &PublishMetadata,
    cancel_marker: Option<&Path>,
) -> Result<PublishResult> {
    let operation_id = Uuid::new_v4().to_string();
    if cancel_marker.is_some_and(Path::exists) {
        return Ok(cancelled_result(operation_id));
    }
    let readiness = validate_metadata(metadata, project_dir);
    if !readiness.valid {
        return Ok(PublishResult {
            operation_id,
            status: PublishJobStatus::Failed,
            progress: 0,
            dry_run: false,
            uploaded: false,
            video_url: None,
            message: readiness
                .checks
                .iter()
                .filter(|check| !check.ok)
                .map(|check| check.detail.clone())
                .collect::<Vec<_>>()
                .join("；"),
        });
    }
    let client_id = env_value("YTPM_YOUTUBE_CLIENT_ID").unwrap();
    let client_secret = env_value("YTPM_YOUTUBE_CLIENT_SECRET")
        .ok_or_else(|| YtpmError::InvalidInput("缺少 YTPM_YOUTUBE_CLIENT_SECRET".into()))?;
    let refresh_token = env_value("YTPM_YOUTUBE_REFRESH_TOKEN").unwrap();
    let client = Client::new();
    let token: TokenResponse = client
        .post(OAUTH_TOKEN_ENDPOINT)
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("refresh_token", refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .map_err(|error| {
            YtpmError::InvalidInput(format!("YouTube token endpoint 連線失敗：{error}"))
        })?
        .error_for_status()
        .map_err(|error| YtpmError::InvalidInput(format!("YouTube token refresh 失敗：{error}")))?
        .json()
        .map_err(|error| {
            YtpmError::InvalidInput(format!("YouTube token response 無法解析：{error}"))
        })?;
    if cancel_marker.is_some_and(Path::exists) {
        return Ok(cancelled_result(operation_id));
    }
    let video_path = project_dir.join("09_exports").join("final.mp4");
    reject_reparse_points(project_dir)?;
    reject_reparse_points(&video_path)?;
    let file_size = fs::metadata(&video_path)
        .map_err(|error| YtpmError::io(&video_path, error))?
        .len();
    let resource = VideoResource {
        snippet: VideoSnippet {
            title: metadata.title.clone(),
            description: metadata.description.clone(),
            tags: metadata.tags.clone(),
            category_id: "22".into(),
        },
        status: VideoStatus {
            privacy_status: metadata.visibility.clone(),
            publish_at: metadata.scheduled_at.clone(),
        },
    };
    let init = client
        .post(format!("{YOUTUBE_UPLOAD_ENDPOINT}?part=snippet,status"))
        .bearer_auth(&token.access_token)
        .header(CONTENT_TYPE, "application/json")
        .header("X-Upload-Content-Type", "video/mp4")
        .header("X-Upload-Content-Length", file_size)
        .json(&resource)
        .send()
        .map_err(|error| {
            YtpmError::InvalidInput(format!("YouTube resumable upload 初始化失敗：{error}"))
        })?
        .error_for_status()
        .map_err(|error| YtpmError::InvalidInput(format!("YouTube upload 初始化拒絕：{error}")))?;
    let upload_url = init
        .headers()
        .get(LOCATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| YtpmError::InvalidInput("YouTube upload 初始化沒有回傳 Location".into()))?
        .to_owned();
    if cancel_marker.is_some_and(Path::exists) {
        return Ok(cancelled_result(operation_id));
    }
    let file = File::open(&video_path).map_err(|error| YtpmError::io(&video_path, error))?;
    let response: YouTubeVideoResponse = client
        .put(upload_url)
        .bearer_auth(&token.access_token)
        .header(CONTENT_TYPE, "video/mp4")
        .header(CONTENT_LENGTH, file_size)
        .body(Body::new(file))
        .send()
        .map_err(|error| YtpmError::InvalidInput(format!("YouTube 影片上傳失敗：{error}")))?
        .error_for_status()
        .map_err(|error| YtpmError::InvalidInput(format!("YouTube 影片上傳被拒絕：{error}")))?
        .json()
        .map_err(|error| {
            YtpmError::InvalidInput(format!("YouTube upload response 無法解析：{error}"))
        })?;
    Ok(PublishResult {
        operation_id,
        status: PublishJobStatus::Completed,
        progress: 100,
        dry_run: false,
        uploaded: true,
        video_url: Some(format!("https://www.youtube.com/watch?v={}", response.id)),
        message: "YouTube 影片上傳完成。".into(),
    })
}

fn metadata_path(project_dir: &Path) -> Result<PathBuf> {
    if project_dir
        .components()
        .any(|item| item == Component::ParentDir)
    {
        return Err(YtpmError::InvalidInput("專案路徑不可包含 ..".into()));
    }
    reject_reparse_points(project_dir)?;
    let path = project_dir.join("08_metadata").join("publish.json");
    reject_reparse_points(&path)?;
    Ok(path)
}

fn reject_reparse_points(path: &Path) -> Result<()> {
    let mut current = Some(path);
    while let Some(candidate) = current {
        match fs::symlink_metadata(candidate) {
            Ok(metadata) if metadata_is_reparse_point(&metadata) => {
                return Err(YtpmError::InvalidInput(format!(
                    "拒絕操作 symlink/junction/reparse path：{}",
                    candidate.display()
                )))
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(YtpmError::io(candidate, error)),
        }
        current = candidate.parent();
    }
    Ok(())
}

fn metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn env_value(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn check(id: &str, label: &str, ok: bool, detail: &str) -> PublishCheck {
    PublishCheck {
        id: id.into(),
        label: label.into(),
        ok,
        detail: if ok {
            format!("{label}檢查通過。")
        } else {
            detail.into()
        },
    }
}

fn cancelled_result(operation_id: String) -> PublishResult {
    PublishResult {
        operation_id,
        status: PublishJobStatus::Cancelled,
        progress: 0,
        dry_run: false,
        uploaded: false,
        video_url: None,
        message: "發布已取消，未完成上傳。".into(),
    }
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
}

#[derive(Debug, Serialize)]
struct VideoResource {
    snippet: VideoSnippet,
    status: VideoStatus,
}

#[derive(Debug, Serialize)]
struct VideoSnippet {
    title: String,
    description: String,
    tags: Vec<String>,
    #[serde(rename = "categoryId")]
    category_id: String,
}

#[derive(Debug, Serialize)]
struct VideoStatus {
    #[serde(rename = "privacyStatus")]
    privacy_status: PublishVisibility,
    #[serde(rename = "publishAt", skip_serializing_if = "Option::is_none")]
    publish_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct YouTubeVideoResponse {
    id: String,
}
