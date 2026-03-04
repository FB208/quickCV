use crate::logger;
use crate::overlay_window;
use crate::services::settings;
use crate::tray;
use tauri::AppHandle;
use tauri::Manager;
use tauri::WindowEvent;
use tauri_plugin_global_shortcut::ShortcutState;

pub fn handle_shortcut_event(app: &AppHandle, state: ShortcutState) {
    if state == ShortcutState::Pressed {
        let _ = overlay_window::open_overlay_with_context(
            app,
            overlay_window::OverlayContext::default(),
        );
    }
}

pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    if let Err(error) = logger::ensure_logs_dir(app.handle()) {
        eprintln!("[logger] initialize failed: {error}");
    }
    logger::info(app.handle(), "startup", "quickCV booting");
    app.manage(overlay_window::RuntimeState::default());
    tray::setup_tray(app.handle())?;

    if let Ok(settings) = settings::load_settings(app.handle()) {
        let _ = settings::register_main_shortcut(app.handle(), &settings.shortcut);
    }

    if is_autostart_launch() {
        logger::info(
            app.handle(),
            "startup",
            "detected autostart launch, hiding main window",
        );
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
    }

    overlay_window::prepare_overlay_window_noactivate(app.handle());
    Ok(())
}

fn is_autostart_launch() -> bool {
    std::env::args().any(|arg| {
        let value = arg.trim().to_lowercase();
        value == "--autostart"
            || value == "--autorun"
            || value == "-autostart"
            || value == "/autostart"
            || value.contains("autostart")
    })
}

pub fn handle_window_event(window: &tauri::Window, event: &WindowEvent) {
    if window.label() == "main" {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = window.hide();
        }
        return;
    }

    if window.label() == "overlay" {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = overlay_window::hide_overlay_window(&window.app_handle());
            return;
        }

        if let WindowEvent::Focused(false) = event {
            overlay_window::handle_overlay_focus_lost(window.app_handle().clone());
        }
    }
}
