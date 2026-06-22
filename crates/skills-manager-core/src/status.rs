use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::agents::AgentId;
use crate::model::{EnvironmentConfig, EnvironmentKind, Skill};
use crate::paths::expand_path_from_cwd;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkillStatus {
    Disabled,
    Pending,
    Enabled,
    Conflict,
    Invalid,
}

impl SkillStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            SkillStatus::Disabled => "disabled",
            SkillStatus::Pending => "pending",
            SkillStatus::Enabled => "enabled",
            SkillStatus::Conflict => "conflict",
            SkillStatus::Invalid => "invalid",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillAgentStatus {
    pub environment_id: String,
    pub agent_id: AgentId,
    pub skill_id: String,
    pub status: SkillStatus,
    pub source_path: Option<PathBuf>,
    pub target_path: PathBuf,
    pub message: Option<String>,
}

pub fn compute_agent_statuses(
    env_config: &EnvironmentConfig,
    agent_id: AgentId,
    skills: &[Skill],
) -> Vec<SkillAgentStatus> {
    let Some(agent) = env_config.agent(agent_id) else {
        return Vec::new();
    };

    let skills_by_id: HashMap<&str, &Skill> = skills
        .iter()
        .map(|skill| (skill.skill_id.as_str(), skill))
        .collect();

    let mut rows = Vec::new();
    for skill in skills {
        let enabled = agent
            .enabled_skill_ids
            .iter()
            .any(|id| id == &skill.skill_id);
        let skills_dir = display_skills_dir(env_config, &agent.skills_dir);
        let target_path = skills_dir.join(&skill.skill_id);
        rows.push(compute_one_status(
            &env_config.environment_id,
            agent_id,
            &skill.skill_id,
            Some(skill.path.clone()),
            target_path,
            enabled,
        ));
    }

    for skill_id in &agent.enabled_skill_ids {
        if skills_by_id.contains_key(skill_id.as_str()) {
            continue;
        }
        let skills_dir = display_skills_dir(env_config, &agent.skills_dir);
        let target_path = skills_dir.join(skill_id);
        rows.push(SkillAgentStatus {
            environment_id: env_config.environment_id.clone(),
            agent_id,
            skill_id: skill_id.clone(),
            status: SkillStatus::Invalid,
            source_path: None,
            target_path,
            message: Some("enabled skill is missing from source".to_string()),
        });
    }

    rows.sort_by(|left, right| left.skill_id.cmp(&right.skill_id));
    rows
}

fn display_skills_dir(env_config: &EnvironmentConfig, path: &std::path::Path) -> PathBuf {
    if env_config.kind != EnvironmentKind::Local {
        return path.to_path_buf();
    }

    expand_path_from_cwd(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::AgentId;
    use crate::model::AgentConfig;
    use crate::paths::home_dir;

    #[test]
    fn expands_local_home_prefixed_skills_dir() {
        let Some(home) = home_dir() else {
            return;
        };
        let source = home.join(".skills-manager-test/source/skills/design-clarifier");
        let env = EnvironmentConfig::local(
            "local",
            vec![AgentConfig {
                agent_id: AgentId::Codex,
                managed: true,
                skills_dir: PathBuf::from("~/.codex/skills"),
                enabled_skill_ids: vec!["design-clarifier".to_string()],
            }],
        );
        let statuses = compute_agent_statuses(
            &env,
            AgentId::Codex,
            &[Skill {
                skill_id: "design-clarifier".to_string(),
                path: source,
            }],
        );

        assert_eq!(
            statuses[0].target_path,
            home.join(".codex/skills/design-clarifier")
        );
    }
}

fn compute_one_status(
    environment_id: &str,
    agent_id: AgentId,
    skill_id: &str,
    source_path: Option<PathBuf>,
    target_path: PathBuf,
    enabled: bool,
) -> SkillAgentStatus {
    if !enabled {
        return SkillAgentStatus {
            environment_id: environment_id.to_string(),
            agent_id,
            skill_id: skill_id.to_string(),
            status: SkillStatus::Disabled,
            source_path,
            target_path,
            message: None,
        };
    }

    if environment_id != "local" {
        return SkillAgentStatus {
            environment_id: environment_id.to_string(),
            agent_id,
            skill_id: skill_id.to_string(),
            status: SkillStatus::Pending,
            source_path,
            target_path,
            message: Some("remote target will be linked on the remote machine".to_string()),
        };
    }

    let Some(source) = source_path.clone() else {
        return SkillAgentStatus {
            environment_id: environment_id.to_string(),
            agent_id,
            skill_id: skill_id.to_string(),
            status: SkillStatus::Invalid,
            source_path,
            target_path,
            message: Some("source skill is missing".to_string()),
        };
    };

    match fs::symlink_metadata(&target_path) {
        Ok(metadata) => {
            if !metadata.file_type().is_symlink() {
                return SkillAgentStatus {
                    environment_id: environment_id.to_string(),
                    agent_id,
                    skill_id: skill_id.to_string(),
                    status: SkillStatus::Conflict,
                    source_path: Some(source.clone()),
                    target_path,
                    message: Some("target path exists and is not a symlink".to_string()),
                };
            }

            match fs::read_link(&target_path) {
                Ok(current) if current.as_path() == source.as_path() => SkillAgentStatus {
                    environment_id: environment_id.to_string(),
                    agent_id,
                    skill_id: skill_id.to_string(),
                    status: SkillStatus::Enabled,
                    source_path: Some(source.clone()),
                    target_path,
                    message: None,
                },
                Ok(_) => SkillAgentStatus {
                    environment_id: environment_id.to_string(),
                    agent_id,
                    skill_id: skill_id.to_string(),
                    status: SkillStatus::Conflict,
                    source_path: Some(source.clone()),
                    target_path,
                    message: Some("target symlink points elsewhere".to_string()),
                },
                Err(_) => SkillAgentStatus {
                    environment_id: environment_id.to_string(),
                    agent_id,
                    skill_id: skill_id.to_string(),
                    status: SkillStatus::Invalid,
                    source_path: Some(source.clone()),
                    target_path,
                    message: Some("target symlink cannot be read".to_string()),
                },
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => SkillAgentStatus {
            environment_id: environment_id.to_string(),
            agent_id,
            skill_id: skill_id.to_string(),
            status: SkillStatus::Pending,
            source_path: Some(source.clone()),
            target_path,
            message: None,
        },
        Err(_) => SkillAgentStatus {
            environment_id: environment_id.to_string(),
            agent_id,
            skill_id: skill_id.to_string(),
            status: SkillStatus::Invalid,
            source_path: Some(source),
            target_path,
            message: Some("target path cannot be inspected".to_string()),
        },
    }
}
