use crate::error::{Result, YtpmError};
use serde_json::Value;

pub const CURRENT_SCHEMA_VERSION: u32 = 2;

/// Applies idempotent JSON migrations before deserializing the portable project DTO.
pub fn migrate_project_value(value: &mut Value) -> Result<()> {
    let version = value
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| YtpmError::InvalidProject("缺少有效的 schema_version".into()))?;

    match version {
        1 => {
            let object = value.as_object_mut().ok_or_else(|| {
                YtpmError::InvalidProject("project.json 必須是 JSON object".into())
            })?;
            object.insert("schema_version".into(), Value::from(CURRENT_SCHEMA_VERSION));
            object.insert("archived_from_status".into(), Value::Null);
        }
        version if version == u64::from(CURRENT_SCHEMA_VERSION) => {}
        unsupported => {
            return Err(YtpmError::InvalidProject(format!(
                "不支援 schema_version {unsupported}，目前版本為 {CURRENT_SCHEMA_VERSION}"
            )))
        }
    }
    Ok(())
}
