use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde_json::{json, Value};
use skills_manager_core::agents::AgentId;
use skills_manager_core::config::{
    active_source_profile, config_path, environment, read_workspace_config,
};
use skills_manager_core::model::{
    AgentConfig, EnvironmentConfig, SourceProfile, SyncDirection, WorkspaceConfig,
};
use skills_manager_core::paths::expand_path_from_cwd;

use crate::args::CliArgs;

pub fn config_context(
    args: &CliArgs,
) -> skills_manager_core::Result<(WorkspaceConfig, PathBuf, EnvironmentConfig)> {
    let config_home = args.config_home();
    let config = read_workspace_config(&config_path(&config_home))?;
    let source_root = resolve_active_source_root(&config)?;
    let env_config = environment(&config, args.option("environment").as_deref())?.clone();
    Ok((config, source_root, env_config))
}

pub fn resolve_active_source_root(
    config: &WorkspaceConfig,
) -> skills_manager_core::Result<PathBuf> {
    match active_source_profile(config)? {
        SourceProfile::Local(profile) => expand_path_from_cwd(&profile.source_root),
        SourceProfile::Remote(profile) => expand_path_from_cwd(&profile.local_cache_root),
    }
}

pub fn active_source_root(args: &CliArgs) -> skills_manager_core::Result<PathBuf> {
    let config = read_workspace_config(&config_path(&args.config_home()))?;
    resolve_active_source_root(&config)
}

pub fn read_or_empty_config(path: &Path) -> WorkspaceConfig {
    if path.exists() {
        read_workspace_config(path).unwrap_or_else(|_| empty_config())
    } else {
        empty_config()
    }
}

pub fn empty_config() -> WorkspaceConfig {
    WorkspaceConfig {
        schema_version: 1,
        active_source_profile_id: "local-personal".to_string(),
        source_profiles: Vec::new(),
        environments: vec![EnvironmentConfig::local("local", Vec::new())],
    }
}

pub fn upsert_source_profile(config: &mut WorkspaceConfig, profile: SourceProfile) {
    let source_profile_id = profile.source_profile_id().to_string();
    config
        .source_profiles
        .retain(|existing| existing.source_profile_id() != source_profile_id);
    config.source_profiles.push(profile);
}

pub fn ensure_environment<'a>(
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

pub fn upsert_agent(environment: &mut EnvironmentConfig, agent: AgentConfig) {
    environment
        .agents
        .retain(|existing| existing.agent_id != agent.agent_id);
    environment.agents.push(agent);
}

pub fn reconcile_action_json(action: &skills_manager_core::reconcile::ReconcileAction) -> Value {
    json!({
        "type": action.kind.as_str(),
        "status": action.status.as_str(),
        "environmentId": action.environment_id,
        "agentId": action.agent_id,
        "skillId": action.skill_id,
        "sourcePath": action.source_path,
        "targetPath": action.target_path,
        "message": action.message,
    })
}

pub fn optional_agent(args: &CliArgs) -> skills_manager_core::Result<Option<AgentId>> {
    args.option("agent")
        .map(|value| parse_agent_id(&value))
        .transpose()
}

pub fn required_agent(args: &CliArgs, usage: &str) -> skills_manager_core::Result<AgentId> {
    args.option("agent")
        .ok_or_else(|| skills_manager_core::Error::InvalidInput(format!("usage: {usage}")))
        .and_then(|value| parse_agent_id(&value))
}

pub fn parse_agent_id(value: &str) -> skills_manager_core::Result<AgentId> {
    AgentId::from_str(value).map_err(skills_manager_core::Error::InvalidInput)
}

pub fn parse_sync_direction(value: &str) -> skills_manager_core::Result<SyncDirection> {
    match value {
        "push-local-to-remote" => Ok(SyncDirection::PushLocalToRemote),
        "pull-remote-to-local" => Ok(SyncDirection::PullRemoteToLocal),
        _ => Err(skills_manager_core::Error::InvalidInput(format!(
            "unknown sync direction: {value}"
        ))),
    }
}

pub fn required_env_host(env_config: &EnvironmentConfig) -> skills_manager_core::Result<String> {
    env_config.host.clone().ok_or_else(|| {
        skills_manager_core::Error::InvalidInput(format!(
            "host is required for environment {}",
            env_config.environment_id
        ))
    })
}

pub fn required_env_user(env_config: &EnvironmentConfig) -> skills_manager_core::Result<String> {
    env_config.user.clone().ok_or_else(|| {
        skills_manager_core::Error::InvalidInput(format!(
            "user is required for environment {}",
            env_config.environment_id
        ))
    })
}

pub fn required_path_option(
    args: &CliArgs,
    name: &str,
    usage: &str,
) -> skills_manager_core::Result<PathBuf> {
    args.option(name)
        .map(PathBuf::from)
        .ok_or_else(|| skills_manager_core::Error::InvalidInput(format!("usage: {usage}")))
}

pub fn required_option(
    args: &CliArgs,
    name: &str,
    usage: &str,
) -> skills_manager_core::Result<String> {
    args.option(name)
        .ok_or_else(|| skills_manager_core::Error::InvalidInput(format!("usage: {usage}")))
}

pub fn required_path_value(
    args: &CliArgs,
    index: usize,
    name: &str,
    usage: &str,
) -> skills_manager_core::Result<String> {
    args.option(name)
        .or_else(|| args.command.get(index).cloned())
        .ok_or_else(|| skills_manager_core::Error::InvalidInput(format!("usage: {usage}")))
}

pub fn required_positional<'a>(
    args: &'a CliArgs,
    index: usize,
    usage: &str,
) -> skills_manager_core::Result<&'a str> {
    args.command
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| skills_manager_core::Error::InvalidInput(format!("usage: {usage}")))
}
