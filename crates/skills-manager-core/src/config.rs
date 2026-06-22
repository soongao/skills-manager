use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::model::{EnvironmentConfig, SourceProfile, WorkspaceConfig};

pub const CONFIG_FILE: &str = "config.json";
pub const STATE_FILE: &str = "state.json";
pub const LOG_DIR: &str = "logs";
pub const RUNS_DIR: &str = "runs";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedLink {
    pub environment_id: String,
    pub agent_id: crate::agents::AgentId,
    pub skill_id: String,
    pub source_path: PathBuf,
    pub target_path: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncState {
    pub environment_id: String,
    pub last_sync_at: Option<String>,
    pub last_status: Option<String>,
    pub last_error_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceState {
    pub schema_version: u32,
    pub managed_links: Vec<ManagedLink>,
    pub sync: Vec<SyncState>,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            schema_version: 1,
            managed_links: Vec::new(),
            sync: Vec::new(),
        }
    }
}

pub fn config_path(config_home: &Path) -> PathBuf {
    config_home.join(CONFIG_FILE)
}

pub fn state_path(config_home: &Path) -> PathBuf {
    config_home.join(STATE_FILE)
}

pub fn logs_dir(config_home: &Path) -> PathBuf {
    config_home.join(LOG_DIR)
}

pub fn runs_dir(config_home: &Path) -> PathBuf {
    config_home.join(RUNS_DIR)
}

pub fn init_config_home(config_home: &Path) -> Result<()> {
    fs::create_dir_all(config_home)
        .map_err(|err| Error::io(format!("failed to create {}", config_home.display()), err))?;
    fs::create_dir_all(logs_dir(config_home)).map_err(|err| {
        Error::io(
            format!("failed to create {}", logs_dir(config_home).display()),
            err,
        )
    })?;
    fs::create_dir_all(runs_dir(config_home)).map_err(|err| {
        Error::io(
            format!("failed to create {}", runs_dir(config_home).display()),
            err,
        )
    })?;
    Ok(())
}

pub fn validate_workspace_config(config: &WorkspaceConfig) -> Result<()> {
    if config.schema_version != 1 {
        return Err(Error::InvalidInput(format!(
            "unsupported config schema version: {}",
            config.schema_version
        )));
    }

    if config
        .source_profiles
        .iter()
        .all(|profile| profile.source_profile_id() != config.active_source_profile_id)
    {
        return Err(Error::InvalidInput(format!(
            "active source profile is not configured: {}",
            config.active_source_profile_id
        )));
    }

    for environment in &config.environments {
        validate_environment(environment)?;
    }

    Ok(())
}

pub fn active_source_profile(config: &WorkspaceConfig) -> Result<&SourceProfile> {
    validate_workspace_config(config)?;
    config
        .source_profiles
        .iter()
        .find(|profile| profile.source_profile_id() == config.active_source_profile_id)
        .ok_or_else(|| Error::InvalidInput("active source profile is missing".to_string()))
}

pub fn environment<'a>(
    config: &'a WorkspaceConfig,
    environment_id: Option<&str>,
) -> Result<&'a EnvironmentConfig> {
    validate_workspace_config(config)?;
    if let Some(environment_id) = environment_id {
        return config
            .environments
            .iter()
            .find(|environment| environment.environment_id == environment_id)
            .ok_or_else(|| {
                Error::InvalidInput(format!("environment is not configured: {environment_id}"))
            });
    }

    config
        .environments
        .iter()
        .find(|environment| environment.environment_id == "local")
        .or_else(|| config.environments.first())
        .ok_or_else(|| Error::InvalidInput("no environment is configured".to_string()))
}

fn validate_environment(environment: &EnvironmentConfig) -> Result<()> {
    if environment.environment_id.trim().is_empty() {
        return Err(Error::InvalidInput(
            "environment id must not be empty".to_string(),
        ));
    }

    for agent in &environment.agents {
        if agent.managed && agent.skills_dir.as_os_str().is_empty() {
            return Err(Error::InvalidInput(format!(
                "skillsDir is required for managed agent {} in environment {}",
                agent.agent_id, environment.environment_id
            )));
        }
    }

    Ok(())
}

pub fn read_workspace_config(path: &Path) -> Result<WorkspaceConfig> {
    let body = fs::read_to_string(path)
        .map_err(|err| Error::io(format!("failed to read {}", path.display()), err))?;
    let config: WorkspaceConfig = serde_json::from_str(&body).map_err(|err| Error::Json {
        context: format!("failed to parse {}", path.display()),
        source: err,
    })?;
    validate_workspace_config(&config)?;
    Ok(config)
}

pub fn write_workspace_config(path: &Path, config: &WorkspaceConfig) -> Result<()> {
    write_json(path, config)
}

pub fn read_workspace_state(path: &Path) -> Result<WorkspaceState> {
    if !path.exists() {
        return Ok(WorkspaceState::default());
    }
    let body = fs::read_to_string(path)
        .map_err(|err| Error::io(format!("failed to read {}", path.display()), err))?;
    serde_json::from_str(&body).map_err(|err| Error::Json {
        context: format!("failed to parse {}", path.display()),
        source: err,
    })
}

pub fn write_workspace_state(path: &Path, state: &WorkspaceState) -> Result<()> {
    write_json(path, state)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| Error::io(format!("failed to create {}", parent.display()), err))?;
    }
    let body = serde_json::to_string_pretty(value).map_err(|err| Error::Json {
        context: format!("failed to serialize {}", path.display()),
        source: err,
    })?;
    fs::write(path, format!("{body}\n"))
        .map_err(|err| Error::io(format!("failed to write {}", path.display()), err))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{LocalSourceProfile, SourceProfile};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn round_trips_workspace_config() {
        let root = test_dir("config");
        let path = root.join(CONFIG_FILE);
        let config = WorkspaceConfig {
            schema_version: 1,
            active_source_profile_id: "local".to_string(),
            source_profiles: vec![SourceProfile::Local(LocalSourceProfile {
                source_profile_id: "local".to_string(),
                source_root: PathBuf::from("/tmp/skills"),
            })],
            environments: Vec::new(),
        };

        write_workspace_config(&path, &config).unwrap();
        assert_eq!(read_workspace_config(&path).unwrap(), config);
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
