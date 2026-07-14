use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::{Result, YtpmError};

pub const CURRENT_ASSET_SCHEMA_VERSION: u32 = 1;

const IGNORED_FILE_NAMES: &[&str] = &[
    "project.json",
    "tasks.json",
    "assets.json",
    "README.md",
    "activity.log",
];
const IGNORED_DIRECTORIES: &[&str] = &[".ytpm", ".ytpm-backup"];
const HASH_BUFFER_SIZE: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Research,
    Script,
    Voice,
    Music,
    SoundEffect,
    Image,
    Video,
    Subtitle,
    Thumbnail,
    Metadata,
    Export,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetState {
    Available,
    Missing,
    Archived,
    Processing,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub id: String,
    pub kind: AssetKind,
    pub relative_path: String,
    #[serde(default)]
    pub display_name: Option<String>,
    pub state: AssetState,
    #[serde(default)]
    pub source_type: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<u64>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub generator: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub version_group_id: Option<String>,
    #[serde(default)]
    pub version_number: Option<u32>,
    #[serde(default)]
    pub is_adopted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetCatalog {
    pub schema_version: u32,
    pub assets: Vec<Asset>,
}

/// Recursively discovers ordinary files and updates the portable assets catalog.
pub fn scan_assets(project_dir: &Path) -> Result<AssetCatalog> {
    ensure_project_dir(project_dir)?;

    let catalog_path = project_dir.join("assets.json");
    let mut existing_by_path = HashMap::new();
    if catalog_exists(&catalog_path)? {
        let existing = read_catalog(&catalog_path)?;
        for asset in existing.assets {
            validate_catalog_asset(&asset)?;
            if existing_by_path
                .insert(asset.relative_path.clone(), asset)
                .is_some()
            {
                return Err(YtpmError::InvalidProject(
                    "assets.json 包含重複的 relative_path".into(),
                ));
            }
        }
    }

    let mut assets = Vec::new();
    for entry in WalkDir::new(project_dir).follow_links(false) {
        let entry = entry.map_err(|error| {
            YtpmError::InvalidInput(format!("掃描 {} 失敗：{error}", project_dir.display()))
        })?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(path).map_err(|error| YtpmError::io(path, error))?;
        if metadata_is_reparse_point(&metadata) {
            return Err(YtpmError::InvalidInput(format!(
                "拒絕掃描 symlink/junction/reparse path：{}",
                path.display()
            )));
        }
        if path == project_dir || !metadata.file_type().is_file() {
            continue;
        }

        let relative_path = relative_path(project_dir, path)?;
        if is_ignored_path(&relative_path) {
            continue;
        }
        validate_relative_path(&relative_path)?;

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                YtpmError::InvalidInput(format!("檔名不是有效 UTF-8：{}", path.display()))
            })?;
        let modified_at = modified_at(&metadata);
        let sha256 = hash_file(path)?;
        let kind = kind_for_relative_path(&relative_path);
        let archived = is_archived_path(&relative_path);

        let mut asset = existing_by_path
            .remove(&relative_path)
            .unwrap_or_else(|| Asset {
                id: Uuid::new_v4().to_string(),
                kind: kind.clone(),
                relative_path: relative_path.clone(),
                display_name: Some(file_name.to_string()),
                state: if archived {
                    AssetState::Archived
                } else {
                    AssetState::Available
                },
                source_type: Some("imported".into()),
                size_bytes: None,
                sha256: None,
                duration_ms: None,
                width: None,
                height: None,
                generator: None,
                model: None,
                prompt: None,
                version_group_id: None,
                version_number: Some(1),
                is_adopted: false,
                created_at: modified_at,
                updated_at: modified_at,
            });

        if matches!(asset.state, AssetState::Missing) {
            asset.state = if archived {
                AssetState::Archived
            } else {
                AssetState::Available
            };
        }
        if asset.display_name.is_none() {
            asset.display_name = Some(file_name.to_string());
        }
        asset.kind = kind;
        asset.relative_path = relative_path;
        asset.size_bytes = Some(metadata.len());
        asset.sha256 = Some(sha256);
        asset.updated_at = modified_at;
        assets.push(asset);
    }

    let now = Utc::now();
    for (_, mut asset) in existing_by_path {
        let path = safe_asset_path(project_dir, &asset.relative_path)?;
        if asset.state != AssetState::Missing {
            asset.state = AssetState::Missing;
            asset.updated_at = now;
        }
        if path.exists() {
            return Err(YtpmError::InvalidInput(format!(
                "素材路徑不是一般檔案：{}",
                path.display()
            )));
        }
        assets.push(asset);
    }

    sort_assets(&mut assets);
    let catalog = AssetCatalog {
        schema_version: CURRENT_ASSET_SCHEMA_VERSION,
        assets,
    };
    atomic_write_json(&catalog_path, &catalog)?;
    Ok(catalog)
}

