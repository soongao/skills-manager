use std::path::PathBuf;

use crate::agents::AgentId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceConfig {
    pub schema_version: u32,
    pub active_source_profile_id: String,
    pub source_profiles: Vec<SourceProfile>,
    pub environments: Vec<EnvironmentConfig>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SourceProfile {
    Local(LocalSourceProfile),
    Remote(RemoteSourceProfile),
}

impl SourceProfile {
    pub fn source_profile_id(&self) -> &str {
        match self {
            SourceProfile::Local(profile) => &profile.source_profile_id,
            SourceProfile::Remote(profile) => &profile.source_profile_id,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalSourceProfile {
    pub source_profile_id: String,
    pub source_root: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSourceProfile {
    pub source_profile_id: String,
    pub host: String,
    pub user: String,
    pub remote_source_root: PathBuf,
    pub local_cache_root: PathBuf,
    pub auto_sync: bool,
    pub delete_extraneous: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EnvironmentKind {
    Local,
    Remote,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SyncDirection {
    PushLocalToRemote,
    PullRemoteToLocal,
}

impl SyncDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            SyncDirection::PushLocalToRemote => "push-local-to-remote",
            SyncDirection::PullRemoteToLocal => "pull-remote-to-local",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub skill_id: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    pub agent_id: AgentId,
    pub managed: bool,
    pub skills_dir: PathBuf,
    pub enabled_skill_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentConfig {
    pub environment_id: String,
    pub kind: EnvironmentKind,
    pub host: Option<String>,
    pub user: Option<String>,
    pub sync_direction: Option<SyncDirection>,
    pub remote_cache_root: Option<PathBuf>,
    pub auto_sync: bool,
    pub delete_extraneous: bool,
    pub agents: Vec<AgentConfig>,
}

impl EnvironmentConfig {
    pub fn local(environment_id: impl Into<String>, agents: Vec<AgentConfig>) -> Self {
        Self {
            environment_id: environment_id.into(),
            kind: EnvironmentKind::Local,
            host: None,
            user: None,
            sync_direction: None,
            remote_cache_root: None,
            auto_sync: false,
            delete_extraneous: true,
            agents,
        }
    }

    pub fn agent(&self, agent_id: AgentId) -> Option<&AgentConfig> {
        self.agents.iter().find(|agent| agent.agent_id == agent_id)
    }
}
