use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;
use serde_json::{json, Value};
use skills_manager_core::agents::{AgentId, BUILTIN_AGENTS};
use skills_manager_core::cache::CacheMarker;
use skills_manager_core::config::{
    active_source_profile, config_path, environment, init_config_home, read_workspace_config,
    read_workspace_state, state_path, write_workspace_config, write_workspace_state,
};
use skills_manager_core::detect::detect_machine;
use skills_manager_core::hook::hook_status;
use skills_manager_core::model::{
    AgentConfig, EnvironmentConfig, EnvironmentKind, LocalSourceProfile, RemoteSourceProfile,
    SourceProfile, SyncDirection, WorkspaceConfig,
};
use skills_manager_core::opencode::ensure_opencode_skill_path;
use skills_manager_core::paths::{default_config_home, expand_path_from_cwd};
use skills_manager_core::reconcile::{reconcile_agent_with_state, ReconcileMode};
use skills_manager_core::remote_link::{execute_remote_link_plan, plan_remote_links};
use skills_manager_core::repository::init_or_update_repository_metadata;
use skills_manager_core::scan::scan_source;
use skills_manager_core::status::compute_agent_statuses;
use skills_manager_core::sync::{
    execute_sync_plan, plan_pull_remote_to_local, plan_push_local_to_remote, PullRemoteToLocal,
    PushLocalToRemote,
};

pub type CommandResult = Result<Value, CommandError>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceSkills {
    source_profile_id: String,
    kind: &'static str,
    source_root: PathBuf,
    skills: Vec<skills_manager_core::model::Skill>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    code: String,
    message: String,
}

pub fn handle<F>(f: F) -> CommandResult
where
    F: FnOnce() -> skills_manager_core::Result<Value>,
{
    f().map_err(|error| CommandError {
        code: error_code(&error).to_string(),
        message: error.to_string(),
    })
}

#[tauri::command]
pub fn load_dashboard(config_home: Option<String>) -> CommandResult {
    handle(|| {
        let config_home = config_home_path(config_home);
        let config_path = config_path(&config_home);
        let config = if config_path.exists() {
            Some(read_workspace_config(&config_path)?)
        } else {
            None
        };
        let state = read_workspace_state(&state_path(&config_home))?;
        let detection = detect_machine(config_home.clone());
        let source_root = config
            .as_ref()
            .and_then(|config| resolve_active_source_root(config).ok());
        let skills = source_root
            .as_ref()
            .and_then(|source_root| scan_source(source_root).ok())
            .unwrap_or_default();
        let source_skills = config
            .as_ref()
            .map(scan_source_profiles)
            .transpose()?
            .unwrap_or_default();
        let mut statuses = Vec::new();
        if let Some(config) = &config {
            for env_config in &config.environments {
                for agent in &env_config.agents {
                    statuses.extend(compute_agent_statuses(env_config, agent.agent_id, &skills));
                }
            }
        }
        let hooks = BUILTIN_AGENTS
            .into_iter()
            .map(hook_status)
            .collect::<Vec<_>>();

        Ok(json!({
            "configHome": config_home,
            "config": config,
            "state": state,
            "detection": detection,
            "sourceRoot": source_root,
            "skills": skills,
            "sourceSkills": source_skills,
            "statuses": statuses,
            "hooks": hooks,
        }))
    })
}

