use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::paths::home_dir;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OpenCodePathStatus {
    Applied,
    AlreadyPresent,
    ConfigConflict,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCodePathReport {
    pub status: OpenCodePathStatus,
    pub config_path: PathBuf,
    pub skills_root: PathBuf,
    pub backup_path: Option<PathBuf>,
    pub message: Option<String>,
}

pub fn default_opencode_config_path() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("OPENCODE_CONFIG_DIR").map(PathBuf::from) {
        return Some(dir.join("opencode.json"));
    }
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
        return Some(dir.join("opencode/opencode.json"));
    }
    home_dir().map(|home| home.join(".config/opencode/opencode.json"))
}

pub fn ensure_opencode_skill_path(
    config_path: Option<PathBuf>,
    skills_root: &Path,
    backup_suffix: &str,
) -> Result<OpenCodePathReport> {
    let config_path = config_path
        .or_else(default_opencode_config_path)
        .ok_or_else(|| Error::InvalidInput("cannot determine OpenCode config path".to_string()))?;

    if config_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext != "json")
        .unwrap_or(false)
    {
        return Ok(OpenCodePathReport {
            status: OpenCodePathStatus::ConfigConflict,
            config_path,
            skills_root: skills_root.to_path_buf(),
            backup_path: None,
            message: Some("only plain JSON OpenCode config can be safely modified".to_string()),
        });
    }

    let mut config = if config_path.exists() {
        let body = fs::read_to_string(&config_path)
            .map_err(|err| Error::io(format!("failed to read {}", config_path.display()), err))?;
        serde_json::from_str::<Value>(&body).map_err(|err| Error::Json {
            context: format!("failed to parse {}", config_path.display()),
            source: err,
        })?
    } else {
        json!({})
    };

    let Some(object) = config.as_object_mut() else {
        return Ok(OpenCodePathReport {
            status: OpenCodePathStatus::ConfigConflict,
            config_path,
            skills_root: skills_root.to_path_buf(),
            backup_path: None,
            message: Some("OpenCode config root is not an object".to_string()),
        });
    };

    let skills = object.entry("skills").or_insert_with(|| json!({}));
    let Some(skills_object) = skills.as_object_mut() else {
        return Ok(OpenCodePathReport {
            status: OpenCodePathStatus::ConfigConflict,
            config_path,
            skills_root: skills_root.to_path_buf(),
            backup_path: None,
            message: Some("OpenCode skills config is not an object".to_string()),
        });
    };

    let paths = skills_object.entry("paths").or_insert_with(|| json!([]));
    let Some(paths_array) = paths.as_array_mut() else {
        return Ok(OpenCodePathReport {
            status: OpenCodePathStatus::ConfigConflict,
            config_path,
            skills_root: skills_root.to_path_buf(),
            backup_path: None,
            message: Some("OpenCode skills.paths config is not an array".to_string()),
        });
    };

    let skills_root_string = skills_root.display().to_string();
    if paths_array
        .iter()
        .any(|path| path.as_str() == Some(skills_root_string.as_str()))
    {
        return Ok(OpenCodePathReport {
            status: OpenCodePathStatus::AlreadyPresent,
            config_path,
            skills_root: skills_root.to_path_buf(),
            backup_path: None,
            message: None,
        });
    }

    paths_array.push(Value::String(skills_root_string));

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| Error::io(format!("failed to create {}", parent.display()), err))?;
    }

    let backup_path = if config_path.exists() {
        let backup_path = config_path.with_extension(format!("json.{backup_suffix}.bak"));
        fs::copy(&config_path, &backup_path).map_err(|err| {
            Error::io(
                format!(
                    "failed to backup {} to {}",
                    config_path.display(),
                    backup_path.display()
                ),
                err,
            )
        })?;
        Some(backup_path)
    } else {
        None
    };

    let body = serde_json::to_string_pretty(&config).map_err(|err| Error::Json {
        context: format!("failed to serialize {}", config_path.display()),
        source: err,
    })?;
    fs::write(&config_path, format!("{body}\n"))
        .map_err(|err| Error::io(format!("failed to write {}", config_path.display()), err))?;

    Ok(OpenCodePathReport {
        status: OpenCodePathStatus::Applied,
        config_path,
        skills_root: skills_root.to_path_buf(),
        backup_path,
        message: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn appends_skill_path_to_json_config() {
        let root = test_dir("opencode");
        let config = root.join("opencode.json");
        let skills = root.join("skills");
        fs::write(&config, "{}").unwrap();

        let report = ensure_opencode_skill_path(Some(config.clone()), &skills, "test").unwrap();

        assert_eq!(report.status, OpenCodePathStatus::Applied);
        let body = fs::read_to_string(config).unwrap();
        assert!(body.contains("skills"));
        assert!(body.contains("paths"));
        assert!(body.contains(&skills.display().to_string()));
    }

    fn test_dir(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("skills-manager-{name}-{nanos}"));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
