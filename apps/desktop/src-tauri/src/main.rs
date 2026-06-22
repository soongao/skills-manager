mod commands;
mod tray;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            tray::setup(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_dashboard,
            commands::init_config,
            commands::set_local_source,
            commands::set_agent_dir,
            commands::set_skill_enabled,
            commands::reconcile,
            commands::opencode_ensure_path,
            commands::set_remote_source,
            commands::set_remote_environment,
            commands::remote_sync,
            commands::remote_cli_status
        ])
        .on_window_event(tray::hide_on_close)
        .run(tauri::generate_context!())
        .expect("failed to run Skills Manager desktop");
}
