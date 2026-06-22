use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use skills_manager_core::agents::BUILTIN_AGENTS;
use skills_manager_core::cache::{init_cache_marker, verify_cache_marker, CacheMarker};
use skills_manager_core::config::{
    config_path, init_config_home, read_workspace_config, read_workspace_state, state_path,
    write_workspace_config, write_workspace_state,
};
use skills_manager_core::detect::detect_machine;
use skills_manager_core::hook::{hook_status as core_hook_status, install_hook};
use skills_manager_core::model::{
    AgentConfig, EnvironmentConfig, LocalSourceProfile, SourceProfile, WorkspaceConfig,
};
use skills_manager_core::opencode::{ensure_opencode_skill_path, OpenCodePathStatus};
use skills_manager_core::paths::expand_path_from_cwd;
use skills_manager_core::reconcile::{reconcile_agent, reconcile_agent_with_state, ReconcileMode};
use skills_manager_core::repository::init_or_update_repository_metadata;
use skills_manager_core::scan::scan_source;
use skills_manager_core::status::compute_agent_statuses;

use crate::args::CliArgs;
use crate::output::RunContext;
use crate::util::{
    active_source_root, config_context, optional_agent, parse_agent_id, reconcile_action_json,
    required_agent, required_path_option, required_path_value, required_positional,
    resolve_active_source_root,
};

pub fn version() -> skills_manager_core::Result<Value> {
    Ok(json!({
        "name": "skills-manager",
        "version": env!("CARGO_PKG_VERSION"),
        "schemaVersion": 1,
    }))
}

pub fn detect(args: &CliArgs) -> skills_manager_core::Result<Value> {
    Ok(json!(detect_machine(args.config_home())))
}

pub fn init_config(args: &CliArgs, run: &mut RunContext) -> skills_manager_core::Result<Value> {
    let config_home = args.config_home();
    init_config_home(&config_home)?;

    let source_root =
        required_path_option(args, "source-root", "init-config --source-root <path>")?;
    let source_root = expand_path_from_cwd(&source_root)?;
    let repository =
        init_or_update_repository_metadata(&source_root, args.option("repo-id").as_deref())?;
    let skills = scan_source(&source_root)?;
    let enabled_skill_ids = skills
        .iter()
        .map(|skill| skill.skill_id.clone())
        .collect::<Vec<_>>();

    let agents = BUILTIN_AGENTS
        .into_iter()
        .filter_map(|agent_id| {
            if let Some(skills_dir) = args.option(&format!("{}-skills-dir", agent_id.as_str())) {
                return Some(AgentConfig {
                    agent_id,
                    managed: true,
                    skills_dir: PathBuf::from(skills_dir),
                    enabled_skill_ids: enabled_skill_ids.clone(),
                });
            }
            skills_manager_core::detect::recommended_skills_dir(agent_id).map(|skills_dir| {
                AgentConfig {
                    agent_id,
                    managed: true,
                    skills_dir,
                    enabled_skill_ids: enabled_skill_ids.clone(),
                }
            })
        })
        .collect::<Vec<_>>();

    let source_profile_id = args
        .option("source-profile-id")
        .unwrap_or_else(|| "local-personal".to_string());
    let config = WorkspaceConfig {
        schema_version: 1,
        active_source_profile_id: source_profile_id.clone(),
        source_profiles: vec![SourceProfile::Local(LocalSourceProfile {
            source_profile_id,
            source_root: source_root.clone(),
        })],
        environments: vec![EnvironmentConfig::local("local", agents)],
    };

    let path = config_path(&config_home);
    write_workspace_config(&path, &config)?;
    run.add_action(json!({
        "type": "write-config",
        "status": "applied",
        "path": path,
    }));

    Ok(json!({
        "configHome": config_home,
        "configPath": path,
        "sourceRoot": source_root,
        "repository": repository,
        "config": config,
    }))
}

pub fn opencode_ensure_path(
    args: &CliArgs,
    run: &mut RunContext,
) -> skills_manager_core::Result<Value> {
    let skills_root = if let Some(path) = args.option("skills-root") {
        expand_path_from_cwd(Path::new(&path))?
    } else {
        active_source_root(args)?.join("skills")
    };
    let config_path = args.option("config-path").map(PathBuf::from);
    let report = ensure_opencode_skill_path(config_path, &skills_root, &run.run_id)?;
    run.add_action(json!({
        "type": "opencode-skill-path",
        "status": match report.status {
            OpenCodePathStatus::Applied => "applied",
            OpenCodePathStatus::AlreadyPresent => "skipped",
            OpenCodePathStatus::ConfigConflict => "conflict",
        },
        "targetPath": report.config_path,
        "sourcePath": report.skills_root,
        "message": report.message,
    }));
    Ok(json!({ "opencode": report }))
}

