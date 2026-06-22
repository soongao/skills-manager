use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::args::CliArgs;
use crate::time::{compact_timestamp, run_id, utc_now_string};

#[derive(Clone, Debug)]
pub struct RunContext {
    pub command: String,
    pub run_id: String,
    started_at: String,
    ended_at: Option<String>,
    config_home: std::path::PathBuf,
    pub ok: bool,
    status: String,
    actions: Vec<Value>,
    warnings: Vec<Value>,
    errors: Vec<Value>,
    payload: Value,
}

impl RunContext {
    pub fn new(args: &CliArgs) -> Self {
        Self {
            command: args.command.join(" "),
            run_id: run_id(),
            started_at: utc_now_string(),
            ended_at: None,
            config_home: args.config_home(),
            ok: true,
            status: "success".to_string(),
            actions: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
            payload: json!({}),
        }
    }

    pub fn add_action(&mut self, action: Value) {
        self.actions.push(action);
    }

    pub fn finish_success(&mut self, payload: Value) {
        self.ended_at = Some(utc_now_string());
        self.payload = payload;
        let conflicts = self
            .actions
            .iter()
            .filter(|action| {
                action
                    .get("status")
                    .and_then(Value::as_str)
                    .map(|status| status == "conflict")
                    .unwrap_or(false)
            })
            .count();
        let failed = self
            .actions
            .iter()
            .filter(|action| {
                action
                    .get("status")
                    .and_then(Value::as_str)
                    .map(|status| status == "failed")
                    .unwrap_or(false)
            })
            .count();
        self.ok = failed == 0;
        self.status = if failed > 0 {
            "failed".to_string()
        } else if conflicts > 0 {
            "partial".to_string()
        } else {
            "success".to_string()
        };
    }

    pub fn finish_error(&mut self, err: &skills_manager_core::Error) {
        self.ended_at = Some(utc_now_string());
        self.ok = false;
        self.status = "failed".to_string();
        self.errors.push(error_json(err));
    }

    pub fn output(&self) -> Value {
        json!({
            "schemaVersion": 1,
            "ok": self.ok,
            "status": self.status,
            "command": self.command,
            "runId": self.run_id,
            "startedAt": self.started_at,
            "endedAt": self.ended_at,
            "summary": self.summary(),
            "actions": self.actions,
            "warnings": self.warnings,
            "errors": self.errors,
            "result": self.payload,
        })
    }

    pub fn persist(&self) {
        if self.actions.is_empty() && self.errors.is_empty() {
            return;
        }

        let _ = skills_manager_core::config::init_config_home(&self.config_home);
        let output = self.output();
        let logs_dir = skills_manager_core::config::logs_dir(&self.config_home);
        let runs_dir = skills_manager_core::config::runs_dir(&self.config_home);
        let _ = fs::create_dir_all(&logs_dir);
        let _ = fs::create_dir_all(&runs_dir);

        let log_line = json!({
            "timestamp": utc_now_string(),
            "level": if self.ok { "info" } else { "error" },
            "runId": self.run_id,
            "component": "cli",
            "event": "command.finish",
            "message": format!("skills-manager {} finished with {}", self.command, self.status),
            "details": self.summary(),
        });
        let _ = append_line(&logs_dir.join("skills-manager.log"), &log_line.to_string());

        let file_name = format!(
            "{}-{}-{}.json",
            compact_timestamp(&self.started_at),
            self.run_id,
            sanitize_file_part(&self.command)
        );
        let _ = fs::write(
            runs_dir.join(file_name),
            serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string()),
        );
    }

    fn summary(&self) -> Value {
        let mut applied = 0;
        let mut skipped = 0;
        let mut conflicts = 0;
        let mut errors = self.errors.len();
        for action in &self.actions {
            match action.get("status").and_then(Value::as_str) {
                Some("applied") => applied += 1,
                Some("skipped") => skipped += 1,
                Some("conflict") => conflicts += 1,
                Some("failed") => errors += 1,
                _ => {}
            }
        }
        json!({
            "applied": applied,
            "skipped": skipped,
            "conflicts": conflicts,
            "errors": errors,
        })
    }
}

pub fn error_json(err: &skills_manager_core::Error) -> Value {
    json!({
        "code": error_code(err),
        "message": err.to_string(),
    })
}

pub fn error_code(err: &skills_manager_core::Error) -> &'static str {
    match err {
        skills_manager_core::Error::Io { .. } => "IO_ERROR",
        skills_manager_core::Error::InvalidInput(_) => "CONFIG_INVALID",
        skills_manager_core::Error::SourceNotFound(_) => "SOURCE_NOT_FOUND",
        skills_manager_core::Error::SourceInvalidLayout(_) => "SOURCE_INVALID_LAYOUT",
        skills_manager_core::Error::AgentNotConfigured(_) => "AGENT_NOT_DETECTED",
        skills_manager_core::Error::AgentSkillsDirInvalid(_) => "AGENT_SKILLS_DIR_INVALID",
        skills_manager_core::Error::CacheMarkerMissing(_) => "CACHE_MARKER_MISSING",
        skills_manager_core::Error::CacheMarkerMismatch(_) => "CACHE_MARKER_MISMATCH",
        skills_manager_core::Error::CommandUnavailable(program) if program == "ssh" => {
            "SYNC_SSH_UNAVAILABLE"
        }
        skills_manager_core::Error::CommandUnavailable(program) if program == "rsync" => {
            "SYNC_RSYNC_UNAVAILABLE"
        }
        skills_manager_core::Error::CommandUnavailable(_) => "COMMAND_UNAVAILABLE",
        skills_manager_core::Error::CommandFailed { .. } => "SYNC_FAILED",
        skills_manager_core::Error::Json { .. } => "CONFIG_INVALID",
    }
}

pub fn print_human(command: &[String], payload: Value) {
    match command.first().map(String::as_str) {
        Some("scan") => {
            if let Some(skills) = payload.get("skills").and_then(Value::as_array) {
                for skill in skills {
                    println!(
                        "{}\t{}",
                        skill.get("skillId").and_then(Value::as_str).unwrap_or(""),
                        skill.get("path").and_then(Value::as_str).unwrap_or("")
                    );
                }
            }
        }
        Some("status") => {
            if let Some(statuses) = payload.get("statuses").and_then(Value::as_array) {
                for status in statuses {
                    println!(
                        "{}\t{}\t{}",
                        status.get("status").and_then(Value::as_str).unwrap_or(""),
                        status.get("agentId").and_then(Value::as_str).unwrap_or(""),
                        status.get("skillId").and_then(Value::as_str).unwrap_or("")
                    );
                }
            }
        }
        Some("reconcile") => {
            if let Some(reports) = payload.get("reports").and_then(Value::as_array) {
                for report in reports {
                    if let Some(actions) = report.get("actions").and_then(Value::as_array) {
                        for action in actions {
                            println!(
                                "{}\t{}\t{}",
                                action.get("status").and_then(Value::as_str).unwrap_or(""),
                                action.get("kind").and_then(Value::as_str).unwrap_or(""),
                                action.get("skillId").and_then(Value::as_str).unwrap_or("")
                            );
                        }
                    }
                }
            }
        }
        _ => println!("{}", serde_json::to_string_pretty(&payload).unwrap()),
    }
}

fn append_line(path: &Path, line: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")
}

fn sanitize_file_part(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    sanitized.trim_matches('-').to_string()
}
