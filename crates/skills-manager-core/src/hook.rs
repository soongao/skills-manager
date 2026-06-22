use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::agents::AgentId;
use crate::detect::{detect_agent, CapabilityStatus};
use crate::error::{Error, Result};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HookInstallStatus {
    NotInstalled,
    Installed,
    NeedsUpdate,
    ConfigConflict,
    Unsupported,
    VersionUnverified,
}

impl HookInstallStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotInstalled => "not-installed",
            Self::Installed => "installed",
            Self::NeedsUpdate => "needs-update",
            Self::ConfigConflict => "config-conflict",
            Self::Unsupported => "unsupported",
            Self::VersionUnverified => "version-unverified",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookStatus {
    pub agent_id: AgentId,
    pub status: HookInstallStatus,
    pub config_paths: Vec<PathBuf>,
    pub install_supported: bool,
    pub reason: String,
}

pub fn hook_status(agent_id: AgentId) -> HookStatus {
    let detection = detect_agent(agent_id);
    let config_paths = hook_config_candidates(agent_id);
    let status = if detection.hook.install_supported {
        HookInstallStatus::NotInstalled
    } else if detection.hook.status == CapabilityStatus::Unknown {
        HookInstallStatus::VersionUnverified
    } else {
        HookInstallStatus::Unsupported
    };

    HookStatus {
        agent_id,
        status,
        config_paths,
        install_supported: detection.hook.install_supported,
        reason: detection.hook.reason,
    }
}

pub fn install_hook(agent_id: AgentId) -> Result<HookStatus> {
    let status = hook_status(agent_id);
    if status.install_supported {
        return Ok(status);
    }

    Err(Error::InvalidInput(format!(
        "hook install is not supported for {}: {}",
        agent_id, status.reason
    )))
}

pub fn hook_config_candidates(agent_id: AgentId) -> Vec<PathBuf> {
    let Some(home) = crate::paths::home_dir() else {
        return Vec::new();
    };

    match agent_id {
        AgentId::Codex => vec![
            home.join(".codex/hooks.json"),
            home.join(".codex/config.toml"),
        ],
        AgentId::ClaudeCode => {
            if let Some(config_dir) = std::env::var_os("CLAUDE_CONFIG_DIR").map(PathBuf::from) {
                vec![config_dir.join("settings.json")]
            } else {
                vec![home.join(".claude/settings.json")]
            }
        }
        AgentId::OpenCode => vec![home.join(".config/opencode/opencode.json")],
    }
}