pub fn scan(args: &CliArgs) -> skills_manager_core::Result<Value> {
    let source_root = if let Some(source_root) = args.option("source-root") {
        expand_path_from_cwd(Path::new(&source_root))?
    } else if args.command.len() > 1 {
        expand_path_from_cwd(Path::new(&args.command[1]))?
    } else {
        active_source_root(args)?
    };

    let skills = scan_source(&source_root)?;
    Ok(json!({
        "sourceRoot": source_root,
        "skills": skills,
    }))
}

pub fn refresh_config(args: &CliArgs, run: &mut RunContext) -> skills_manager_core::Result<Value> {
    let config_home = args.config_home();
    let path = config_path(&config_home);
    let mut config = read_workspace_config(&path)?;
    let source_root = resolve_active_source_root(&config)?;
    let skills = scan_source(&source_root)?;
    let skill_ids = skills
        .iter()
        .map(|skill| skill.skill_id.clone())
        .collect::<Vec<_>>();

    let mut added = 0usize;
    for environment in &mut config.environments {
        for agent in &mut environment.agents {
            if !agent.managed {
                continue;
            }
            for skill_id in &skill_ids {
                if agent
                    .enabled_skill_ids
                    .iter()
                    .any(|existing| existing == skill_id)
                {
                    continue;
                }
                agent.enabled_skill_ids.push(skill_id.clone());
                added += 1;
            }
            agent.enabled_skill_ids.sort();
        }
    }

    write_workspace_config(&path, &config)?;
    run.add_action(json!({
        "type": "refresh-config",
        "status": "applied",
        "path": path,
        "message": format!("added {added} enabled skill entries"),
    }));

    Ok(json!({
        "configPath": path,
        "sourceRoot": source_root,
        "added": added,
        "config": config,
    }))
}

pub fn status(args: &CliArgs) -> skills_manager_core::Result<Value> {
    if let Some(source_root) = args.option("source-root") {
        return status_legacy(args, PathBuf::from(source_root));
    }
    if args.command.len() >= 4 {
        return status_legacy(args, PathBuf::from(&args.command[1]));
    }

    let (config, source_root, env_config) = config_context(args)?;
    let agent_filter = optional_agent(args)?;
    let skills = scan_source(&source_root)?;
    let mut statuses = Vec::new();

    for agent in &env_config.agents {
        if agent_filter.is_some() && agent_filter != Some(agent.agent_id) {
            continue;
        }
        statuses.extend(compute_agent_statuses(&env_config, agent.agent_id, &skills));
    }

    Ok(json!({
        "activeSourceProfileId": config.active_source_profile_id,
        "sourceRoot": source_root,
        "environmentId": env_config.environment_id,
        "statuses": statuses,
    }))
}

pub fn reconcile(args: &CliArgs, run: &mut RunContext) -> skills_manager_core::Result<Value> {
    let mode = if args.flag("plan") {
        ReconcileMode::Plan
    } else {
        ReconcileMode::Apply
    };

    if let Some(source_root) = args.option("source-root") {
        return reconcile_legacy(args, run, PathBuf::from(source_root), mode);
    }
    if args.command.len() >= 4 {
        return reconcile_legacy(args, run, PathBuf::from(&args.command[1]), mode);
    }

    let config_home = args.config_home();
    init_config_home(&config_home)?;
    let config = read_workspace_config(&config_path(&config_home))?;
    let source_root = resolve_active_source_root(&config)?;
    let env_config =
        skills_manager_core::config::environment(&config, args.option("environment").as_deref())?;
    let agent_filter = optional_agent(args)?;
    let skills = scan_source(&source_root)?;
    let mut state = read_workspace_state(&state_path(&config_home))?;
    let mut reports = Vec::new();

    for agent in &env_config.agents {
        if agent_filter.is_some() && agent_filter != Some(agent.agent_id) {
            continue;
        }
        let report =
            reconcile_agent_with_state(env_config, agent.agent_id, &skills, mode, &mut state)?;
        for action in &report.actions {
            run.add_action(reconcile_action_json(action));
        }
        reports.push(report);
    }

    if mode == ReconcileMode::Apply {
        write_workspace_state(&state_path(&config_home), &state)?;
    }

    Ok(json!({
        "sourceRoot": source_root,
        "environmentId": env_config.environment_id,
        "mode": mode,
        "reports": reports,
        "statePath": state_path(&config_home),
    }))
}

pub fn init_repo(args: &CliArgs) -> skills_manager_core::Result<Value> {
    let source_root = required_path_option(args, "source-root", "init-repo --source-root <path>")?;
    let source_root = expand_path_from_cwd(&source_root)?;
    let metadata = init_or_update_repository_metadata(
        &source_root,
        args.option("name")
            .as_deref()
            .or(args.option("repo-id").as_deref()),
    )?;
    Ok(json!({
        "sourceRoot": source_root,
        "repository": metadata,
    }))
}

