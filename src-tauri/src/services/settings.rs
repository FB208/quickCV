use crate::logger;
use crate::models::Settings;
use crate::storage;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_global_shortcut::GlobalShortcutExt as _;

pub fn load_settings(app: &AppHandle) -> Result<Settings, String> {
    logger::info(app, "settings", "load_settings command start");
    let mut settings = storage::load_settings(app)?;
    if let Ok(enabled) = app.autolaunch().is_enabled() {
        settings.launch_at_startup = enabled;
    }
    logger::info(app, "settings", "load_settings command success");
    Ok(settings)
}

pub fn save_settings(app: &AppHandle, settings: Settings) -> Result<Settings, String> {
    logger::info(app, "settings", "save_settings command start");
    let mut to_save = settings.clone();

    let current_autostart = app.autolaunch().is_enabled().ok();
    let should_update_autostart = current_autostart
        .map(|enabled| enabled != settings.launch_at_startup)
        .unwrap_or(true);

    if should_update_autostart {
        let apply = if settings.launch_at_startup {
            app.autolaunch().enable()
        } else {
            app.autolaunch().disable()
        };

        if let Err(error) = apply {
            let action = if settings.launch_at_startup {
                "启用"
            } else {
                "关闭"
            };
            logger::warn(
                app,
                "settings",
                &format!("{action}开机启动失败，已继续保存其它设置: {error}"),
            );

            if let Ok(enabled) = app.autolaunch().is_enabled() {
                to_save.launch_at_startup = enabled;
            }
        }
    }

    storage::save_settings(app, &to_save).map_err(|error| {
        logger::error(app, "settings", &format!("保存设置文件失败: {error}"));
        error
    })?;
    register_main_shortcut(app, &to_save.shortcut).map_err(|error| {
        logger::error(app, "settings", &format!("注册主快捷键失败: {error}"));
        error
    })?;

    let mut loaded = storage::load_settings(app).map_err(|error| {
        logger::error(app, "settings", &format!("回读设置文件失败: {error}"));
        error
    })?;

    if let Ok(enabled) = app.autolaunch().is_enabled() {
        loaded.launch_at_startup = enabled;
    }

    logger::info(app, "settings", "save_settings command success");
    Ok(loaded)
}

pub fn register_main_shortcut(app: &AppHandle, shortcut: &str) -> Result<(), String> {
    let manager = app.global_shortcut();
    manager
        .unregister_all()
        .map_err(|error| format!("清理旧快捷键失败: {error}"))?;

    let value = shortcut.trim();
    if value.is_empty() {
        return Ok(());
    }

    manager
        .register(value)
        .map_err(|error| format!("注册全局快捷键失败: {error}"))
}