/// Reads the catalog, preserving records whose files no longer exist.
pub fn list_assets(project_dir: &Path) -> Result<Vec<Asset>> {
    ensure_project_dir(project_dir)?;
    let catalog_path = project_dir.join("assets.json");
    if !catalog_exists(&catalog_path)? {
        return Ok(scan_assets(project_dir)?.assets);
    }

    let mut catalog = read_catalog(&catalog_path)?;
    let mut changed = false;
    for asset in &mut catalog.assets {
        validate_catalog_asset(asset)?;
        let path = safe_asset_path(project_dir, &asset.relative_path)?;
        let is_regular_file = match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if metadata_is_reparse_point(&metadata) {
                    return Err(YtpmError::InvalidInput(format!(
                        "拒絕讀取 symlink/junction/reparse path：{}",
                        path.display()
                    )));
                }
                metadata.file_type().is_file()
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(YtpmError::io(&path, error)),
        };

        if !is_regular_file && asset.state != AssetState::Missing {
            asset.state = AssetState::Missing;
            asset.updated_at = Utc::now();
            changed = true;
        }
    }

    if changed {
        sort_assets(&mut catalog.assets);
        atomic_write_json(&catalog_path, &catalog)?;
    }
    sort_assets(&mut catalog.assets);
    Ok(catalog.assets)
}

fn read_catalog(path: &Path) -> Result<AssetCatalog> {
    reject_reparse_points(path)?;
    let content = fs::read_to_string(path).map_err(|error| YtpmError::io(path, error))?;
    let catalog: AssetCatalog = serde_json::from_str(&content)?;
    if catalog.schema_version != CURRENT_ASSET_SCHEMA_VERSION {
        return Err(YtpmError::InvalidProject(format!(
            "不支援 assets.json schema_version {}，目前版本為 {}",
            catalog.schema_version, CURRENT_ASSET_SCHEMA_VERSION
        )));
    }

    let mut ids = HashSet::new();
    for asset in &catalog.assets {
        validate_catalog_asset(asset)?;
        if asset.id.is_empty() || !ids.insert(asset.id.as_str()) {
            return Err(YtpmError::InvalidProject(
                "assets.json 包含空白或重複的 asset id".into(),
            ));
        }
    }
    Ok(catalog)
}

fn validate_catalog_asset(asset: &Asset) -> Result<()> {
    validate_relative_path(&asset.relative_path)?;
    if is_ignored_path(&asset.relative_path) {
        return Err(YtpmError::InvalidInput(format!(
            "catalog 不可記錄受保護的 metadata 路徑：{}",
            asset.relative_path
        )));
    }
    if asset.version_number == Some(0) {
        return Err(YtpmError::InvalidInput(format!(
            "version_number 必須大於 0：{}",
            asset.relative_path
        )));
    }
    Ok(())
}

