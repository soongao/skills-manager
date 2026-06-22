use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::agents::AgentId;
use crate::error::{Error, Result};
use crate::model::{EnvironmentConfig, Skill};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteLinkPlan {
    pub host: String,
    pub user: String,
    pub environment_id: String,
    pub source_root: PathBuf,
    pub links: Vec<RemoteLinkItem>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteLinkItem {
    pub agent_id: AgentId,
    pub skill_id: String,
    pub source_path: PathBuf,
    pub target_path: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteLinkReport {
    pub plan: RemoteLinkPlan,
    pub actions: Vec<RemoteLinkAction>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteLinkAction {
    pub status: String,
    pub agent_id: AgentId,
    pub skill_id: String,
    pub source_path: PathBuf,
    pub target_path: PathBuf,
    pub message: Option<String>,
}

pub fn plan_remote_links(
    env_config: &EnvironmentConfig,
    source_root: &Path,
    skills: &[Skill],
) -> Result<RemoteLinkPlan> {
    let host = env_config.host.clone().ok_or_else(|| {
        Error::InvalidInput(format!(
            "host is required for environment {}",
            env_config.environment_id
        ))
    })?;
    let user = env_config.user.clone().ok_or_else(|| {
        Error::InvalidInput(format!(
            "user is required for environment {}",
            env_config.environment_id
        ))
    })?;
    let mut links = Vec::new();

    for agent in &env_config.agents {
        if !agent.managed {
            continue;
        }
        for skill_id in &agent.enabled_skill_ids {
            let Some(skill) = skills.iter().find(|skill| &skill.skill_id == skill_id) else {
                continue;
            };
            links.push(RemoteLinkItem {
                agent_id: agent.agent_id,
                skill_id: skill.skill_id.clone(),
                source_path: source_root.join("skills").join(&skill.skill_id),
                target_path: agent.skills_dir.join(&skill.skill_id),
            });
        }
    }

    Ok(RemoteLinkPlan {
        host,
        user,
        environment_id: env_config.environment_id.clone(),
        source_root: source_root.to_path_buf(),
        links,
    })
}

pub fn execute_remote_link_plan(plan: &RemoteLinkPlan) -> Result<RemoteLinkReport> {
    if plan.links.is_empty() {
        return Ok(RemoteLinkReport {
            plan: plan.clone(),
            actions: Vec::new(),
        });
    }

    let remote = format!("{}@{}", plan.user, plan.host);
    let script = remote_link_script(&plan.links)?;
    let output = Command::new("ssh")
        .arg(&remote)
        .arg(script)
        .output()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                Error::CommandUnavailable("ssh".to_string())
            } else {
                Error::io("failed to execute ssh", err)
            }
        })?;

    if !output.status.success() {
        return Err(Error::CommandFailed {
            program: "ssh".to_string(),
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(RemoteLinkReport {
        plan: plan.clone(),
        actions: parse_remote_link_actions(plan, &stdout),
    })
}

fn remote_link_script(links: &[RemoteLinkItem]) -> Result<String> {
    let payload = serde_json::to_string(links).map_err(|err| Error::Json {
        context: "failed to serialize remote link plan".to_string(),
        source: err,
    })?;
    Ok(format!(
        concat!(
            "python3 - <<'PY'\n",
            "import json, os, pathlib\n",
            "items = json.loads({payload:?})\n",
            "for item in items:\n",
            "    source = pathlib.Path(os.path.expanduser(item['sourcePath']))\n",
            "    target = pathlib.Path(os.path.expanduser(item['targetPath']))\n",
            "    status = 'applied'\n",
            "    message = None\n",
            "    target.parent.mkdir(parents=True, exist_ok=True)\n",
            "    if not source.exists():\n",
            "        status = 'failed'\n",
            "        message = 'source skill path does not exist'\n",
            "    elif target.is_symlink():\n",
            "        current = os.readlink(target)\n",
            "        if current == str(source):\n",
            "            status = 'skipped'\n",
            "            message = 'managed target already points to source'\n",
            "        else:\n",
            "            status = 'conflict'\n",
            "            message = 'target symlink points elsewhere'\n",
            "    elif target.exists():\n",
            "        status = 'conflict'\n",
            "        message = 'target path already exists'\n",
            "    else:\n",
            "        target.symlink_to(source, target_is_directory=True)\n",
            "    print(json.dumps({{\n",
            "        'status': status,\n",
            "        'agentId': item['agentId'],\n",
            "        'skillId': item['skillId'],\n",
            "        'sourcePath': str(source),\n",
            "        'targetPath': str(target),\n",
            "        'message': message,\n",
            "    }}, separators=(',', ':')))\n",
            "PY\n"
        ),
        payload = payload
    ))
}

fn parse_remote_link_actions(plan: &RemoteLinkPlan, stdout: &str) -> Vec<RemoteLinkAction> {
    let mut actions = Vec::new();
    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        if let Ok(action) = serde_json::from_str::<RemoteLinkAction>(line) {
            actions.push(action);
        }
    }

    if actions.len() == plan.links.len() {
        return actions;
    }

    actions.extend(
        plan.links[actions.len()..]
            .iter()
            .map(|item| RemoteLinkAction {
                status: "failed".to_string(),
                agent_id: item.agent_id,
                skill_id: item.skill_id.clone(),
                source_path: item.source_path.clone(),
                target_path: item.target_path.clone(),
                message: Some("remote link command did not report this item".to_string()),
            }),
    );
    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::AgentId;
    use crate::model::{AgentConfig, EnvironmentKind, SyncDirection};

    #[test]
    fn remote_link_plan_uses_remote_cache_and_agent_paths() {
        let env = EnvironmentConfig {
            environment_id: "devbox".to_string(),
            kind: EnvironmentKind::Remote,
            host: Some("devbox".to_string()),
            user: Some("alice".to_string()),
            sync_direction: Some(SyncDirection::PushLocalToRemote),
            remote_cache_root: Some(PathBuf::from("~/.skills-manager/cache/personal")),
            auto_sync: false,
            delete_extraneous: true,
            agents: vec![AgentConfig {
                agent_id: AgentId::Codex,
                managed: true,
                skills_dir: PathBuf::from("~/.codex/skills"),
                enabled_skill_ids: vec!["design-clarifier".to_string()],
            }],
        };
        let plan = plan_remote_links(
            &env,
            Path::new("~/.skills-manager/cache/personal"),
            &[Skill {
                skill_id: "design-clarifier".to_string(),
                path: PathBuf::from("/local/source/skills/design-clarifier"),
            }],
        )
        .unwrap();

        assert_eq!(
            plan.links[0].source_path,
            PathBuf::from("~/.skills-manager/cache/personal/skills/design-clarifier")
        );
        assert_eq!(
            plan.links[0].target_path,
            PathBuf::from("~/.codex/skills/design-clarifier")
        );
    }
}
