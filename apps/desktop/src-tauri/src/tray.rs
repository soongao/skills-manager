use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager};

const SETTINGS_LABEL: &str = "settings";

pub fn setup(app: &mut App) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    setup_tray(app.handle())
}

pub fn hide_on_close(window: &tauri::Window, event: &tauri::WindowEvent) {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.hide();
    }
}

pub fn show_settings(app: &AppHandle) -> tauri::Result<()> {
    show_window(app, SETTINGS_LABEL)
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let open_settings = MenuItem::with_id(app, "open-settings", "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Skills Manager", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open_settings, &separator, &quit])?;

    TrayIconBuilder::with_id("skills-manager")
        .tooltip("Skills Manager")
        .icon(tray_icon_image())
        .icon_as_template(true)
        .show_menu_on_left_click(false)
        .menu(&menu)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = show_settings(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open-settings" => {
                let _ = show_settings(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

fn tray_icon_image() -> Image<'static> {
    let size = 32u32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let inside = x > 3 && x < 28 && y > 3 && y < 28;
            let core = x > 8 && x < 23 && y > 8 && y < 23;
            let (r, g, b, a) = if core {
                (31, 45, 40, 255)
            } else if inside {
                (167, 216, 111, 255)
            } else {
                (0, 0, 0, 0)
            };
            rgba.extend_from_slice(&[r, g, b, a]);
        }
    }
    Image::new_owned(rgba, size, size)
}

fn show_window(app: &AppHandle, label: &str) -> tauri::Result<()> {
    let window = app
        .get_webview_window(label)
        .ok_or_else(|| tauri::Error::WindowNotFound)?;
    window.show()?;
    window.set_focus()?;
    Ok(())
}
