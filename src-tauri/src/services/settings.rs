use crate::logger;
use crate::models::Settings;
use crate::storage;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_global_shortcut::GlobalShortcutExt as _;

pub fn load_settings(app: &AppHandle) -> Result<Settings, String> {
    logger::info(app, "settings", "load_settings command start");
    let mut settings = storage::load_settings(app)?;
    settings.launch_at_startup_effective = reconcile_autostart(app, settings.launch_at_startup);
    logger::info(app, "settings", "load_settings command success");
    Ok(settings)
}

pub fn save_settings(app: &AppHandle, settings: Settings) -> Result<Settings, String> {
    logger::info(app, "settings", "save_settings command start");
    let mut to_save = settings;
    to_save.launch_at_startup_effective = false;

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

    loaded.launch_at_startup_effective = reconcile_autostart(app, loaded.launch_at_startup);
    if loaded.launch_at_startup_effective != loaded.launch_at_startup {
        logger::warn(
            app,
            "settings",
            "开机启动期望状态与系统实际状态不一致，请检查系统权限或安全软件",
        );
    }

    logger::info(app, "settings", "save_settings command success");
    Ok(loaded)
}

fn reconcile_autostart(app: &AppHandle, desired: bool) -> bool {
    let current = match app.autolaunch().is_enabled() {
        Ok(enabled) => enabled,
        Err(error) => {
            logger::warn(app, "settings", &format!("读取开机启动状态失败: {error}"));
            return false;
        }
    };

    if current == desired {
        return current;
    }

    let action = if desired { "启用" } else { "关闭" };
    let apply_result = if desired {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    };

    if let Err(error) = apply_result {
        logger::warn(
            app,
            "settings",
            &format!("{action}开机启动失败，可能被系统策略拦截: {error}"),
        );
    }

    match app.autolaunch().is_enabled() {
        Ok(enabled) => enabled,
        Err(error) => {
            logger::warn(app, "settings", &format!("回读开机启动状态失败: {error}"));
            current
        }
    }
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