pub fn cache_init(args: &CliArgs) -> skills_manager_core::Result<Value> {
    let cache_root = required_path_value(
        args,
        2,
        "cache-root",
        "cache init <cache-root> <repo-id> <source-profile-id>",
    )?;
    let repo_id = args
        .option("repo-id")
        .or_else(|| args.command.get(3).cloned())
        .ok_or_else(|| {
            skills_manager_core::Error::InvalidInput("repo-id is required".to_string())
        })?;
    let source_profile_id = args
        .option("source-profile-id")
        .or_else(|| args.command.get(4).cloned())
        .ok_or_else(|| {
            skills_manager_core::Error::InvalidInput("source-profile-id is required".to_string())
        })?;
    let cache_root = expand_path_from_cwd(Path::new(&cache_root))?;
    let marker = CacheMarker::new(repo_id, source_profile_id);
    init_cache_marker(&cache_root, &marker)?;
    Ok(json!({
        "cacheRoot": cache_root,
        "marker": marker,
    }))
}

pub fn cache_verify(args: &CliArgs) -> skills_manager_core::Result<Value> {
    let cache_root = required_path_value(
        args,
        2,
        "cache-root",
        "cache verify <cache-root> <repo-id> <source-profile-id>",
    )?;
    let repo_id = args
        .option("repo-id")
        .or_else(|| args.command.get(3).cloned())
        .ok_or_else(|| {
            skills_manager_core::Error::InvalidInput("repo-id is required".to_string())
        })?;
    let source_profile_id = args
        .option("source-profile-id")
        .or_else(|| args.command.get(4).cloned())
        .ok_or_else(|| {
            skills_manager_core::Error::InvalidInput("source-profile-id is required".to_string())
        })?;
    let cache_root = expand_path_from_cwd(Path::new(&cache_root))?;
    let marker = CacheMarker::new(repo_id, source_profile_id);
    verify_cache_marker(&cache_root, &marker)?;
    Ok(json!({
        "cacheRoot": cache_root,
        "marker": marker,
        "verified": true,
    }))
}

pub fn hook_status(args: &CliArgs) -> skills_manager_core::Result<Value> {
    let agent = optional_agent(args)?;
    let statuses = BUILTIN_AGENTS
        .into_iter()
        .filter(|agent_id| agent.is_none() || agent == Some(*agent_id))
        .map(core_hook_status)
        .collect::<Vec<_>>();
    Ok(json!({ "hooks": statuses }))
}

pub fn hook_install(args: &CliArgs) -> skills_manager_core::Result<Value> {
    let agent = required_agent(args, "hook install --agent <agent>")?;
    let status = install_hook(agent)?;
    Ok(json!({ "hook": status }))
}

fn status_legacy(args: &CliArgs, source_root: PathBuf) -> skills_manager_core::Result<Value> {
    let source_root = expand_path_from_cwd(&source_root)?;
    let agent_id = if let Some(agent) = args.option("agent") {
        parse_agent_id(&agent)?
    } else {
        parse_agent_id(required_positional(
            args,
            2,
            "status <source-root> <agent> <skills-dir>",
        )?)?
    };
    let skills_dir = if let Some(skills_dir) = args.option("skills-dir") {
        PathBuf::from(skills_dir)
    } else {
        PathBuf::from(required_positional(
            args,
            3,
            "status <source-root> <agent> <skills-dir>",
        )?)
    };
    let skills = scan_source(&source_root)?;
    let enabled_skill_ids = skills.iter().map(|skill| skill.skill_id.clone()).collect();
    let env_config = EnvironmentConfig::local(
        "local",
        vec![AgentConfig {
            agent_id,
            managed: true,
            skills_dir,
            enabled_skill_ids,
        }],
    );
    let statuses = compute_agent_statuses(&env_config, agent_id, &skills);
    Ok(json!({
        "sourceRoot": source_root,
        "environmentId": "local",
        "statuses": statuses,
    }))
}

fn reconcile_legacy(
    args: &CliArgs,
    run: &mut RunContext,
    source_root: PathBuf,
    mode: ReconcileMode,
) -> skills_manager_core::Result<Value> {
    let source_root = expand_path_from_cwd(&source_root)?;
    let agent_id = if let Some(agent) = args.option("agent") {
        parse_agent_id(&agent)?
    } else {
        parse_agent_id(required_positional(
            args,
            2,
            "reconcile <source-root> <agent> <skills-dir>",
        )?)?
    };
    let skills_dir = if let Some(skills_dir) = args.option("skills-dir") {
        PathBuf::from(skills_dir)
    } else {
        PathBuf::from(required_positional(
            args,
            3,
            "reconcile <source-root> <agent> <skills-dir>",
        )?)
    };
    let skills = scan_source(&source_root)?;
    let enabled_skill_ids = skills.iter().map(|skill| skill.skill_id.clone()).collect();
    let env_config = EnvironmentConfig::local(
        "local",
        vec![AgentConfig {
            agent_id,
            managed: true,
            skills_dir,
            enabled_skill_ids,
        }],
    );
    let report = reconcile_agent(&env_config, agent_id, &skills, mode)?;
    for action in &report.actions {
        run.add_action(reconcile_action_json(action));
    }
    Ok(json!({
        "sourceRoot": source_root,
        "mode": mode,
        "reports": [report],
    }))
}
