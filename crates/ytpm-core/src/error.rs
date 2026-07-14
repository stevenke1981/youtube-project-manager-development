use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, YtpmError>;

#[derive(Debug, Error)]
pub enum YtpmError {
    #[error("輸入無效：{0}")]
    InvalidInput(String),

    #[error("無法存取路徑 {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("JSON 格式錯誤：{0}")]
    Json(#[from] serde_json::Error),

    #[error("不是有效的 YTPM 專案：{0}")]
    InvalidProject(String),
}

impl YtpmError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
