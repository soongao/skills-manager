use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::agents::AgentId;
use crate::config::{ManagedLink, WorkspaceState};
use crate::error::{Error, Result};
use crate::model::{EnvironmentConfig, EnvironmentKind, Skill};
use crate::paths::expand_path_from_cwd;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReconcileMode {
    Plan,
    Apply,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionKind {
    CreateSymlink,
    RemoveSymlink,
    SkipDisabled,
    Conflict,
    Invalid,
}

impl ActionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ActionKind::CreateSymlink => "create-symlink",
            ActionKind::RemoveSymlink => "remove-symlink",
            ActionKind::SkipDisabled => "skip-disabled",
            ActionKind::Conflict => "conflict",
            ActionKind::Invalid => "invalid",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionStatus {
    Planned,
    Applied,
    Skipped,
    Conflict,
    Failed,
}

impl ActionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ActionStatus::Planned => "planned",
            ActionStatus::Applied => "applied",
            ActionStatus::Skipped => "skipped",
            ActionStatus::Conflict => "conflict",
            ActionStatus::Failed => "failed",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconcileAction {
    pub kind: ActionKind,
    pub status: ActionStatus,
    pub environment_id: String,
    pub agent_id: AgentId,
    pub skill_id: String,
    pub source_path: Option<PathBuf>,
    pub target_path: PathBuf,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconcileReport {
    pub environment_id: String,
    pub agent_id: AgentId,
    pub actions: Vec<ReconcileAction>,
}

pub fn reconcile_agent(
    env_config: &EnvironmentConfig,
    agent_id: AgentId,
    skills: &[Skill],
    mode: ReconcileMode,
) -> Result<ReconcileReport> {
    let mut state = WorkspaceState::default();
    reconcile_agent_with_state(env_config, agent_id, skills, mode, &mut state)
}

pub fn reconcile_agent_with_state(
    env_config: &EnvironmentConfig,
    agent_id: AgentId,
    skills: &[Skill],
    mode: ReconcileMode,
    state: &mut WorkspaceState,
) -> Result<ReconcileReport> {
    let agent = env_config
        .agent(agent_id)
        .ok_or_else(|| Error::AgentNotConfigured(agent_id.to_string()))?;

    if !agent.managed {
        return Ok(ReconcileReport {
            environment_id: env_config.environment_id.clone(),
            agent_id,
            actions: Vec::new(),
        });
    }

    let skills_dir = runtime_skills_dir(env_config, &agent.skills_dir)?;
    ensure_skills_dir(&skills_dir, mode)?;

    let skills_by_id: HashMap<&str, &Skill> = skills
        .iter()
        .map(|skill| (skill.skill_id.as_str(), skill))
        .collect();

    let mut actions = Vec::new();
    for skill_id in &agent.enabled_skill_ids {
        let target_path = skills_dir.join(skill_id);
        let Some(skill) = skills_by_id.get(skill_id.as_str()) else {
            actions.push(ReconcileAction {
                kind: ActionKind::Conflict,
                status: ActionStatus::Conflict,
                environment_id: env_config.environment_id.clone(),
                agent_id,
                skill_id: skill_id.clone(),
                source_path: None,
                target_path,
                message: Some("source skill is missing".to_string()),
            });
            continue;
        };

        actions.push(reconcile_enabled_skill(
            &env_config.environment_id,
            agent_id,
            skill,
            target_path,
            mode,
            state,
        )?);
    }

    let enabled: std::collections::HashSet<&str> =
        agent.enabled_skill_ids.iter().map(String::as_str).collect();
    let managed_links = state.managed_links.clone();
    for link in managed_links {
        if link.environment_id != env_config.environment_id
            || link.agent_id != agent_id
            || enabled.contains(link.skill_id.as_str())
        {
            continue;
        }
        actions.push(reconcile_disabled_managed_skill(
            env_config, agent_id, &link, mode, state,
        )?);
    }

    Ok(ReconcileReport {
        environment_id: env_config.environment_id.clone(),
        agent_id,
        actions,
    })
}

fn ensure_skills_dir(path: &Path, mode: ReconcileMode) -> Result<()> {
    if path.exists() {
        if path.is_dir() {
            return Ok(());
        }
        return Err(Error::AgentSkillsDirInvalid(path.display().to_string()));
    }

    if mode == ReconcileMode::Apply {
        fs::create_dir_all(path)
            .map_err(|err| Error::io(format!("failed to create {}", path.display()), err))?;
    }
    Ok(())
}

fn runtime_skills_dir(env_config: &EnvironmentConfig, path: &Path) -> Result<PathBuf> {
    if env_config.kind != EnvironmentKind::Local {
        return Ok(path.to_path_buf());
    }

    expand_path_from_cwd(path)
}

fn reconcile_enabled_skill(
    environment_id: &str,
    agent_id: AgentId,
    skill: &Skill,
    target_path: PathBuf,
    mode: ReconcileMode,
    state: &mut WorkspaceState,
) -> Result<ReconcileAction> {
    if !skill.path.exists() {
        return Ok(ReconcileAction {
            kind: ActionKind::Invalid,
            status: ActionStatus::Failed,
            environment_id: environment_id.to_string(),
            agent_id,
            skill_id: skill.skill_id.clone(),
            source_path: Some(skill.path.clone()),
            target_path,
            message: Some("source skill path does not exist".to_string()),
        });
    }

    match fs::symlink_metadata(&target_path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                let current_target = fs::read_link(&target_path).map_err(|err| {
                    Error::io(format!("failed to read {}", target_path.display()), err)
                })?;
                if current_target.as_path() == skill.path.as_path() {
                    register_managed_link(
                        state,
                        environment_id,
                        agent_id,
                        &skill.skill_id,
                        &skill.path,
                        &target_path,
                    );
                    return Ok(ReconcileAction {
                        kind: ActionKind::CreateSymlink,
                        status: ActionStatus::Skipped,
                        environment_id: environment_id.to_string(),
                        agent_id,
                        skill_id: skill.skill_id.clone(),
                        source_path: Some(skill.path.clone()),
                        target_path,
                        message: Some("managed target already points to source".to_string()),
                    });
                }
            }

            Ok(ReconcileAction {
                kind: ActionKind::Conflict,
                status: ActionStatus::Conflict,
                environment_id: environment_id.to_string(),
                agent_id,
                skill_id: skill.skill_id.clone(),
                source_path: Some(skill.path.clone()),
                target_path,
                message: Some("target path already exists".to_string()),
            })
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            if mode == ReconcileMode::Apply {
                create_dir_symlink(&skill.path, &target_path)?;
                register_managed_link(
                    state,
                    environment_id,
                    agent_id,
                    &skill.skill_id,
                    &skill.path,
                    &target_path,
                );
            }

            Ok(ReconcileAction {
                kind: ActionKind::CreateSymlink,
                status: if mode == ReconcileMode::Apply {
                    ActionStatus::Applied
                } else {
                    ActionStatus::Planned
                },
                environment_id: environment_id.to_string(),
                agent_id,
                skill_id: skill.skill_id.clone(),
                source_path: Some(skill.path.clone()),
                target_path,
                message: None,
            })
        }
        Err(err) => Err(Error::io(
            format!("failed to inspect {}", target_path.display()),
            err,
        )),
    }
}

