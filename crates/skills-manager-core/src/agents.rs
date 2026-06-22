use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum AgentId {
    #[serde(rename = "codex")]
    Codex,
    #[serde(rename = "claude-code")]
    ClaudeCode,
    #[serde(rename = "opencode")]
    OpenCode,
}

impl AgentId {
    pub fn as_str(self) -> &'static str {
        match self {
            AgentId::Codex => "codex",
            AgentId::ClaudeCode => "claude-code",
            AgentId::OpenCode => "opencode",
        }
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AgentId {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "codex" => Ok(Self::Codex),
            "claude-code" => Ok(Self::ClaudeCode),
            "opencode" => Ok(Self::OpenCode),
            _ => Err(format!("unknown agent id: {value}")),
        }
    }
}

pub const BUILTIN_AGENTS: [AgentId; 3] = [AgentId::Codex, AgentId::ClaudeCode, AgentId::OpenCode];
