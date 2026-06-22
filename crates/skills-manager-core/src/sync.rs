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
    pub prepare: Vec<PrepareStep>,
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
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum PrepareStep {
    LocalMkdir {
        paths: Vec<PathBuf>,
    },
    RemoteMkdir {
        host: String,
        user: String,
        paths: Vec<PathBuf>,
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
        vec![PrepareStep::LocalMkdir {
            paths: vec![
                local_skills_path(&input.local_cache_root),
                local_repository_dir(&input.local_cache_root),
            ],
        }],
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
        vec![PrepareStep::RemoteMkdir {
            host: input.host.clone(),
            user: input.user.clone(),
            paths: vec![
                remote_skills_path_buf(&input.remote_cache_root),
                remote_repository_dir(&input.remote_cache_root),
            ],
        }],
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

    for step in &plan.prepare {
        run_prepare(step)?;
        actions.push(SyncExecutionAction {
            kind: "prepare".to_string(),
            status: "applied".to_string(),
            command: Some(prepare_command_display(step)),
            message: Some(prepare_message(step)),
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

pub fn run_prepare(step: &PrepareStep) -> Result<()> {
    match step {
        PrepareStep::LocalMkdir { paths } => {
            for path in paths {
                std::fs::create_dir_all(path).map_err(|err| {
                    Error::io(format!("failed to create {}", path.display()), err)
                })?;
            }
            Ok(())
        }
        PrepareStep::RemoteMkdir { host, user, paths } => {
            let remote = format!("{user}@{host}");
            let script = remote_mkdir_script(paths);
            let mut command = Command::new("ssh");
            command.arg(&remote).arg(script);
            let output = command.output().map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    Error::CommandUnavailable("ssh".to_string())
                } else {
                    Error::io("failed to execute ssh", err)
                }
            })?;

            if output.status.success() {
                return Ok(());
            }

            Err(Error::CommandFailed {
                program: "ssh".to_string(),
                status: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
        }
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
    prepare: Vec<PrepareStep>,
    commands: Vec<RsyncCommand>,
    delete_extraneous: bool,
) -> SyncPlan {
    SyncPlan {
        direction,
        delete_extraneous,
        preflight,
        prepare,
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

fn prepare_message(step: &PrepareStep) -> String {
    match step {
        PrepareStep::LocalMkdir { paths } => format!(
            "created local directories {}",
            paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        PrepareStep::RemoteMkdir { host, user, paths } => format!(
            "created remote directories on {}@{}: {}",
            user,
            host,
            paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn prepare_command_display(step: &PrepareStep) -> Vec<String> {
    match step {
        PrepareStep::LocalMkdir { paths } => {
            let mut command = vec!["mkdir".to_string(), "-p".to_string()];
            command.extend(paths.iter().map(|path| path.display().to_string()));
            command
        }
        PrepareStep::RemoteMkdir { host, user, paths } => {
            let mut command = vec![
                "ssh".to_string(),
                format!("{user}@{host}"),
                "mkdir".to_string(),
                "-p".to_string(),
            ];
            command.extend(paths.iter().map(|path| path.display().to_string()));
            command
        }
    }
}

fn remote_mkdir_script(paths: &[PathBuf]) -> String {
    let mut script = String::from("mkdir -p --");
    for path in paths {
        script.push(' ');
        script.push_str(&shell_quote(&path.display().to_string()));
    }
    script
}

fn shell_quote(value: &str) -> String {
    if let Some(rest) = value.strip_prefix("~/") {
        return format!("$HOME/{quoted}", quoted = shell_quote(rest));
    }
    if value == "~" {
        return "$HOME".to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn local_skills_path(root: &Path) -> PathBuf {
    root.join("skills")
}

fn remote_skills_path(root: &Path) -> String {
    format!("{}/skills", root.display())
}

fn remote_skills_path_buf(root: &Path) -> PathBuf {
    root.join("skills")
}

fn local_repository_path(root: &Path) -> PathBuf {
    root.join(".skills-manager").join("repository.json")
}

fn local_repository_dir(root: &Path) -> PathBuf {
    root.join(".skills-manager")
}

fn remote_repository_path(root: &Path) -> String {
    format!("{}/.skills-manager/repository.json", root.display())
}

fn remote_repository_dir(root: &Path) -> PathBuf {
    root.join(".skills-manager")
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
        let plan = plan_pull_remote_to_local(&input).unwrap();
        assert_eq!(plan.prepare.len(), 1);
        match &plan.prepare[0] {
            PrepareStep::LocalMkdir { paths } => {
                assert!(paths.contains(&cache_root.join("skills")));
                assert!(paths.contains(&cache_root.join(".skills-manager")));
            }
            _ => panic!("expected local mkdir"),
        }
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
        assert_eq!(plan.prepare.len(), 1);
        match &plan.prepare[0] {
            PrepareStep::RemoteMkdir { paths, .. } => {
                assert!(paths.contains(&PathBuf::from("~/.skills-manager/cache/personal/skills")));
                assert!(paths.contains(&PathBuf::from(
                    "~/.skills-manager/cache/personal/.skills-manager"
                )));
            }
            _ => panic!("expected remote mkdir"),
        }
        assert_eq!(plan.commands.len(), 2);
        assert!(plan.commands[0].args.iter().any(|arg| arg == "--delete"));
        assert!(!plan.commands[1].args.iter().any(|arg| arg == "--delete"));
        assert_eq!(plan.commands[0].source, "/Users/alice/skills/skills/");
    }

    #[test]
    fn remote_mkdir_script_uses_recursive_mkdir_and_preserves_home_expansion() {
        let script = remote_mkdir_script(&[
            PathBuf::from("~/.skills-manager/cache/personal/skills"),
            PathBuf::from("/home/alice/has space/.skills-manager"),
        ]);

        assert_eq!(
            script,
            "mkdir -p -- $HOME/'.skills-manager/cache/personal/skills' '/home/alice/has space/.skills-manager'"
        );
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