#[tauri::command]
pub fn init_config(
    config_home: Option<String>,
    source_root: String,
    codex_skills_dir: Option<String>,
    claude_code_skills_dir: Option<String>,
    opencode_skills_dir: Option<String>,
) -> CommandResult {
    handle(|| {
        let config_home = config_home_path(config_home);
        init_config_home(&config_home)?;
        let source_root = expand_path_from_cwd(Path::new(&source_root))?;
        let repository = init_or_update_repository_metadata(&source_root, None)?;
        let skills = scan_source(&source_root)?;
        let enabled_skill_ids = skills
            .iter()
            .map(|skill| skill.skill_id.clone())
            .collect::<Vec<_>>();

        let mut agents = Vec::new();
        for agent_id in BUILTIN_AGENTS {
            let selected = match agent_id {
                AgentId::Codex => codex_skills_dir.clone(),
                AgentId::ClaudeCode => claude_code_skills_dir.clone(),
                AgentId::OpenCode => opencode_skills_dir.clone(),
            };
            let skills_dir = selected
                .map(|path| expand_path_from_cwd(Path::new(&path)))
                .transpose()?
                .or_else(|| skills_manager_core::detect::recommended_skills_dir(agent_id));
            if let Some(skills_dir) = skills_dir {
                agents.push(AgentConfig {
                    agent_id,
                    managed: true,
                    skills_dir,
                    enabled_skill_ids: enabled_skill_ids.clone(),
                });
            }
        }

        let config = WorkspaceConfig {
            schema_version: 1,
            active_source_profile_id: "local-personal".to_string(),
            source_profiles: vec![SourceProfile::Local(LocalSourceProfile {
                source_profile_id: "local-personal".to_string(),
                source_root: source_root.clone(),
            })],
            environments: vec![EnvironmentConfig::local("local", agents)],
        };
        write_workspace_config(&config_path(&config_home), &config)?;

        Ok(json!({
            "configHome": config_home,
            "sourceRoot": source_root,
            "repository": repository,
            "config": config,
        }))
    })
}

#[tauri::command]
pub fn set_local_source(
    config_home: Option<String>,
    source_profile_id: Option<String>,
    source_root: String,
) -> CommandResult {
    handle(|| {
        let config_home = config_home_path(config_home);
        init_config_home(&config_home)?;
        let path = config_path(&config_home);
        let mut config = read_workspace_config(&path)?;
        let source_profile_id = source_profile_id.unwrap_or_else(|| "local-personal".to_string());
        let source_root = expand_path_from_cwd(Path::new(&source_root))?;
        let repository = init_or_update_repository_metadata(&source_root, None)?;

        upsert_source_profile(
            &mut config,
            SourceProfile::Local(LocalSourceProfile {
                source_profile_id: source_profile_id.clone(),
                source_root: source_root.clone(),
            }),
        );
        config.active_source_profile_id = source_profile_id;
        write_workspace_config(&path, &config)?;

        Ok(json!({
            "sourceRoot": source_root,
            "repository": repository,
            "config": config,
        }))
    })
}

#[tauri::command]
pub fn set_agent_dir(
    config_home: Option<String>,
    environment_id: Option<String>,
    agent_id: String,
    skills_dir: String,
    managed: bool,
) -> CommandResult {
    handle(|| {
        let config_home = config_home_path(config_home);
        let path = config_path(&config_home);
        let mut config = read_workspace_config(&path)?;
        let agent_id = parse_agent(&agent_id)?;
        let environment_id = environment_id.unwrap_or_else(|| "local".to_string());
        let environment = ensure_environment(&mut config, &environment_id);
        let skills_dir = if environment.kind == EnvironmentKind::Local {
            expand_path_from_cwd(Path::new(&skills_dir))?
        } else {
            PathBuf::from(skills_dir)
        };

        if let Some(agent) = environment
            .agents
            .iter_mut()
            .find(|agent| agent.agent_id == agent_id)
        {
            agent.skills_dir = skills_dir;
            agent.managed = managed;
        } else {
            environment.agents.push(AgentConfig {
                agent_id,
                managed,
                skills_dir,
                enabled_skill_ids: Vec::new(),
            });
        }
        write_workspace_config(&path, &config)?;
        Ok(json!({ "config": config }))
    })
}

