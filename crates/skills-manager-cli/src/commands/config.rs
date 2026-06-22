use std::path::PathBuf;

use serde_json::{json, Value};
use skills_manager_core::agents::BUILTIN_AGENTS;
use skills_manager_core::config::{
    config_path, init_config_home, read_workspace_config, read_workspace_state, state_path,
    write_workspace_config,
};
use skills_manager_core::model::{
    AgentConfig, EnvironmentConfig, EnvironmentKind, LocalSourceProfile, RemoteSourceProfile,
    SourceProfile, SyncDirection,
};
use skills_manager_core::paths::expand_path_from_cwd;
use skills_manager_core::scan::scan_source;

use crate::args::CliArgs;
use crate::output::RunContext;
use crate::util::{
    ensure_environment, parse_sync_direction, read_or_empty_config, required_agent,
    required_option, required_path_option, resolve_active_source_root, upsert_agent,
    upsert_source_profile,
};

pub fn show(args: &CliArgs) -> skills_manager_core::Result<Value> {
    let config_home = args.config_home();
    let config = read_workspace_config(&config_path(&config_home))?;
    let state = read_workspace_state(&state_path(&config_home))?;
    Ok(json!({
        "configHome": config_home,
        "config": config,
        "state": state,
    }))
}

pub fn set_local_source(
    args: &CliArgs,
    run: &mut RunContext,
) -> skills_manager_core::Result<Value> {
    let config_home = args.config_home();
    init_config_home(&config_home)?;
    let path = config_path(&config_home);
    let mut config = read_or_empty_config(&path);
    let source_profile_id = args
        .option("source-profile-id")
        .unwrap_or_else(|| "local-personal".to_string());
    let source_root = required_path_option(
        args,
        "source-root",
        "config set-local-source --source-root <path>",
    )?;
    let source_root = expand_path_from_cwd(&source_root)?;

    upsert_source_profile(
        &mut config,
        SourceProfile::Local(LocalSourceProfile {
            source_profile_id: source_profile_id.clone(),
            source_root: source_root.clone(),
        }),
    );
    config.active_source_profile_id = source_profile_id;
    write_workspace_config(&path, &config)?;
    run.add_action(json!({
        "type": "set-local-source",
        "status": "applied",
        "path": path,
    }));
    Ok(json!({ "configPath": path, "config": config }))
}

pub fn set_remote_source(
    args: &CliArgs,
    run: &mut RunContext,
) -> skills_manager_core::Result<Value> {
    let config_home = args.config_home();
    init_config_home(&config_home)?;
    let path = config_path(&config_home);
    let mut config = read_or_empty_config(&path);
    let source_profile_id = args
        .option("source-profile-id")
        .unwrap_or_else(|| "remote-personal".to_string());
    let host = required_option(args, "host", "config set-remote-source --host <host>")?;
    let user = required_option(args, "user", "config set-remote-source --user <user>")?;
    let remote_source_root = required_path_option(
        args,
        "remote-source-root",
        "config set-remote-source --remote-source-root <path>",
    )?;
    let local_cache_root = required_path_option(
        args,
        "local-cache-root",
        "config set-remote-source --local-cache-root <path>",
    )?;
    let local_cache_root = expand_path_from_cwd(&local_cache_root)?;

    upsert_source_profile(
        &mut config,
        SourceProfile::Remote(RemoteSourceProfile {
            source_profile_id: source_profile_id.clone(),
            host,
            user,
            remote_source_root,
            local_cache_root,
            auto_sync: args.flag("auto-sync"),
            delete_extraneous: !args.flag("no-delete"),
        }),
    );
    config.active_source_profile_id = source_profile_id;
    write_workspace_config(&path, &config)?;
    run.add_action(json!({
        "type": "set-remote-source",
        "status": "applied",
        "path": path,
    }));
    Ok(json!({ "configPath": path, "config": config }))
}