fn reconcile_disabled_managed_skill(
    env_config: &EnvironmentConfig,
    agent_id: AgentId,
    link: &ManagedLink,
    mode: ReconcileMode,
    state: &mut WorkspaceState,
) -> Result<ReconcileAction> {
    let target_path = runtime_managed_link_path(env_config, &link.target_path)?;
    let source_path = runtime_managed_link_path(env_config, &link.source_path)?;

    match fs::symlink_metadata(&target_path) {
        Ok(metadata) => {
            if !metadata.file_type().is_symlink() {
                return Ok(ReconcileAction {
                    kind: ActionKind::Conflict,
                    status: ActionStatus::Conflict,
                    environment_id: env_config.environment_id.clone(),
                    agent_id,
                    skill_id: link.skill_id.clone(),
                    source_path: Some(source_path),
                    target_path,
                    message: Some("managed target is no longer a symlink".to_string()),
                });
            }

            let current_target = fs::read_link(&target_path).map_err(|err| {
                Error::io(format!("failed to read {}", target_path.display()), err)
            })?;
            if current_target != source_path {
                return Ok(ReconcileAction {
                    kind: ActionKind::Conflict,
                    status: ActionStatus::Conflict,
                    environment_id: env_config.environment_id.clone(),
                    agent_id,
                    skill_id: link.skill_id.clone(),
                    source_path: Some(source_path),
                    target_path,
                    message: Some("managed target points elsewhere".to_string()),
                });
            }

            if mode == ReconcileMode::Apply {
                fs::remove_file(&target_path).map_err(|err| {
                    Error::io(format!("failed to remove {}", target_path.display()), err)
                })?;
                unregister_managed_link(
                    state,
                    &env_config.environment_id,
                    agent_id,
                    &link.skill_id,
                );
            }

            Ok(ReconcileAction {
                kind: ActionKind::RemoveSymlink,
                status: if mode == ReconcileMode::Apply {
                    ActionStatus::Applied
                } else {
                    ActionStatus::Planned
                },
                environment_id: env_config.environment_id.clone(),
                agent_id,
                skill_id: link.skill_id.clone(),
                source_path: Some(source_path),
                target_path,
                message: None,
            })
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            unregister_managed_link(state, &env_config.environment_id, agent_id, &link.skill_id);
            Ok(ReconcileAction {
                kind: ActionKind::RemoveSymlink,
                status: ActionStatus::Skipped,
                environment_id: env_config.environment_id.clone(),
                agent_id,
                skill_id: link.skill_id.clone(),
                source_path: Some(source_path),
                target_path,
                message: Some("managed target already missing".to_string()),
            })
        }
        Err(err) => Err(Error::io(
            format!("failed to inspect {}", target_path.display()),
            err,
        )),
    }
}