#[tauri::command]
pub fn set_skill_enabled(
    config_home: Option<String>,
    environment_id: Option<String>,
    agent_id: String,
    skill_id: String,
    enabled: bool,
) -> CommandResult {
    handle(|| {
        let config_home = config_home_path(config_home);
        let path = config_path(&config_home);
        let mut config = read_workspace_config(&path)?;
        let agent_id = parse_agent(&agent_id)?;
        let environment_id = environment_id.unwrap_or_else(|| "local".to_string());
        let env_config = ensure_environment(&mut config, &environment_id);
        let agent = env_config
            .agents
            .iter_mut()
            .find(|agent| agent.agent_id == agent_id)
            .ok_or_else(|| skills_manager_core::Error::AgentNotConfigured(agent_id.to_string()))?;

        if enabled {
            if !agent.enabled_skill_ids.iter().any(|id| id == &skill_id) {
                agent.enabled_skill_ids.push(skill_id);
                agent.enabled_skill_ids.sort();
            }
        } else {
            agent.enabled_skill_ids.retain(|id| id != &skill_id);
        }

        write_workspace_config(&path, &config)?;
        Ok(json!({ "config": config }))
    })
}

#[tauri::command]
pub fn reconcile(
    config_home: Option<String>,
    agent_id: Option<String>,
    plan: bool,
) -> CommandResult {
    handle(|| {
        let config_home = config_home_path(config_home);
        init_config_home(&config_home)?;
        let config = read_workspace_config(&config_path(&config_home))?;
        let source_root = resolve_active_source_root(&config)?;
        let env_config = environment(&config, Some("local"))?;
        let skills = scan_source(&source_root)?;
        let mut state = read_workspace_state(&state_path(&config_home))?;
        let mode = if plan {
            ReconcileMode::Plan
        } else {
            ReconcileMode::Apply
        };
        let filter = agent_id.as_deref().map(parse_agent).transpose()?;
        let mut reports = Vec::new();

        for agent in &env_config.agents {
            if filter.is_some() && filter != Some(agent.agent_id) {
                continue;
            }
            reports.push(reconcile_agent_with_state(
                env_config,
                agent.agent_id,
                &skills,
                mode,
                &mut state,
            )?);
        }

        if !plan {
            write_workspace_state(&state_path(&config_home), &state)?;
        }

        Ok(json!({
            "sourceRoot": source_root,
            "reports": reports,
            "state": state,
        }))
    })
}

#[tauri::command]
pub fn opencode_ensure_path(
    config_home: Option<String>,
    config_path_override: Option<String>,
) -> CommandResult {
    handle(|| {
        let config_home = config_home_path(config_home);
        let config = read_workspace_config(&config_path(&config_home))?;
        let source_root = resolve_active_source_root(&config)?;
        let report = ensure_opencode_skill_path(
            config_path_override
                .map(|path| expand_path_from_cwd(Path::new(&path)))
                .transpose()?,
            &source_root.join("skills"),
            "desktop",
        )?;
        Ok(json!({ "opencode": report }))
    })
}

#[tauri::command]
pub fn set_remote_source(
    config_home: Option<String>,
    source_profile_id: Option<String>,
    host: String,
    user: String,
    remote_source_root: String,
    local_cache_root: String,
    auto_sync: bool,
    delete_extraneous: bool,
) -> CommandResult {
    handle(|| {
        let config_home = config_home_path(config_home);
        let path = config_path(&config_home);
        let mut config = read_workspace_config(&path)?;
        let source_profile_id = source_profile_id.unwrap_or_else(|| "remote-personal".to_string());
        let local_cache_root = expand_path_from_cwd(Path::new(&local_cache_root))?;

        upsert_source_profile(
            &mut config,
            SourceProfile::Remote(RemoteSourceProfile {
                source_profile_id: source_profile_id.clone(),
                host,
                user,
                remote_source_root: PathBuf::from(remote_source_root),
                local_cache_root,
                auto_sync,
                delete_extraneous,
            }),
        );
        config.active_source_profile_id = source_profile_id;
        write_workspace_config(&path, &config)?;

        Ok(json!({ "config": config }))
    })
}

