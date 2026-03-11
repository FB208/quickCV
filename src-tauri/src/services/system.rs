use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt as _;

use crate::release;
use crate::storage;
use crate::tray;

pub fn open_release_page(app: &AppHandle, version: Option<&str>) -> Result<(), String> {
    let target_url = version
        .map(release::release_tag_url)
        .unwrap_or_else(|| release::RELEASE_PAGE_URL.to_string());

    app.opener()
        .open_url(target_url, None::<&str>)
        .map_err(|error| format!("打开发布页失败: {error}"))
}

pub fn open_config_folder(app: &AppHandle) -> Result<(), String> {
    let dir = storage::app_data_dir(app)?;
    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|error| format!("打开配置文件夹失败: {error}"))
}

pub fn open_main_templates(app: &AppHandle) -> Result<(), String> {
    tray::show_main_window(app).map_err(|error| format!("打开设置页失败: {error}"))?;
    app.emit_to("main", "navigate_main_tab", "templates")
        .map_err(|error| format!("切换到模板管理失败: {error}"))
}