fn runtime_managed_link_path(env_config: &EnvironmentConfig, path: &Path) -> Result<PathBuf> {
    if env_config.kind != EnvironmentKind::Local {
        return Ok(path.to_path_buf());
    }

    expand_path_from_cwd(path)
}

fn register_managed_link(
    state: &mut WorkspaceState,
    environment_id: &str,
    agent_id: AgentId,
    skill_id: &str,
    source_path: &Path,
    target_path: &Path,
) {
    unregister_managed_link(state, environment_id, agent_id, skill_id);
    state.managed_links.push(ManagedLink {
        environment_id: environment_id.to_string(),
        agent_id,
        skill_id: skill_id.to_string(),
        source_path: source_path.to_path_buf(),
        target_path: target_path.to_path_buf(),
    });
}

fn unregister_managed_link(
    state: &mut WorkspaceState,
    environment_id: &str,
    agent_id: AgentId,
    skill_id: &str,
) {
    state.managed_links.retain(|link| {
        !(link.environment_id == environment_id
            && link.agent_id == agent_id
            && link.skill_id == skill_id)
    });
}

#[cfg(unix)]
fn create_dir_symlink(source: &Path, target: &Path) -> Result<()> {
    std::os::unix::fs::symlink(source, target).map_err(|err| {
        Error::io(
            format!(
                "failed to link {} -> {}",
                target.display(),
                source.display()
            ),
            err,
        )
    })
}

#[cfg(windows)]
fn create_dir_symlink(source: &Path, target: &Path) -> Result<()> {
    std::os::windows::fs::symlink_dir(source, target).map_err(|err| {
        Error::io(
            format!(
                "failed to link {} -> {}",
                target.display(),
                source.display()
            ),
            err,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AgentConfig, EnvironmentConfig};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn creates_symlink_for_enabled_skill() {
        let root = test_dir("reconcile");
        let source = root.join("source/skills/design-clarifier");
        let skills_dir = root.join("agent-skills");
        fs::create_dir_all(&source).unwrap();

        let skill = Skill {
            skill_id: "design-clarifier".to_string(),
            path: source.clone(),
        };
        let env = EnvironmentConfig::local(
            "local",
            vec![AgentConfig {
                agent_id: AgentId::ClaudeCode,
                managed: true,
                skills_dir: skills_dir.clone(),
                enabled_skill_ids: vec!["design-clarifier".to_string()],
            }],
        );

        let report =
            reconcile_agent(&env, AgentId::ClaudeCode, &[skill], ReconcileMode::Apply).unwrap();
        assert_eq!(report.actions.len(), 1);
        assert_eq!(report.actions[0].status, ActionStatus::Applied);
        assert_eq!(
            fs::read_link(skills_dir.join("design-clarifier")).unwrap(),
            source
        );
    }

    #[test]
    fn reports_conflict_when_target_exists() {
        let root = test_dir("conflict");
        let source = root.join("source/skills/design-clarifier");
        let skills_dir = root.join("agent-skills");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(skills_dir.join("design-clarifier")).unwrap();

        let skill = Skill {
            skill_id: "design-clarifier".to_string(),
            path: source,
        };
        let env = EnvironmentConfig::local(
            "local",
            vec![AgentConfig {
                agent_id: AgentId::ClaudeCode,
                managed: true,
                skills_dir,
                enabled_skill_ids: vec!["design-clarifier".to_string()],
            }],
        );

        let report =
            reconcile_agent(&env, AgentId::ClaudeCode, &[skill], ReconcileMode::Apply).unwrap();
        assert_eq!(report.actions[0].status, ActionStatus::Conflict);
    }

    #[test]
    fn removes_only_registered_managed_symlink_when_disabled() {
        let root = test_dir("disable");
        let source = root.join("source/skills/design-clarifier");
        let skills_dir = root.join("agent-skills");
        let target = skills_dir.join("design-clarifier");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&skills_dir).unwrap();
        create_dir_symlink(&source, &target).unwrap();

        let skill = Skill {
            skill_id: "design-clarifier".to_string(),
            path: source.clone(),
        };
        let env = EnvironmentConfig::local(
            "local",
            vec![AgentConfig {
                agent_id: AgentId::ClaudeCode,
                managed: true,
                skills_dir: skills_dir.clone(),
                enabled_skill_ids: Vec::new(),
            }],
        );
        let mut state = WorkspaceState {
            schema_version: 1,
            managed_links: vec![ManagedLink {
                environment_id: "local".to_string(),
                agent_id: AgentId::ClaudeCode,
                skill_id: "design-clarifier".to_string(),
                source_path: source,
                target_path: target.clone(),
            }],
            sync: Vec::new(),
        };

        let report = reconcile_agent_with_state(
            &env,
            AgentId::ClaudeCode,
            &[skill],
            ReconcileMode::Apply,
            &mut state,
        )
        .unwrap();

        assert_eq!(report.actions[0].status, ActionStatus::Applied);
        assert!(!target.exists());
        assert!(state.managed_links.is_empty());
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
