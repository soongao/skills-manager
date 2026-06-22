mod config;
mod general;
mod remote;

use serde_json::Value;

use crate::args::CliArgs;
use crate::output::RunContext;

pub fn dispatch(args: &CliArgs, run: &mut RunContext) -> skills_manager_core::Result<Value> {
    match args.command.first().map(String::as_str) {
        Some("detect") => general::detect(args),
        Some("version") => general::version(),
        Some("init-config") => general::init_config(args, run),
        Some("scan") => general::scan(args),
        Some("refresh-config") => general::refresh_config(args, run),
        Some("status") => general::status(args),
        Some("reconcile") => general::reconcile(args, run),
        Some("init-repo") => general::init_repo(args),
        Some("cache") if args.command.get(1).map(String::as_str) == Some("init") => {
            general::cache_init(args)
        }
        Some("cache") if args.command.get(1).map(String::as_str) == Some("verify") => {
            general::cache_verify(args)
        }
        Some("hook") if args.command.get(1).map(String::as_str) == Some("status") => {
            general::hook_status(args)
        }
        Some("hook") if args.command.get(1).map(String::as_str) == Some("install") => {
            general::hook_install(args)
        }
        Some("opencode") if args.command.get(1).map(String::as_str) == Some("ensure-path") => {
            general::opencode_ensure_path(args, run)
        }
        Some("config") if args.command.get(1).map(String::as_str) == Some("show") => {
            config::show(args)
        }
        Some("config") if args.command.get(1).map(String::as_str) == Some("set-local-source") => {
            config::set_local_source(args, run)
        }
        Some("config") if args.command.get(1).map(String::as_str) == Some("set-remote-source") => {
            config::set_remote_source(args, run)
        }
        Some("config") if args.command.get(1).map(String::as_str) == Some("set-agent") => {
            config::set_agent(args, run)
        }
        Some("config") if args.command.get(1).map(String::as_str) == Some("enable") => {
            config::set_skill_enabled(args, run, true)
        }
        Some("config") if args.command.get(1).map(String::as_str) == Some("disable") => {
            config::set_skill_enabled(args, run, false)
        }
        Some("config") if args.command.get(1).map(String::as_str) == Some("add-remote-env") => {
            config::add_remote_env(args, run)
        }
        Some("remote") if args.command.get(1).map(String::as_str) == Some("sync") => {
            remote::sync(args, run)
        }
        Some("remote") if args.command.get(1).map(String::as_str) == Some("cli-status") => {
            remote::cli_status(args)
        }
        Some("remote") if args.command.get(1).map(String::as_str) == Some("status") => {
            remote::status(args)
        }
        _ => Err(skills_manager_core::Error::InvalidInput(format!(
            "unknown command: {}",
            args.command.join(" ")
        ))),
    }
}
