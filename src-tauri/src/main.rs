#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod storage;
mod sync;
mod webdav;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use arboard::{Clipboard, ImageData};
use models::{Settings, SyncResult, TemplateStore, UpdateCheckResult, WebDavSettings};
use rdev::{simulate, EventType, Key};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Position, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};
use tauri_plugin_global_shortcut::{GlobalShortcutExt as _, ShortcutState};
use tauri_plugin_updater::UpdaterExt as _;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, POINT};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::ClientToScreen;
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, SetForegroundWindow,
    GUITHREADINFO,
};

const CLIPBOARD_RESTORE_DELAY_MS: u64 = 140;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OverlayContext {
    query: String,
}

impl Default for OverlayContext {
    fn default() -> Self {
        Self {
            query: String::new(),
        }
    }
}

enum ClipboardBackup {
    Text(String),
    Html(String),
    Image(ImageData<'static>),
    FileList(Vec<std::path::PathBuf>),
    Empty,
}

#[derive(Default)]
struct RuntimeState {
    overlay_open: AtomicBool,
    overlay_context: Mutex<OverlayContext>,
    previous_input_window: Mutex<Option<isize>>,
}

#[tauri::command]
fn get_app_version(app: AppHandle) -> Result<String, String> {
    Ok(app.package_info().version.to_string())
}

#[tauri::command]
fn load_settings(app: AppHandle) -> Result<Settings, String> {
    let mut settings = storage::load_settings(&app)?;
    if let Ok(enabled) = app.autolaunch().is_enabled() {
        settings.launch_at_startup = enabled;
    }
    Ok(settings)
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: Settings) -> Result<Settings, String> {
    if settings.launch_at_startup {
        app.autolaunch()
            .enable()
            .map_err(|error| format!("启用开机启动失败: {error}"))?;
    } else {
        app.autolaunch()
            .disable()
            .map_err(|error| format!("关闭开机启动失败: {error}"))?;
    }

    storage::save_settings(&app, &settings)?;
    register_main_shortcut(&app, &settings.shortcut)?;
    storage::load_settings(&app)
}

#[tauri::command]
fn load_template_store(app: AppHandle) -> Result<TemplateStore, String> {
    storage::load_template_store(&app)
}

#[tauri::command]
fn save_template_store(app: AppHandle, store: TemplateStore) -> Result<TemplateStore, String> {
    storage::save_template_store(&app, &store)
}

#[tauri::command]
async fn test_webdav(webdav: WebDavSettings) -> Result<String, String> {
    webdav::test_connection(&webdav).await?;
    Ok("WebDAV 连通成功".to_string())
}

#[tauri::command]
async fn sync_pull(app: AppHandle) -> Result<SyncResult, String> {
    let mut settings = storage::load_settings(&app)?;
    if settings.webdav.url.trim().is_empty() {
        return Err("请先在设置中填写 WebDAV 地址".to_string());
    }

    let local_store = storage::load_template_store(&app)?;
    let remote_store = webdav::fetch_remote_store(&settings.webdav)
        .await?
        .unwrap_or_default();

    let now = storage::now_ts();
    let (mut merged, report) = sync::merge_stores(
        &local_store,
        &remote_store,
        settings.last_synced_version,
        &settings.device_id,
        now,
    );
    merged.dataset_version = now;

    storage::validate_template_keys(&merged)?;
    storage::save_template_store_raw(&app, &merged)?;

    settings.last_synced_version = remote_store.dataset_version;
    storage::save_settings(&app, &settings)?;

    Ok(SyncResult {
        blocked: false,
        message: "已从云端拉取并自动合并".to_string(),
        local_version: merged.dataset_version,
        remote_version: remote_store.dataset_version,
        conflict_copies: report.conflict_copies,
        key_conflicts: report.key_conflicts,
    })
}

#[tauri::command]
async fn sync_push(app: AppHandle) -> Result<SyncResult, String> {
    let mut settings = storage::load_settings(&app)?;
    if settings.webdav.url.trim().is_empty() {
        return Err("请先在设置中填写 WebDAV 地址".to_string());
    }

    let mut local_store = storage::load_template_store(&app)?;
    storage::validate_template_keys(&local_store)?;

    let remote_store = webdav::fetch_remote_store(&settings.webdav).await?;
    let remote_version = remote_store
        .as_ref()
        .map(|item| item.dataset_version)
        .unwrap_or(0);

    if remote_version > settings.last_synced_version {
        return Ok(SyncResult {
            blocked: true,
            message: format!(
                "云端版本 {} 新于本地同步版本 {}，请先拉取后再推送",
                remote_version, settings.last_synced_version
            ),
            local_version: local_store.dataset_version,
            remote_version,
            conflict_copies: Vec::new(),
            key_conflicts: Vec::new(),
        });
    }

    local_store.dataset_version = storage::now_ts();
    webdav::push_remote_store(&settings.webdav, &local_store).await?;

    storage::save_template_store_raw(&app, &local_store)?;
    settings.last_synced_version = local_store.dataset_version;
    storage::save_settings(&app, &settings)?;

    Ok(SyncResult {
        blocked: false,
        message: "已推送到云端".to_string(),
        local_version: local_store.dataset_version,
        remote_version: local_store.dataset_version,
        conflict_copies: Vec::new(),
        key_conflicts: Vec::new(),
    })
}

#[tauri::command]
async fn check_update(app: AppHandle) -> Result<UpdateCheckResult, String> {
    let settings = storage::load_settings(&app)?;
    let current_version = app.package_info().version.to_string();

    if settings.updater.endpoint.trim().is_empty() || settings.updater.pubkey.trim().is_empty() {
        return Ok(UpdateCheckResult {
            status: "not_configured".to_string(),
            message: "未配置更新地址或公钥，请在设置中补全后再检查更新。".to_string(),
            current_version,
            latest_version: None,
        });
    }

    let updater = build_configured_updater(&app, &settings)?;
    let result = match updater.check().await {
        Ok(value) => value,
        Err(error) => {
            return Ok(UpdateCheckResult {
                status: "error".to_string(),
                message: format!(
                    "检查更新失败：{}。请确认更新地址可访问、签名公钥正确。",
                    error
                ),
                current_version,
                latest_version: None,
            });
        }
    };

    if let Some(update) = result {
        return Ok(UpdateCheckResult {
            status: "available".to_string(),
            message: format!(
                "发现新版本 {}（当前 {}）",
                update.version, update.current_version
            ),
            current_version: update.current_version,
            latest_version: Some(update.version),
        });
    }

    Ok(UpdateCheckResult {
        status: "latest".to_string(),
        message: "当前已经是最新版本。".to_string(),
        current_version,
        latest_version: None,
    })
}

#[tauri::command]
async fn download_and_install_update(app: AppHandle) -> Result<String, String> {
    let settings = storage::load_settings(&app)?;
    let updater = build_configured_updater(&app, &settings)?;

    let Some(update) = updater
        .check()
        .await
        .map_err(|error| format!("检查更新失败: {error}"))?
    else {
        return Ok("当前已经是最新版本。".to_string());
    };

    let version = update.version.clone();
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|error| format!("下载或安装更新失败: {error}"))?;