pub fn set_agent(args: &CliArgs, run: &mut RunContext) -> skills_manager_core::Result<Value> {
    let config_home = args.config_home();
    let path = config_path(&config_home);
    let mut config = read_workspace_config(&path)?;
    let environment_id = args
        .option("environment")
        .unwrap_or_else(|| "local".to_string());
    let agent_id = required_agent(args, "config set-agent --agent <agent> --skills-dir <path>")?;
    let skills_dir = required_path_option(
        args,
        "skills-dir",
        "config set-agent --agent <agent> --skills-dir <path>",
    )?;
    let skills_dir = expand_path_from_cwd(&skills_dir)?;
    let managed = !args.flag("unmanaged");
    let source_root = resolve_active_source_root(&config)?;
    let skill_ids = scan_source(&source_root)
        .map(|skills| skills.into_iter().map(|skill| skill.skill_id).collect())
        .unwrap_or_default();

    let environment = ensure_environment(&mut config, &environment_id);
    upsert_agent(
        environment,
        AgentConfig {
            agent_id,
            managed,
            skills_dir,
            enabled_skill_ids: skill_ids,
        },
    );

    write_workspace_config(&path, &config)?;
    run.add_action(json!({
        "type": "set-agent",
        "status": "applied",
        "environmentId": environment_id,
        "agentId": agent_id,
    }));
    Ok(json!({ "configPath": path, "config": config }))
}

pub fn set_skill_enabled(
    args: &CliArgs,
    run: &mut RunContext,
    enabled: bool,
) -> skills_manager_core::Result<Value> {
    let config_home = args.config_home();
    let path = config_path(&config_home);
    let mut config = read_workspace_config(&path)?;
    let environment_id = args
        .option("environment")
        .unwrap_or_else(|| "local".to_string());
    let agent_id = required_agent(
        args,
        "config enable|disable --agent <agent> --skill <skillId>",
    )?;
    let skill_id = required_option(args, "skill", "config enable|disable --skill <skillId>")?;
    let environment = config
        .environments
        .iter_mut()
        .find(|environment| environment.environment_id == environment_id)
        .ok_or_else(|| {
            skills_manager_core::Error::InvalidInput(format!(
                "environment is not configured: {environment_id}"
            ))
        })?;
    let agent = environment
        .agents
        .iter_mut()
        .find(|agent| agent.agent_id == agent_id)
        .ok_or_else(|| {
            skills_manager_core::Error::InvalidInput(format!("agent is not configured: {agent_id}"))
        })?;

    if enabled {
        if !agent.enabled_skill_ids.iter().any(|id| id == &skill_id) {
            agent.enabled_skill_ids.push(skill_id.clone());
            agent.enabled_skill_ids.sort();
        }
    } else {
        agent.enabled_skill_ids.retain(|id| id != &skill_id);
    }

    write_workspace_config(&path, &config)?;
    run.add_action(json!({
        "type": if enabled { "enable-skill" } else { "disable-skill" },
        "status": "applied",
        "environmentId": environment_id,
        "agentId": agent_id,
        "skillId": skill_id,
    }));
    Ok(json!({ "configPath": path, "config": config }))
}

pub fn add_remote_env(args: &CliArgs, run: &mut RunContext) -> skills_manager_core::Result<Value> {
    let config_home = args.config_home();
    let path = config_path(&config_home);
    let mut config = read_workspace_config(&path)?;
    let environment_id = required_option(
        args,
        "environment",
        "config add-remote-env --environment <id> --host <host> --user <user>",
    )?;
    let host = required_option(args, "host", "config add-remote-env --host <host>")?;
    let user = required_option(args, "user", "config add-remote-env --user <user>")?;
    let direction = args
        .option("direction")
        .map(|value| parse_sync_direction(&value))
        .transpose()?
        .unwrap_or(SyncDirection::PushLocalToRemote);
    let remote_cache_root = args.option("remote-cache-root").map(PathBuf::from);

    let mut environment = EnvironmentConfig {
        environment_id: environment_id.clone(),
        kind: EnvironmentKind::Remote,
        host: Some(host),
        user: Some(user),
        sync_direction: Some(direction),
        remote_cache_root,
        auto_sync: args.flag("auto-sync"),
        delete_extraneous: !args.flag("no-delete"),
        agents: Vec::new(),
    };

    for agent_id in BUILTIN_AGENTS {
        if let Some(skills_dir) = args.option(&format!("{}-skills-dir", agent_id.as_str())) {
            environment.agents.push(AgentConfig {
                agent_id,
                managed: true,
                skills_dir: PathBuf::from(skills_dir),
                enabled_skill_ids: Vec::new(),
            });
        }
    }

    config
        .environments
        .retain(|existing| existing.environment_id != environment_id);
    config.environments.push(environment);
    write_workspace_config(&path, &config)?;
    run.add_action(json!({
        "type": "add-remote-env",
        "status": "applied",
        "environmentId": environment_id,
    }));
    Ok(json!({ "configPath": path, "config": config }))
}