#[tauri::command]
pub fn set_remote_environment(
    config_home: Option<String>,
    environment_id: String,
    host: String,
    user: String,
    remote_cache_root: String,
    direction: String,
    auto_sync: bool,
    delete_extraneous: bool,
    codex_skills_dir: Option<String>,
    claude_code_skills_dir: Option<String>,
    opencode_skills_dir: Option<String>,
) -> CommandResult {
    handle(|| {
        let config_home = config_home_path(config_home);
        let path = config_path(&config_home);
        let mut config = read_workspace_config(&path)?;
        let direction = parse_sync_direction(&direction)?;
        let source_root = resolve_active_source_root(&config)?;
        let enabled_skill_ids: Vec<String> = scan_source(&source_root)
            .map(|skills| skills.into_iter().map(|skill| skill.skill_id).collect())
            .unwrap_or_default();
        let mut agents = Vec::new();

        for agent_id in BUILTIN_AGENTS {
            let selected = match agent_id {
                AgentId::Codex => codex_skills_dir.clone(),
                AgentId::ClaudeCode => claude_code_skills_dir.clone(),
                AgentId::OpenCode => opencode_skills_dir.clone(),
            };
            if let Some(skills_dir) = selected.filter(|value| !value.trim().is_empty()) {
                agents.push(AgentConfig {
                    agent_id,
                    managed: true,
                    skills_dir: PathBuf::from(skills_dir),
                    enabled_skill_ids: enabled_skill_ids.clone(),
                });
            }
        }

        config
            .environments
            .retain(|existing| existing.environment_id != environment_id);
        config.environments.push(EnvironmentConfig {
            environment_id: environment_id.clone(),
            kind: EnvironmentKind::Remote,
            host: Some(host),
            user: Some(user),
            sync_direction: Some(direction),
            remote_cache_root: Some(PathBuf::from(remote_cache_root)),
            auto_sync,
            delete_extraneous,
            agents,
        });
        write_workspace_config(&path, &config)?;

        Ok(json!({ "config": config }))
    })
}

#[tauri::command]
pub fn remote_sync(
    config_home: Option<String>,
    environment_id: Option<String>,
    direction: String,
    plan: bool,
    repo_id: Option<String>,
) -> CommandResult {
    handle(|| {
        let config_home = config_home_path(config_home);
        let config = read_workspace_config(&config_path(&config_home))?;
        let source = active_source_profile(&config)?;
        let direction = parse_sync_direction(&direction)?;
        let marker = CacheMarker::new(
            repo_id.unwrap_or_else(|| config.active_source_profile_id.clone()),
            config.active_source_profile_id.clone(),
        );

        let plan_value = match direction {
            SyncDirection::PushLocalToRemote => {
                let SourceProfile::Local(local) = source else {
                    return Err(skills_manager_core::Error::InvalidInput(
                        "push-local-to-remote requires a local active source".to_string(),
                    ));
                };
                let env_config = environment(&config, environment_id.as_deref())?;
                let host = required_env_host(env_config)?;
                let user = required_env_user(env_config)?;
                let remote_cache_root = env_config.remote_cache_root.clone().ok_or_else(|| {
                    skills_manager_core::Error::InvalidInput(
                        "remoteCacheRoot is required for push-local-to-remote".to_string(),
                    )
                })?;
                plan_push_local_to_remote(&PushLocalToRemote {
                    host,
                    user,
                    local_source_root: expand_path_from_cwd(&local.source_root)?,
                    remote_cache_root,
                    marker,
                    delete_extraneous: env_config.delete_extraneous,
                })
            }
            SyncDirection::PullRemoteToLocal => {
                let SourceProfile::Remote(remote) = source else {
                    return Err(skills_manager_core::Error::InvalidInput(
                        "pull-remote-to-local requires a remote active source".to_string(),
                    ));
                };
                plan_pull_remote_to_local(&PullRemoteToLocal {
                    host: remote.host.clone(),
                    user: remote.user.clone(),
                    remote_source_root: remote.remote_source_root.clone(),
                    local_cache_root: expand_path_from_cwd(&remote.local_cache_root)?,
                    marker,
                    delete_extraneous: remote.delete_extraneous,
                })?
            }
        };

        if plan {
            let remote_link_plan = if direction == SyncDirection::PushLocalToRemote {
                let env_config = environment(&config, environment_id.as_deref())?;
                env_config
                    .remote_cache_root
                    .as_ref()
                    .map(|remote_cache_root| {
                        plan_remote_links(
                            env_config,
                            remote_cache_root,
                            &scan_source(&resolve_active_source_root(&config)?)?,
                        )
                    })
                    .transpose()?
            } else {
                None
            };
            return Ok(json!({
                "plan": plan_value,
                "remoteLinkPlan": remote_link_plan,
            }));
        }

        let report = execute_sync_plan(&plan_value)?;
        let remote_link = if direction == SyncDirection::PushLocalToRemote {
            let env_config = environment(&config, environment_id.as_deref())?;
            let remote_cache_root = env_config.remote_cache_root.clone().ok_or_else(|| {
                skills_manager_core::Error::InvalidInput(
                    "remoteCacheRoot is required for push-local-to-remote".to_string(),
                )
            })?;
            let source_root = resolve_active_source_root(&config)?;
            let skills = scan_source(&source_root)?;
            let link_plan = plan_remote_links(env_config, &remote_cache_root, &skills)?;
            Some(execute_remote_link_plan(&link_plan)?)
        } else {
            None
        };
        Ok(json!({
            "sync": report,
            "remoteLink": remote_link,
        }))
    })
}