fn ensure_project_dir(project_dir: &Path) -> Result<()> {
    if project_dir
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(YtpmError::InvalidInput(format!(
            "專案路徑不可包含 ..：{}",
            project_dir.display()
        )));
    }
    reject_reparse_points(project_dir)?;
    let metadata = fs::metadata(project_dir).map_err(|error| YtpmError::io(project_dir, error))?;
    if !metadata.is_dir() {
        return Err(YtpmError::InvalidInput(format!(
            "專案路徑不是資料夾：{}",
            project_dir.display()
        )));
    }
    Ok(())
}

fn catalog_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata_is_reparse_point(&metadata) {
                return Err(YtpmError::InvalidInput(format!(
                    "拒絕操作 symlink/junction/reparse catalog：{}",
                    path.display()
                )));
            }
            Ok(metadata.is_file())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(YtpmError::io(path, error)),
    }
}

fn relative_path(project_dir: &Path, path: &Path) -> Result<String> {
    let relative = path
        .strip_prefix(project_dir)
        .map_err(|_| YtpmError::InvalidInput(format!("素材路徑不在專案內：{}", path.display())))?;
    let mut components = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(value) => components.push(value.to_str().ok_or_else(|| {
                YtpmError::InvalidInput(format!("路徑不是有效 UTF-8：{}", path.display()))
            })?),
            _ => {
                return Err(YtpmError::InvalidInput(format!(
                    "素材相對路徑包含非法元件：{}",
                    path.display()
                )))
            }
        }
    }
    let relative = components.join("/");
    validate_relative_path(&relative)?;
    Ok(relative)
}

fn safe_asset_path(project_dir: &Path, relative: &str) -> Result<PathBuf> {
    validate_relative_path(relative)?;
    let path = project_dir.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
    reject_reparse_points(&path)?;
    Ok(path)
}

fn validate_relative_path(relative: &str) -> Result<()> {
    if relative.is_empty()
        || relative.starts_with('/')
        || relative.contains('\\')
        || relative.contains("//")
        || relative.ends_with('/')
    {
        return Err(YtpmError::InvalidInput(format!(
            "asset relative_path 必須是安全的相對路徑：{relative}"
        )));
    }
    let path = Path::new(relative);
    if path.is_absolute() {
        return Err(YtpmError::InvalidInput(format!(
            "asset relative_path 不可是 absolute path：{relative}"
        )));
    }
    for component in path.components() {
        match component {
            Component::Normal(value) => validate_path_component(value, relative)?,
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(YtpmError::InvalidInput(format!(
                    "asset relative_path 不可包含 .、.. 或 root：{relative}"
                )))
            }
        }
    }
    Ok(())
}

