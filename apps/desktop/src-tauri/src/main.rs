use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{json, Value};
use skills_manager_core::agents::{AgentId, BUILTIN_AGENTS};
use skills_manager_core::config::{
    active_source_profile, config_path, environment, init_config_home, read_workspace_config,
    read_workspace_state, state_path, write_workspace_config, write_workspace_state,
};
use skills_manager_core::detect::detect_machine;
use skills_manager_core::hook::hook_status;
use skills_manager_core::model::{
    AgentConfig, EnvironmentConfig, LocalSourceProfile, SourceProfile, WorkspaceConfig,
};
use skills_manager_core::opencode::ensure_opencode_skill_path;
use skills_manager_core::paths::{default_config_home, expand_path_from_cwd};
use skills_manager_core::reconcile::{reconcile_agent_with_state, ReconcileMode};
use skills_manager_core::repository::init_or_update_repository_metadata;
use skills_manager_core::scan::scan_source;
use skills_manager_core::status::compute_agent_statuses;

#[tauri::command]
fn load_dashboard(config_home: Option<String>) -> CommandResult {
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
        let mut statuses = Vec::new();
        if let Some(config) = &config {
            if let Ok(env_config) = environment(config, Some("local")) {
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
            "statuses": statuses,
            "hooks": hooks,
        }))
    })
}

#[tauri::command]
fn init_config(
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
                .map(PathBuf::from)
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
fn set_agent_dir(
    config_home: Option<String>,
    agent_id: String,
    skills_dir: String,
    managed: bool,
) -> CommandResult {
    handle(|| {
        let config_home = config_home_path(config_home);
        let path = config_path(&config_home);
        let mut config = read_workspace_config(&path)?;
        let agent_id = parse_agent(&agent_id)?;
        let environment = ensure_environment(&mut config, "local");
        if let Some(agent) = environment
            .agents
            .iter_mut()
            .find(|agent| agent.agent_id == agent_id)
        {
            agent.skills_dir = PathBuf::from(skills_dir);
            agent.managed = managed;
        } else {
            environment.agents.push(AgentConfig {
                agent_id,
                managed,
                skills_dir: PathBuf::from(skills_dir),
                enabled_skill_ids: Vec::new(),
            });
        }
        write_workspace_config(&path, &config)?;
        Ok(json!({ "config": config }))
    })
}

#[tauri::command]
fn set_skill_enabled(
    config_home: Option<String>,
    agent_id: String,
    skill_id: String,
    enabled: bool,
) -> CommandResult {
    handle(|| {
        let config_home = config_home_path(config_home);
        let path = config_path(&config_home);
        let mut config = read_workspace_config(&path)?;
        let agent_id = parse_agent(&agent_id)?;
        let env_config = ensure_environment(&mut config, "local");
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
fn reconcile(config_home: Option<String>, agent_id: Option<String>, plan: bool) -> CommandResult {
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
fn opencode_ensure_path(config_home: Option<String>, config_path_override: Option<String>) -> CommandResult {
    handle(|| {
        let config_home = config_home_path(config_home);
        let config = read_workspace_config(&config_path(&config_home))?;
        let source_root = resolve_active_source_root(&config)?;
        let report = ensure_opencode_skill_path(
            config_path_override.map(PathBuf::from),
            &source_root.join("skills"),
            "desktop",
        )?;
        Ok(json!({ "opencode": report }))
    })
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_dashboard,
            init_config,
            set_agent_dir,
            set_skill_enabled,
            reconcile,
            opencode_ensure_path
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Skills Manager desktop");
}

type CommandResult = Result<Value, CommandError>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandError {
    code: String,
    message: String,
}

fn handle<F>(f: F) -> CommandResult
where
    F: FnOnce() -> skills_manager_core::Result<Value>,
{
    f().map_err(|error| CommandError {
        code: error_code(&error).to_string(),
        message: error.to_string(),
    })
}

fn config_home_path(config_home: Option<String>) -> PathBuf {
    config_home.map(PathBuf::from).unwrap_or_else(default_config_home)
}

fn resolve_active_source_root(config: &WorkspaceConfig) -> skills_manager_core::Result<PathBuf> {
    match active_source_profile(config)? {
        SourceProfile::Local(profile) => expand_path_from_cwd(&profile.source_root),
        SourceProfile::Remote(profile) => expand_path_from_cwd(&profile.local_cache_root),
    }
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

fn parse_agent(value: &str) -> skills_manager_core::Result<AgentId> {
    value
        .parse::<AgentId>()
        .map_err(skills_manager_core::Error::InvalidInput)
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
