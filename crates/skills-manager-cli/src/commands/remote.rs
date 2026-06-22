use std::process::Command;

use serde_json::{json, Value};
use skills_manager_core::cache::CacheMarker;
use skills_manager_core::config::{
    active_source_profile, config_path, environment, init_config_home, read_workspace_config,
    read_workspace_state, state_path,
};
use skills_manager_core::model::{SourceProfile, SyncDirection};
use skills_manager_core::paths::expand_path_from_cwd;
use skills_manager_core::sync::{
    execute_sync_plan, plan_pull_remote_to_local, plan_push_local_to_remote, PullRemoteToLocal,
    PushLocalToRemote,
};

use crate::args::CliArgs;
use crate::output::RunContext;
use crate::util::{parse_sync_direction, required_env_host, required_env_user};

pub fn sync(args: &CliArgs, run: &mut RunContext) -> skills_manager_core::Result<Value> {
    let config_home = args.config_home();
    init_config_home(&config_home)?;
    let config = read_workspace_config(&config_path(&config_home))?;
    let source = active_source_profile(&config)?;
    let direction = args
        .option("direction")
        .map(|value| parse_sync_direction(&value))
        .transpose()?;
    let env_config = environment(&config, args.option("environment").as_deref())?;
    let repo_id = args
        .option("repo-id")
        .unwrap_or_else(|| config.active_source_profile_id.clone());
    let marker = CacheMarker::new(repo_id, config.active_source_profile_id.clone());

    let plan = match direction.or(env_config.sync_direction) {
        Some(SyncDirection::PushLocalToRemote) => {
            let SourceProfile::Local(local) = source else {
                return Err(skills_manager_core::Error::InvalidInput(
                    "push-local-to-remote requires a local active source".to_string(),
                ));
            };
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
        Some(SyncDirection::PullRemoteToLocal) => {
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
        None => {
            return Err(skills_manager_core::Error::InvalidInput(
                "sync direction is required".to_string(),
            ));
        }
    };

    if args.flag("plan") {
        run.add_action(json!({
            "type": "sync-plan",
            "status": "planned",
            "direction": plan.direction,
        }));
        return Ok(json!({ "plan": plan }));
    }

    let report = execute_sync_plan(&plan)?;
    for action in &report.actions {
        run.add_action(json!({
            "type": action.kind,
            "status": action.status,
            "command": action.command,
            "message": action.message,
        }));
    }
    Ok(json!({ "sync": report }))
}

pub fn cli_status(args: &CliArgs) -> skills_manager_core::Result<Value> {
    let config = read_workspace_config(&config_path(&args.config_home()))?;
    let env_config = environment(&config, args.option("environment").as_deref())?;
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
            "environmentId": env_config.environment_id,
            "remote": remote,
            "available": false,
            "status": output.status.code(),
            "stderr": String::from_utf8_lossy(&output.stderr).trim(),
        }));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed = serde_json::from_str::<Value>(&stdout).unwrap_or_else(|_| {
        json!({
            "raw": stdout.trim(),
        })
    });
    Ok(json!({
        "environmentId": env_config.environment_id,
        "remote": remote,
        "available": true,
        "version": parsed,
    }))
}

pub fn status(args: &CliArgs) -> skills_manager_core::Result<Value> {
    let config_home = args.config_home();
    let config = read_workspace_config(&config_path(&config_home))?;
    let state = read_workspace_state(&state_path(&config_home))?;
    let env_config = environment(&config, args.option("environment").as_deref())?;
    let sync_state = state
        .sync
        .iter()
        .find(|sync| sync.environment_id == env_config.environment_id)
        .cloned();
    Ok(json!({
        "environment": env_config,
        "sync": sync_state,
    }))
}