#[tauri::command]
pub fn remote_cli_status(config_home: Option<String>, environment_id: String) -> CommandResult {
    handle(|| {
        let config_home = config_home_path(config_home);
        let config = read_workspace_config(&config_path(&config_home))?;
        let env_config = environment(&config, Some(&environment_id))?;
        let host = required_env_host(env_config)?;
        let user = required_env_user(env_config)?;
        let remote = format!("{user}@{host}");
        let output = Command::new("ssh")
            .arg(&remote)
            .arg("skills-manager version --json")
            .output()
            .map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    skills_manager_core::Error::CommandUnavailable("ssh".to_string())
                } else {
                    skills_manager_core::Error::io("failed to execute ssh", err)
                }
            })?;

        if !output.status.success() {
            return Ok(json!({
                "remoteStatus": {
                    "environmentId": env_config.environment_id,
                    "remote": remote,
                    "available": false,
                    "status": output.status.code(),
                    "stderr": String::from_utf8_lossy(&output.stderr).trim(),
                }
            }));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed = serde_json::from_str::<Value>(&stdout).unwrap_or_else(|_| {
            json!({
                "raw": stdout.trim(),
            })
        });
        Ok(json!({
            "remoteStatus": {
                "environmentId": env_config.environment_id,
                "remote": remote,
                "available": true,
                "version": parsed,
            }
        }))
    })
}

fn config_home_path(config_home: Option<String>) -> PathBuf {
    config_home
        .map(PathBuf::from)
        .map(|path| expand_path_from_cwd(&path).unwrap_or(path))
        .unwrap_or_else(default_config_home)
}

fn resolve_active_source_root(config: &WorkspaceConfig) -> skills_manager_core::Result<PathBuf> {
    match active_source_profile(config)? {
        SourceProfile::Local(profile) => expand_path_from_cwd(&profile.source_root),
        SourceProfile::Remote(profile) => expand_path_from_cwd(&profile.local_cache_root),
    }
}