    Ok(format!("版本 {version} 已开始安装，应用将自动退出。"))
}

#[tauri::command]
fn open_overlay(app: AppHandle, query: Option<String>) -> Result<(), String> {
    show_overlay_window_with_context(
        &app,
        OverlayContext {
            query: query.unwrap_or_default(),
        },
    )
}

#[tauri::command]
fn close_overlay(app: AppHandle) -> Result<(), String> {
    let runtime = app.state::<RuntimeState>();
    let target_window = runtime
        .previous_input_window
        .lock()
        .ok()
        .and_then(|item| *item);

    hide_overlay_window(&app).map_err(|error| format!("关闭浮窗失败: {error}"))?;
    restore_input_window_focus(target_window);
    Ok(())
}

#[tauri::command]
fn get_overlay_context(app: AppHandle) -> Result<OverlayContext, String> {
    let runtime = app.state::<RuntimeState>();
    runtime
        .overlay_context
        .lock()
        .map(|item| item.clone())
        .map_err(|_| "读取浮窗上下文失败".to_string())
}

fn build_configured_updater(
    app: &AppHandle,
    settings: &Settings,
) -> Result<tauri_plugin_updater::Updater, String> {
    let endpoint_text = settings.updater.endpoint.trim();
    let pubkey = settings.updater.pubkey.trim();

    if endpoint_text.is_empty() || pubkey.is_empty() {
        return Err("请先配置更新地址与签名公钥".to_string());
    }

    let endpoint =
        Url::parse(endpoint_text).map_err(|error| format!("更新地址格式错误: {error}"))?;

    app.updater_builder()
        .pubkey(pubkey.to_string())
        .endpoints(vec![endpoint])
        .map_err(|error| format!("更新地址配置失败: {error}"))?
        .build()
        .map_err(|error| format!("更新器初始化失败: {error}"))
}