fn validate_path_component(component: &std::ffi::OsStr, relative: &str) -> Result<()> {
    let component = component.to_str().ok_or_else(|| {
        YtpmError::InvalidInput(format!("asset relative_path 不是有效 UTF-8：{relative}"))
    })?;
    if component.is_empty()
        || component
            .chars()
            .any(|character| character.is_control() || r#"<>:\"|?*"#.contains(character))
        || component.ends_with(' ')
        || component.ends_with('.')
    {
        return Err(YtpmError::InvalidInput(format!(
            "asset relative_path 包含非法 Windows 檔名元件：{relative}"
        )));
    }

    let stem = component
        .split_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(component)
        .to_ascii_uppercase();
    if matches!(
        stem.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    ) {
        return Err(YtpmError::InvalidInput(format!(
            "asset relative_path 使用 Windows 保留名稱：{relative}"
        )));
    }
    Ok(())
}

fn kind_for_relative_path(relative: &str) -> AssetKind {
    let components: Vec<&str> = relative.split('/').collect();
    let first = components
        .first()
        .copied()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let second = components
        .get(1)
        .copied()
        .unwrap_or_default()
        .to_ascii_lowercase();
    match first.as_str() {
        "01_research" | "research" => AssetKind::Research,
        "02_script" | "script" => AssetKind::Script,
        "03_voice" | "voice" => match second.as_str() {
            "music" => AssetKind::Music,
            "sound_effects" | "sound-effects" | "sound_effect" => AssetKind::SoundEffect,
            _ => AssetKind::Voice,
        },
        "04_images" | "images" | "image" => AssetKind::Image,
        "05_video" | "video" => AssetKind::Video,
        "06_subtitles" | "subtitles" | "subtitle" => AssetKind::Subtitle,
        "07_thumbnail" | "thumbnail" => AssetKind::Thumbnail,
        "08_metadata" | "metadata" => AssetKind::Metadata,
        "09_exports" | "exports" | "export" => AssetKind::Export,
        _ => AssetKind::Other,
    }
}

fn is_archived_path(relative: &str) -> bool {
    relative
        .split('/')
        .next()
        .is_some_and(|component| component.eq_ignore_ascii_case("10_archive"))
}

fn is_ignored_path(relative: &str) -> bool {
    let components: Vec<&str> = relative.split('/').collect();
    components.iter().any(|component| {
        IGNORED_DIRECTORIES
            .iter()
            .any(|ignored| component.eq_ignore_ascii_case(ignored))
    }) || components.last().is_some_and(|file_name| {
        IGNORED_FILE_NAMES
            .iter()
            .any(|ignored| file_name.eq_ignore_ascii_case(ignored))
    })
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = File::open(path).map_err(|error| YtpmError::io(path, error))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; HASH_BUFFER_SIZE];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| YtpmError::io(path, error))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn modified_at(metadata: &fs::Metadata) -> DateTime<Utc> {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| {
            DateTime::<Utc>::from_timestamp(
                i64::try_from(duration.as_secs()).ok()?,
                duration.subsec_nanos(),
            )
        })
        .unwrap_or_else(Utc::now)
}

fn sort_assets(assets: &mut [Asset]) {
    assets.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
}

fn atomic_write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let parent = path.parent().ok_or_else(|| {
        YtpmError::InvalidInput(format!("找不到 assets.json parent：{}", path.display()))
    })?;
    reject_reparse_points(parent)?;
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata_is_reparse_point(&metadata) {
            return Err(YtpmError::InvalidInput(format!(
                "拒絕覆寫 symlink/junction/reparse catalog：{}",
                path.display()
            )));
        }
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            YtpmError::InvalidInput(format!("assets.json 檔名無效：{}", path.display()))
        })?;
    let temp_path = parent.join(format!(".{file_name}.{}.tmp", Uuid::new_v4().simple()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|error| YtpmError::io(&temp_path, error))?;
        file.write_all(&bytes)
            .map_err(|error| YtpmError::io(&temp_path, error))?;
        file.write_all(b"\n")
            .map_err(|error| YtpmError::io(&temp_path, error))?;
        file.sync_all()
            .map_err(|error| YtpmError::io(&temp_path, error))?;
        replace_file(&temp_path, path).map_err(|error| YtpmError::io(path, error))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[cfg(not(windows))]
fn replace_file(temp_path: &Path, path: &Path) -> std::io::Result<()> {
    fs::rename(temp_path, path)
}

#[cfg(windows)]
fn replace_file(temp_path: &Path, path: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x0000_0001;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x0000_0008;

    #[link(name = "kernel32")]
    extern "system" {
        fn MoveFileExW(
            existing_file_name: *const u16,
            new_file_name: *const u16,
            flags: u32,
        ) -> i32;
    }

    let existing: Vec<u16> = temp_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let destination: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let result = unsafe {
        MoveFileExW(
            existing.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn reject_reparse_points(path: &Path) -> Result<()> {
    let mut current = Some(path);
    while let Some(candidate) = current {
        if let Ok(metadata) = fs::symlink_metadata(candidate) {
            if metadata_is_reparse_point(&metadata) {
                return Err(YtpmError::InvalidInput(format!(
                    "拒絕操作 symlink/junction/reparse path：{}",
                    candidate.display()
                )));
            }
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
