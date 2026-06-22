use std::env;
use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::agents::{AgentId, BUILTIN_AGENTS};
use crate::paths::home_dir;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityStatus {
    Available,
    MissingDependency,
    PermissionDenied,
    PathMissing,
    Unknown,
}

impl CapabilityStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::MissingDependency => "missing-dependency",
            Self::PermissionDenied => "permission-denied",
            Self::PathMissing => "path-missing",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandDetection {
    pub name: String,
    pub status: CapabilityStatus,
    pub path: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDetection {
    pub agent_id: AgentId,
    pub command: Option<CommandDetection>,
    pub candidate_skills_dirs: Vec<PathCandidate>,
    pub recommended_skills_dir: Option<PathBuf>,
    pub hook: HookDetection,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookDetection {
    pub status: CapabilityStatus,
    pub install_supported: bool,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathCandidate {
    pub path: PathBuf,
    pub exists: bool,
    pub source: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineDetection {
    pub os: String,
    pub arch: String,
    pub config_home: PathBuf,
    pub symlink: CapabilityStatus,
    pub ssh: CommandDetection,
    pub rsync: CommandDetection,
    pub agents: Vec<AgentDetection>,
}

pub fn detect_machine(config_home: PathBuf) -> MachineDetection {
    MachineDetection {
        os: env::consts::OS.to_string(),
        arch: env::consts::ARCH.to_string(),
        config_home,
        symlink: detect_symlink_capability(),
        ssh: detect_command("ssh"),
        rsync: detect_command("rsync"),
        agents: BUILTIN_AGENTS
            .into_iter()
            .map(detect_agent)
            .collect::<Vec<_>>(),
    }
}

pub fn detect_agent(agent_id: AgentId) -> AgentDetection {
    AgentDetection {
        agent_id,
        command: agent_command(agent_id).map(detect_command),
        candidate_skills_dirs: candidate_skills_dirs(agent_id),
        recommended_skills_dir: recommended_skills_dir(agent_id),
        hook: hook_detection(agent_id),
    }
}

pub fn candidate_skills_dirs(agent_id: AgentId) -> Vec<PathCandidate> {
    let mut candidates = Vec::new();
    match agent_id {
        AgentId::Codex => {
            if let Some(codex_home) = env::var_os("CODEX_HOME").map(PathBuf::from) {
                candidates.push(candidate(codex_home.join("skills"), "$CODEX_HOME/skills"));
            }
            if let Some(home) = home_dir() {
                candidates.push(candidate(home.join(".codex/skills"), "~/.codex/skills"));
                candidates.push(candidate(home.join(".agents/skills"), "~/.agents/skills"));
            }
        }
        AgentId::ClaudeCode => {
            if let Some(config_dir) = env::var_os("CLAUDE_CONFIG_DIR").map(PathBuf::from) {
                candidates.push(candidate(
                    config_dir.join("skills"),
                    "$CLAUDE_CONFIG_DIR/skills",
                ));
            }
            if let Some(home) = home_dir() {
                candidates.push(candidate(home.join(".claude/skills"), "~/.claude/skills"));
            }
        }
        AgentId::OpenCode => {
            if let Some(config_home) = env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
                candidates.push(candidate(
                    config_home.join("opencode/skills"),
                    "$XDG_CONFIG_HOME/opencode/skills",
                ));
            }
            if let Some(home) = home_dir() {
                candidates.push(candidate(
                    home.join(".config/opencode/skills"),
                    "~/.config/opencode/skills",
                ));
                candidates.push(candidate(home.join(".agents/skills"), "~/.agents/skills"));
                candidates.push(candidate(home.join(".claude/skills"), "~/.claude/skills"));
            }
        }
    }

    dedupe_candidates(candidates)
}

pub fn recommended_skills_dir(agent_id: AgentId) -> Option<PathBuf> {
    let candidates = candidate_skills_dirs(agent_id);
    candidates
        .iter()
        .find(|candidate| candidate.exists)
        .or_else(|| candidates.first())
        .map(|candidate| candidate.path.clone())
}

pub fn detect_command(name: &str) -> CommandDetection {
    let status = Command::new(name)
        .arg("--version")
        .output()
        .map(|_| CapabilityStatus::Available)
        .unwrap_or(CapabilityStatus::MissingDependency);

    CommandDetection {
        name: name.to_string(),
        status,
        path: None,
    }
}

fn agent_command(agent_id: AgentId) -> Option<&'static str> {
    match agent_id {
        AgentId::Codex => Some("codex"),
        AgentId::ClaudeCode => Some("claude"),
        AgentId::OpenCode => Some("opencode"),
    }
}

fn candidate(path: PathBuf, source: &str) -> PathCandidate {
    PathCandidate {
        exists: path.is_dir(),
        path,
        source: source.to_string(),
    }
}

fn hook_detection(agent_id: AgentId) -> HookDetection {
    let reason = match agent_id {
        AgentId::Codex => {
            "Codex SessionStart hook exists, but this version has not been verified to run before skill discovery."
        }
        AgentId::ClaudeCode => {
            "Claude Code session/setup hooks exist, but this version has not been verified to run before skill discovery."
        }
        AgentId::OpenCode => {
            "OpenCode plugin hooks are available, but no generic pre-skill-discovery hook has been verified."
        }
    };

    HookDetection {
        status: CapabilityStatus::Unknown,
        install_supported: false,
        reason: reason.to_string(),
    }
}

fn detect_symlink_capability() -> CapabilityStatus {
    #[cfg(unix)]
    {
        CapabilityStatus::Available
    }

    #[cfg(windows)]
    {
        CapabilityStatus::Unknown
    }
}

fn dedupe_candidates(candidates: Vec<PathCandidate>) -> Vec<PathCandidate> {
    let mut out = Vec::new();
    for candidate in candidates {
        if out
            .iter()
            .any(|existing: &PathCandidate| existing.path == candidate.path)
        {
            continue;
        }
        out.push(candidate);
    }
    out
}