fn scan_source_profiles(
    config: &WorkspaceConfig,
) -> skills_manager_core::Result<Vec<SourceSkills>> {
    config
        .source_profiles
        .iter()
        .map(|profile| {
            let (source_profile_id, kind, source_root) = match profile {
                SourceProfile::Local(profile) => (
                    profile.source_profile_id.clone(),
                    "local",
                    expand_path_from_cwd(&profile.source_root)?,
                ),
                SourceProfile::Remote(profile) => (
                    profile.source_profile_id.clone(),
                    "remote",
                    expand_path_from_cwd(&profile.local_cache_root)?,
                ),
            };
            let (skills, error) = match scan_source(&source_root) {
                Ok(skills) => (skills, None),
                Err(err) => (Vec::new(), Some(err.to_string())),
            };

            Ok(SourceSkills {
                source_profile_id,
                kind,
                source_root,
                skills,
                error,
            })
        })
        .collect()
}

fn ensure_environment<'a>(
    config: &'a mut WorkspaceConfig,
    environment_id: &str,
) -> &'a mut EnvironmentConfig {
    if let Some(index) = config
        .environments
        .iter()
        .position(|environment| environment.environment_id == environment_id)
    {
        return &mut config.environments[index];
    }
    config
        .environments
        .push(EnvironmentConfig::local(environment_id, Vec::new()));
    config.environments.last_mut().unwrap()
}

fn upsert_source_profile(config: &mut WorkspaceConfig, profile: SourceProfile) {
    let source_profile_id = profile.source_profile_id().to_string();
    config
        .source_profiles
        .retain(|existing| existing.source_profile_id() != source_profile_id);
    config.source_profiles.push(profile);
}

fn parse_agent(value: &str) -> skills_manager_core::Result<AgentId> {
    value
        .parse::<AgentId>()
        .map_err(skills_manager_core::Error::InvalidInput)
}

fn parse_sync_direction(value: &str) -> skills_manager_core::Result<SyncDirection> {
    match value {
        "push-local-to-remote" => Ok(SyncDirection::PushLocalToRemote),
        "pull-remote-to-local" => Ok(SyncDirection::PullRemoteToLocal),
        _ => Err(skills_manager_core::Error::InvalidInput(format!(
            "unknown sync direction: {value}"
        ))),
    }
}

fn required_env_host(env_config: &EnvironmentConfig) -> skills_manager_core::Result<String> {
    env_config.host.clone().ok_or_else(|| {
        skills_manager_core::Error::InvalidInput(format!(
            "host is required for environment {}",
            env_config.environment_id
        ))
    })
}

fn required_env_user(env_config: &EnvironmentConfig) -> skills_manager_core::Result<String> {
    env_config.user.clone().ok_or_else(|| {
        skills_manager_core::Error::InvalidInput(format!(
            "user is required for environment {}",
            env_config.environment_id
        ))
    })
}

fn error_code(err: &skills_manager_core::Error) -> &'static str {
    match err {
        skills_manager_core::Error::Io { .. } => "IO_ERROR",
        skills_manager_core::Error::InvalidInput(_) => "CONFIG_INVALID",
        skills_manager_core::Error::SourceNotFound(_) => "SOURCE_NOT_FOUND",
        skills_manager_core::Error::SourceInvalidLayout(_) => "SOURCE_INVALID_LAYOUT",
        skills_manager_core::Error::AgentNotConfigured(_) => "AGENT_NOT_DETECTED",
        skills_manager_core::Error::AgentSkillsDirInvalid(_) => "AGENT_SKILLS_DIR_INVALID",
        skills_manager_core::Error::CacheMarkerMissing(_) => "CACHE_MARKER_MISSING",
        skills_manager_core::Error::CacheMarkerMismatch(_) => "CACHE_MARKER_MISMATCH",
        skills_manager_core::Error::CommandUnavailable(_) => "COMMAND_UNAVAILABLE",
        skills_manager_core::Error::CommandFailed { .. } => "COMMAND_FAILED",
        skills_manager_core::Error::Json { .. } => "CONFIG_INVALID",
    }
}
