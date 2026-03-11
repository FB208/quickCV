#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_bootstrap;
mod logger;
mod models;
mod overlay_window;
mod paste;
mod release;
mod services;
mod storage;
mod sync;
mod tray;
mod webdav;

use models::{
    AppUpdateCheckResult, AppUpdateWelcome, Settings, SyncResult, TemplateStore, WebDavSettings,
};
use tauri::{AppHandle, State};
use tauri_plugin_autostart::MacosLauncher;

use overlay_window::OverlayContext;

#[tauri::command]
fn get_app_version(app: AppHandle) -> Result<String, String> {
    Ok(app.package_info().version.to_string())
}

#[tauri::command]
fn load_settings(app: AppHandle) -> Result<Settings, String> {
    services::settings::load_settings(&app)
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: Settings) -> Result<Settings, String> {
    services::settings::save_settings(&app, settings)
}

#[tauri::command]
fn load_template_store(app: AppHandle) -> Result<TemplateStore, String> {
    services::store::load_template_store(&app)
}

#[tauri::command]
fn save_template_store(app: AppHandle, store: TemplateStore) -> Result<TemplateStore, String> {
    services::store::save_template_store(&app, store)
}

#[tauri::command]
async fn test_webdav(app: AppHandle, webdav: WebDavSettings) -> Result<String, String> {
    services::webdav::test_webdav(&app, webdav).await
}

#[tauri::command]
async fn sync_pull(app: AppHandle) -> Result<SyncResult, String> {
    services::sync::sync_pull(&app).await
}

#[tauri::command]
async fn sync_push(app: AppHandle) -> Result<SyncResult, String> {
    services::sync::sync_push(&app).await
}

#[tauri::command]
async fn check_app_update(
    app: AppHandle,
    pending_update: State<'_, services::updater::PendingUpdate>,
) -> Result<AppUpdateCheckResult, String> {
    services::updater::check_app_update(&app, &pending_update).await
}

#[tauri::command]
async fn install_app_update(
    app: AppHandle,
    pending_update: State<'_, services::updater::PendingUpdate>,
) -> Result<(), String> {
    services::updater::install_app_update(&app, &pending_update).await
}

#[tauri::command]
fn peek_app_update_welcome(app: AppHandle) -> Result<Option<AppUpdateWelcome>, String> {
    services::updater::peek_update_welcome(&app)
}

#[tauri::command]
fn acknowledge_current_app_version(app: AppHandle) -> Result<(), String> {
    services::updater::acknowledge_current_app_version(&app)
}

#[tauri::command]
fn open_release_page(app: AppHandle, version: Option<String>) -> Result<(), String> {
    services::system::open_release_page(&app, version.as_deref())
}

#[tauri::command]
fn open_config_folder(app: AppHandle) -> Result<(), String> {
    services::system::open_config_folder(&app)
}

#[tauri::command]
fn open_overlay(app: AppHandle, query: Option<String>) -> Result<(), String> {
    overlay_window::open_overlay_with_context(
        &app,
        OverlayContext {
            query: query.unwrap_or_default(),
        },
    )
}

#[tauri::command]
fn close_overlay(app: AppHandle) -> Result<(), String> {
    overlay_window::hide_overlay_window(&app).map_err(|error| format!("关闭浮窗失败: {error}"))?;
    Ok(())
}

#[tauri::command]
fn set_overlay_dragging(app: AppHandle, dragging: bool) -> Result<(), String> {
    overlay_window::set_overlay_dragging(&app, dragging);
    Ok(())
}

#[tauri::command]
fn get_overlay_context(app: AppHandle) -> Result<OverlayContext, String> {
    overlay_window::get_overlay_context(&app)
}

#[tauri::command]
fn copy_template(app: AppHandle, template_id: String) -> Result<(), String> {
    services::insert::copy_template(&app, template_id)
}

#[tauri::command]
fn insert_template(app: AppHandle, template_id: String) -> Result<(), String> {
    services::insert::insert_template(&app, template_id)
}

#[tauri::command]
fn open_main_templates(app: AppHandle) -> Result<(), String> {
    services::system::open_main_templates(&app)
}

fn main() {
    tauri::Builder::default()
        .manage(services::updater::PendingUpdate::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    app_bootstrap::handle_shortcut_event(app, event.state);
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(app_bootstrap::setup)
        .on_window_event(|window, event| app_bootstrap::handle_window_event(window, event))
        .invoke_handler(tauri::generate_handler![
            get_app_version,
            load_settings,
            save_settings,
            load_template_store,
            save_template_store,
            test_webdav,
            sync_pull,
            sync_push,
            check_app_update,
            install_app_update,
            peek_app_update_welcome,
            acknowledge_current_app_version,
            open_release_page,
            open_config_folder,
            open_main_templates,
            open_overlay,
            close_overlay,
            set_overlay_dragging,
            get_overlay_context,
            copy_template,
            insert_template
        ])
        .run(tauri::generate_context!())
        .expect("error while running quickCV");
}
