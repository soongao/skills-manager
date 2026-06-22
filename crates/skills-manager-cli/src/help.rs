pub fn print_help() {
    println!("skills-manager");
    println!();
    println!("Commands:");
    println!("  version [--json]");
    println!("  detect [--json]");
    println!("  init-config --source-root <path> [--config-home <path>] [--<agent>-skills-dir <path>] [--json]");
    println!("  init-repo --source-root <path> [--name <name>] [--json]");
    println!("  refresh-config [--config-home <path>] [--json]");
    println!("  config show [--json]");
    println!("  config set-local-source --source-root <path> [--source-profile-id <id>] [--json]");
    println!("  config set-remote-source --host <host> --user <user> --remote-source-root <path> --local-cache-root <path> [--json]");
    println!(
        "  config set-agent --agent <agent> --skills-dir <path> [--environment <id>] [--json]"
    );
    println!("  config enable --agent <agent> --skill <skillId> [--environment <id>] [--json]");
    println!("  config disable --agent <agent> --skill <skillId> [--environment <id>] [--json]");
    println!("  config add-remote-env --environment <id> --host <host> --user <user> [--direction <direction>] [--json]");
    println!("  scan [--source-root <path>] [--json]");
    println!("  status [--agent <agent>] [--environment <id>] [--json]");
    println!("  reconcile [--agent <agent>] [--environment <id>] [--plan] [--json]");
    println!("  remote sync [--environment <id>] [--direction <direction>] [--plan] [--json]");
    println!("  remote status [--environment <id>] [--json]");
    println!("  remote cli-status --environment <id> [--json]");
    println!("  hook status [--agent <agent>] [--json]");
    println!("  hook install --agent <agent> [--json]");
    println!("  opencode ensure-path [--skills-root <path>] [--config-path <path>] [--json]");
    println!("  cache init <cache-root> <repo-id> <source-profile-id>");
    println!("  cache verify <cache-root> <repo-id> <source-profile-id>");
    println!();
    println!("Legacy manual testing forms are still accepted:");
    println!("  scan <source-root>");
    println!("  status <source-root> <agent> <skills-dir>");
    println!("  reconcile <source-root> <agent> <skills-dir>");
}