#[tauri::command]
fn insert_template(app: AppHandle, template_id: String) -> Result<(), String> {
    let store = storage::load_template_store(&app)?;
    let template = store
        .templates
        .iter()
        .find(|item| item.id == template_id && item.deleted_at.is_none())
        .cloned()
        .ok_or_else(|| "模板不存在或已删除".to_string())?;

    let runtime = app.state::<RuntimeState>();
    let target_window = runtime
        .previous_input_window
        .lock()
        .ok()
        .and_then(|item| *item);

    let _ = hide_overlay_window(&app);
    inject_text_to_active_app(&app, &template.content, target_window)?;
    Ok(())
}

fn show_main_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}

fn show_overlay_window_with_context(
    app: &AppHandle,
    context: OverlayContext,
) -> Result<(), String> {
    let runtime = app.state::<RuntimeState>();

    if let Ok(mut saved) = runtime.overlay_context.lock() {
        *saved = context.clone();
    }

    if let Ok(mut previous) = runtime.previous_input_window.lock() {
        *previous = capture_foreground_window_handle();
    }

    if let Some(window) = app.get_webview_window("overlay") {
        if let Some((x, y)) = caret_screen_position() {
            let _ = window.set_position(Position::Physical(PhysicalPosition {
                x: x + 8,
                y: y + 14,
            }));
        }

        window
            .show()
            .map_err(|error| format!("显示浮窗失败: {error}"))?;
        window
            .set_focus()
            .map_err(|error| format!("聚焦浮窗失败: {error}"))?;
    }

    runtime.overlay_open.store(true, Ordering::SeqCst);
    app.emit_to("overlay", "overlay_context", context)
        .map_err(|error| format!("发送浮窗上下文失败: {error}"))?;

    Ok(())
}

fn hide_overlay_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("overlay") {
        window.hide()?;
    }

    let runtime = app.state::<RuntimeState>();
    runtime.overlay_open.store(false, Ordering::SeqCst);
    if let Ok(mut context) = runtime.overlay_context.lock() {
        *context = OverlayContext::default();
    }
    if let Ok(mut previous) = runtime.previous_input_window.lock() {
        *previous = None;
    }

    Ok(())
}

fn register_main_shortcut(app: &AppHandle, shortcut: &str) -> Result<(), String> {
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

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let open_item = MenuItem::with_id(app, "open", "设置", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_item, &quit_item])?;

    let mut tray_builder = TrayIconBuilder::with_id("quickcv-tray").menu(&menu);
    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }

    tray_builder
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => {
                let _ = show_main_window(app);
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = show_main_window(&tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn inject_text_to_active_app(
    app: &AppHandle,
    text: &str,
    target_window: Option<isize>,
) -> Result<(), String> {
    let runtime = app.state::<RuntimeState>();

    let focus_target = if target_window.is_some() {
        target_window
    } else {
        runtime
            .previous_input_window
            .lock()
            .ok()
            .and_then(|item| *item)
    };
    restore_input_window_focus(focus_target);
    thread::sleep(Duration::from_millis(55));

    paste_text(text)?;

    if let Ok(mut previous) = runtime.previous_input_window.lock() {
        *previous = None;
    }

    Ok(())
}

fn paste_text(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|error| format!("访问剪贴板失败: {error}"))?;
    let previous = capture_clipboard_backup(&mut clipboard);

    clipboard
        .set_text(text.to_string())
        .map_err(|error| format!("写入剪贴板失败: {error}"))?;

    send_paste_shortcut()?;

    thread::sleep(Duration::from_millis(CLIPBOARD_RESTORE_DELAY_MS));
    let should_restore = clipboard
        .get_text()
        .map(|current| current == text)
        .unwrap_or(false);

    if should_restore {
        let _ = restore_clipboard_backup(&mut clipboard, previous);
    }

    Ok(())
}

fn capture_clipboard_backup(clipboard: &mut Clipboard) -> ClipboardBackup {
    if let Ok(files) = clipboard.get().file_list() {
        if !files.is_empty() {
            return ClipboardBackup::FileList(files);
        }
    }

    if let Ok(image) = clipboard.get_image() {
        return ClipboardBackup::Image(image);
    }

    if let Ok(html) = clipboard.get().html() {
        if !html.is_empty() {
            return ClipboardBackup::Html(html);
        }
    }

    if let Ok(text) = clipboard.get_text() {
        return ClipboardBackup::Text(text);
    }

    ClipboardBackup::Empty
}

fn restore_clipboard_backup(
    clipboard: &mut Clipboard,
    backup: ClipboardBackup,
) -> Result<(), String> {
    match backup {
        ClipboardBackup::Text(text) => clipboard
            .set_text(text)
            .map_err(|error| format!("恢复文本剪贴板失败: {error}")),
        ClipboardBackup::Html(html) => clipboard
            .set_html(html, None::<String>)
            .map_err(|error| format!("恢复 HTML 剪贴板失败: {error}")),
        ClipboardBackup::Image(image) => clipboard
            .set_image(image)
            .map_err(|error| format!("恢复图片剪贴板失败: {error}")),
        ClipboardBackup::FileList(files) => clipboard
            .set()
            .file_list(&files)
            .map_err(|error| format!("恢复文件列表剪贴板失败: {error}")),
        ClipboardBackup::Empty => Ok(()),
    }
}

fn send_paste_shortcut() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let modifier = Key::MetaLeft;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::ControlLeft;

    simulate_event(EventType::KeyPress(modifier))?;
    simulate_event(EventType::KeyPress(Key::KeyV))?;
    simulate_event(EventType::KeyRelease(Key::KeyV))?;
    simulate_event(EventType::KeyRelease(modifier))
}

