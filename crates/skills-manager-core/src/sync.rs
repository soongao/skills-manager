use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cache::{verify_cache_marker, CacheMarker};
use crate::error::{Error, Result};
use crate::model::SyncDirection;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPlan {
    pub direction: SyncDirection,
    pub delete_extraneous: bool,
    pub preflight: Vec<PreflightCheck>,
    pub commands: Vec<RsyncCommand>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum PreflightCheck {
    LocalCacheMarker {
        cache_root: PathBuf,
        marker: CacheMarker,
    },
    RemoteCacheMarker {
        host: String,
        user: String,
        cache_root: PathBuf,
        marker: CacheMarker,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RsyncCommand {
    pub source: String,
    pub destination: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncExecutionReport {
    pub plan: SyncPlan,
    pub actions: Vec<SyncExecutionAction>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncExecutionAction {
    pub kind: String,
    pub status: String,
    pub command: Option<Vec<String>>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRemoteToLocal {
    pub host: String,
    pub user: String,
    pub remote_source_root: PathBuf,
    pub local_cache_root: PathBuf,
    pub marker: CacheMarker,
    pub delete_extraneous: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PushLocalToRemote {
    pub host: String,
    pub user: String,
    pub local_source_root: PathBuf,
    pub remote_cache_root: PathBuf,
    pub marker: CacheMarker,
    pub delete_extraneous: bool,
}

pub fn plan_pull_remote_to_local(input: &PullRemoteToLocal) -> Result<SyncPlan> {
    if input.delete_extraneous {
        verify_cache_marker(&input.local_cache_root, &input.marker)?;
    }

    let skills_source = format!(
        "{}@{}:{}/",
        input.user,
        input.host,
        remote_skills_path(&input.remote_source_root)
    );
    let skills_destination = format!("{}/", local_skills_path(&input.local_cache_root).display());
    let repo_source = format!(
        "{}@{}:{}",
        input.user,
        input.host,
        remote_repository_path(&input.remote_source_root)
    );
    let repo_destination = local_repository_path(&input.local_cache_root)
        .display()
        .to_string();
    let preflight = if input.delete_extraneous {
        vec![PreflightCheck::LocalCacheMarker {
            cache_root: input.local_cache_root.clone(),
            marker: input.marker.clone(),
        }]
    } else {
        Vec::new()
    };

    Ok(build_rsync_plan(
        SyncDirection::PullRemoteToLocal,
        preflight,
        vec![
            build_rsync_command(skills_source, skills_destination, input.delete_extraneous),
            build_rsync_command(repo_source, repo_destination, false),
        ],
        input.delete_extraneous,
    ))
}

pub fn plan_push_local_to_remote(input: &PushLocalToRemote) -> SyncPlan {
    let skills_source = format!("{}/", local_skills_path(&input.local_source_root).display());
    let skills_destination = format!(
        "{}@{}:{}/",
        input.user,
        input.host,
        remote_skills_path(&input.remote_cache_root)
    );
    let repo_source = local_repository_path(&input.local_source_root)
        .display()
        .to_string();
    let repo_destination = format!(
        "{}@{}:{}",
        input.user,
        input.host,
        remote_repository_path(&input.remote_cache_root)
    );
    let preflight = if input.delete_extraneous {
        vec![PreflightCheck::RemoteCacheMarker {
            host: input.host.clone(),
            user: input.user.clone(),
            cache_root: input.remote_cache_root.clone(),
            marker: input.marker.clone(),
        }]
    } else {
        Vec::new()
    };

    build_rsync_plan(
        SyncDirection::PushLocalToRemote,
        preflight,
        vec![
            build_rsync_command(skills_source, skills_destination, input.delete_extraneous),
            build_rsync_command(repo_source, repo_destination, false),
        ],
        input.delete_extraneous,
    )
}

pub fn execute_sync_plan(plan: &SyncPlan) -> Result<SyncExecutionReport> {
    let mut actions = Vec::new();

    for check in &plan.preflight {
        run_preflight(check)?;
        actions.push(SyncExecutionAction {
            kind: "preflight".to_string(),
            status: "applied".to_string(),
            command: None,
            message: Some(preflight_message(check)),
        });
    }

    for command in &plan.commands {
        let mut full_command = vec!["rsync".to_string()];
        full_command.extend(command.args.clone());
        let output = Command::new("rsync")
            .args(&command.args)
            .output()
            .map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    Error::CommandUnavailable("rsync".to_string())
                } else {
                    Error::io("failed to execute rsync", err)
                }
            })?;

        if !output.status.success() {
            return Err(Error::CommandFailed {
                program: "rsync".to_string(),
                status: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        actions.push(SyncExecutionAction {
            kind: "rsync".to_string(),
            status: "applied".to_string(),
            command: Some(full_command),
            message: None,
        });
    }

    Ok(SyncExecutionReport {
        plan: plan.clone(),
        actions,
    })
}

pub fn run_preflight(check: &PreflightCheck) -> Result<()> {
    match check {
        PreflightCheck::LocalCacheMarker { cache_root, marker } => {
            verify_cache_marker(cache_root, marker)
        }
        PreflightCheck::RemoteCacheMarker {
            host,
            user,
            cache_root,
            marker,
        } => verify_remote_cache_marker(host, user, cache_root, marker),
    }
}

pub fn verify_remote_cache_marker(
    host: &str,
    user: &str,
    cache_root: &Path,
    marker: &CacheMarker,
) -> Result<()> {
    let remote = format!("{user}@{host}");
    let marker_path = cache_root.join(crate::cache::CACHE_MARKER_FILE);
    let script = format!(
        concat!(
            "python3 - <<'PY'\n",
            "import json, pathlib, sys\n",
            "path = pathlib.Path({path:?})\n",
            "expected_repo = {repo:?}\n",
            "expected_source = {source:?}\n",
            "if not path.is_file():\n",
            "    sys.exit(11)\n",
            "data = json.loads(path.read_text())\n",
            "if data.get('managedBy') != 'skills-manager' or data.get('repoId') != expected_repo or data.get('sourceProfileId') != expected_source:\n",
            "    sys.exit(12)\n",
            "PY\n"
        ),
        path = marker_path.display().to_string(),
        repo = marker.repo_id,
        source = marker.source_profile_id
    );
    let output = Command::new("ssh")
        .arg(remote)
        .arg(script)
        .output()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                Error::CommandUnavailable("ssh".to_string())
            } else {
                Error::io("failed to execute ssh", err)
            }
        })?;

    if output.status.success() {
        return Ok(());
    }

    match output.status.code() {
        Some(11) => Err(Error::CacheMarkerMissing(marker_path.display().to_string())),
        Some(12) => Err(Error::CacheMarkerMismatch(
            marker_path.display().to_string(),
        )),
        _ => Err(Error::CommandFailed {
            program: "ssh".to_string(),
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }),
    }
}

fn build_rsync_plan(
    direction: SyncDirection,
    preflight: Vec<PreflightCheck>,
    commands: Vec<RsyncCommand>,
    delete_extraneous: bool,
) -> SyncPlan {
    SyncPlan {
        direction,
        delete_extraneous,
        preflight,
        commands,
    }
}

fn build_rsync_command(
    source: String,
    destination: String,
    delete_extraneous: bool,
) -> RsyncCommand {
    let mut args = vec!["-az".to_string()];
    if delete_extraneous {
        args.push("--delete".to_string());
    }
    args.push(source.clone());
    args.push(destination.clone());

    RsyncCommand {
        source,
        destination,
        args,
    }
}

fn preflight_message(check: &PreflightCheck) -> String {
    match check {
        PreflightCheck::LocalCacheMarker { cache_root, .. } => {
            format!("verified local cache marker at {}", cache_root.display())
        }
        PreflightCheck::RemoteCacheMarker {
            host,
            user,
            cache_root,
            ..
        } => format!(
            "verified remote cache marker at {}@{}:{}",
            user,
            host,
            cache_root.display()
        ),
    }
}

fn local_skills_path(root: &Path) -> PathBuf {
    root.join("skills")
}

fn remote_skills_path(root: &Path) -> String {
    format!("{}/skills", root.display())
}

fn local_repository_path(root: &Path) -> PathBuf {
    root.join(".skills-manager").join("repository.json")
}

fn remote_repository_path(root: &Path) -> String {
    format!("{}/.skills-manager/repository.json", root.display())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::init_cache_marker;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn pull_plan_requires_marker_when_delete_enabled() {
        let cache_root = test_dir("pull-marker");
        let marker = CacheMarker::new("repo", "source");
        let input = PullRemoteToLocal {
            host: "devbox".to_string(),
            user: "alice".to_string(),
            remote_source_root: PathBuf::from("/srv/team-skills"),
            local_cache_root: cache_root.clone(),
            marker: marker.clone(),
            delete_extraneous: true,
        };

        assert!(plan_pull_remote_to_local(&input).is_err());
        init_cache_marker(&cache_root, &marker).unwrap();
        assert!(plan_pull_remote_to_local(&input).is_ok());
    }

    #[test]
    fn push_plan_builds_rsync_arguments() {
        let input = PushLocalToRemote {
            host: "devbox".to_string(),
            user: "alice".to_string(),
            local_source_root: PathBuf::from("/Users/alice/skills"),
            remote_cache_root: PathBuf::from("~/.skills-manager/cache/personal"),
            marker: CacheMarker::new("repo", "source"),
            delete_extraneous: true,
        };

        let plan = plan_push_local_to_remote(&input);
        assert_eq!(plan.direction, SyncDirection::PushLocalToRemote);
        assert_eq!(plan.preflight.len(), 1);
        assert_eq!(plan.commands.len(), 2);
        assert!(plan.commands[0].args.iter().any(|arg| arg == "--delete"));
        assert!(!plan.commands[1].args.iter().any(|arg| arg == "--delete"));
        assert_eq!(plan.commands[0].source, "/Users/alice/skills/skills/");
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
