use crate::error::{Result, YtpmError};
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path};
use uuid::Uuid;

const MAX_DOCUMENT_BYTES: u64 = 4 * 1024 * 1024;

/// Reads one of the editable, portable text documents in a project.
pub fn read_document(project_dir: &Path, relative_path: &Path) -> Result<String> {
    let path = resolve_document_path(project_dir, relative_path)?;
    let metadata = fs::metadata(&path).map_err(|error| YtpmError::io(&path, error))?;
    reject_if_too_large(&path, metadata.len())?;
    fs::read_to_string(&path).map_err(|error| YtpmError::io(&path, error))
}

/// Atomically replaces one of the editable, portable text documents in a project.
pub fn write_document(project_dir: &Path, relative_path: &Path, content: &str) -> Result<()> {
    let path = resolve_document_path(project_dir, relative_path)?;
    let bytes = content.as_bytes();
    if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err(YtpmError::InvalidInput(format!(
            "文件內容超過 {} MiB 上限：{}",
            MAX_DOCUMENT_BYTES / (1024 * 1024),
            relative_path.display()
        )));
    }

    let parent = path
        .parent()
        .ok_or_else(|| YtpmError::InvalidInput(format!("找不到文件 parent：{}", path.display())))?;
    reject_reparse_ancestors(&path)?;
    let file_name = path.file_name().and_then(OsStr::to_str).ok_or_else(|| {
        YtpmError::InvalidInput(format!("文件名稱無效：{}", relative_path.display()))
    })?;
    let temporary_path = parent.join(format!(".{file_name}.{}.tmp", Uuid::new_v4().simple()));

    let mut temporary_created = false;
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
            .map_err(|error| YtpmError::io(&temporary_path, error))?;
        temporary_created = true;
        file.write_all(bytes)
            .map_err(|error| YtpmError::io(&temporary_path, error))?;
        file.sync_all()
            .map_err(|error| YtpmError::io(&temporary_path, error))?;
        replace_document_file(&temporary_path, &path)
    })();

    if result.is_err() && temporary_created {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

fn resolve_document_path(project_dir: &Path, relative_path: &Path) -> Result<std::path::PathBuf> {
    validate_relative_document_path(relative_path)?;
    reject_reparse_ancestors(project_dir)?;
    let path = project_dir.join(relative_path);
    reject_reparse_ancestors(&path)?;
    Ok(path)
}

fn validate_relative_document_path(relative_path: &Path) -> Result<()> {
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| matches!(component, Component::Prefix(_) | Component::RootDir))
    {
        return Err(YtpmError::InvalidInput(format!(
            "文件路徑必須是允許的相對路徑：{}",
            relative_path.display()
        )));
    }
    if relative_path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(YtpmError::InvalidInput(format!(
            "文件路徑不可包含 . 或 ..：{}",
            relative_path.display()
        )));
    }

    let components = relative_path.components().collect::<Vec<_>>();
    let is_exact_metadata_file = components.len() == 2
        && components[0].as_os_str() == OsStr::new("08_metadata")
        && matches!(
            components[1].as_os_str().to_str(),
            Some("title.md" | "description.md" | "pinned-comment.md" | "chapters.txt" | "tags.txt")
        );
    let is_script = components.len() == 2
        && components[0].as_os_str() == OsStr::new("02_script")
        && components[1].as_os_str() == OsStr::new("script.md");
    let is_subtitle = components.len() == 3
        && components[0].as_os_str() == OsStr::new("06_subtitles")
        && components[1].as_os_str() == OsStr::new("translations")
        && components[2]
            .as_os_str()
            .to_str()
            .and_then(|name| name.rsplit_once('.'))
            .is_some_and(|(stem, extension)| {
                !stem.is_empty()
                    && matches!(
                        extension.to_ascii_lowercase().as_str(),
                        "srt" | "vtt" | "ass"
                    )
            });

    if !(is_script || is_exact_metadata_file || is_subtitle) {
        return Err(YtpmError::InvalidInput(format!(
            "不允許編輯此文件路徑：{}",
            relative_path.display()
        )));
    }
    Ok(())
}

fn reject_reparse_ancestors(path: &Path) -> Result<()> {
    let mut current = Some(path);
    while let Some(candidate) = current {
        match fs::symlink_metadata(candidate) {
            Ok(metadata) if metadata_is_reparse_point(&metadata) => {
                return Err(YtpmError::InvalidInput(format!(
                    "拒絕操作 symlink/junction/reparse path：{}",
                    candidate.display()
                )));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(YtpmError::io(candidate, error)),
        }
        current = candidate.parent();
    }
    Ok(())
}

fn reject_if_too_large(path: &Path, bytes: u64) -> Result<()> {
    if bytes > MAX_DOCUMENT_BYTES {
        return Err(YtpmError::InvalidProject(format!(
            "文件超過 {} MiB 上限：{}",
            MAX_DOCUMENT_BYTES / (1024 * 1024),
            path.display()
        )));
    }
    Ok(())
}

fn replace_document_file(temporary_path: &Path, destination: &Path) -> Result<()> {
    #[cfg(not(windows))]
    {
        fs::rename(temporary_path, destination)
            .map_err(|error| YtpmError::io(destination, error))?;
        Ok(())
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use std::ptr::null_mut;

        fn wide(path: &Path) -> Vec<u16> {
            path.as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect()
        }

        let temporary = wide(temporary_path);
        let destination_wide = wide(destination);
        let result = unsafe {
            if destination.exists() {
                ReplaceFileW(
                    destination_wide.as_ptr(),
                    temporary.as_ptr(),
                    std::ptr::null(),
                    0,
                    null_mut(),
                    null_mut(),
                )
            } else {
                MoveFileExW(
                    temporary.as_ptr(),
                    destination_wide.as_ptr(),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            }
        };
        if result == 0 {
            return Err(YtpmError::io(destination, std::io::Error::last_os_error()));
        }
        Ok(())
    }
}

#[cfg(windows)]
const MOVEFILE_REPLACE_EXISTING: u32 = 0x0000_0001;
#[cfg(windows)]
const MOVEFILE_WRITE_THROUGH: u32 = 0x0000_0008;

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn MoveFileExW(existing_file_name: *const u16, new_file_name: *const u16, flags: u32) -> i32;
    fn ReplaceFileW(
        replaced_file_name: *const u16,
        replacement_file_name: *const u16,
        backup_file_name: *const u16,
        replace_flags: u32,
        exclude: *mut std::ffi::c_void,
        reserved: *mut std::ffi::c_void,
    ) -> i32;
}

#[cfg(windows)]
fn metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}