fn simulate_event(event: EventType) -> Result<(), String> {
    simulate(&event).map_err(|error| format!("模拟输入失败: {error}"))?;
    thread::sleep(Duration::from_millis(7));
    Ok(())
}

#[cfg(target_os = "windows")]
fn capture_foreground_window_handle() -> Option<isize> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        None
    } else {
        Some(hwnd.0 as isize)
    }
}

#[cfg(not(target_os = "windows"))]
fn capture_foreground_window_handle() -> Option<isize> {
    None
}

#[cfg(target_os = "windows")]
fn restore_input_window_focus(handle: Option<isize>) {
    if let Some(raw) = handle {
        let _ = unsafe { SetForegroundWindow(HWND(raw as *mut core::ffi::c_void)) };
    }
}

#[cfg(not(target_os = "windows"))]
fn restore_input_window_focus(_handle: Option<isize>) {}

#[cfg(target_os = "windows")]
fn caret_screen_position() -> Option<(i32, i32)> {
    let foreground = unsafe { GetForegroundWindow() };
    if foreground.0.is_null() {
        return None;
    }

    let thread_id = unsafe { GetWindowThreadProcessId(foreground, None) };
    if thread_id == 0 {
        return None;
    }

    let mut info = GUITHREADINFO::default();
    info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;

    if unsafe { GetGUIThreadInfo(thread_id, &mut info) }.is_err() {
        return None;
    }

    if info.hwndCaret.0.is_null() {
        return None;
    }

    let mut point = POINT {
        x: info.rcCaret.left,
        y: info.rcCaret.bottom,
    };

    if !unsafe { ClientToScreen(info.hwndCaret, &mut point).as_bool() } {
        return None;
    }

    Some((point.x, point.y))
}

#[cfg(not(target_os = "windows"))]
fn caret_screen_position() -> Option<(i32, i32)> {
    None
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let _ = show_overlay_window_with_context(app, OverlayContext::default());
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            app.manage(RuntimeState::default());
            setup_tray(app.handle())?;
            if let Ok(settings) = storage::load_settings(app.handle()) {
                let _ = register_main_shortcut(app.handle(), &settings.shortcut);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
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
                    let _ = hide_overlay_window(&window.app_handle());
                    return;
                }

                if let WindowEvent::Focused(false) = event {
                    let _ = hide_overlay_window(&window.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_app_version,
            load_settings,
            save_settings,
            load_template_store,
            save_template_store,
            test_webdav,
            sync_pull,
            sync_push,
            check_update,
            download_and_install_update,
            open_overlay,
            close_overlay,
            get_overlay_context,
            insert_template
        ])
        .run(tauri::generate_context!())
        .expect("error while running quickCV");
}
