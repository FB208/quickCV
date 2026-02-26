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
use models::{ReleaseCheckResult, Settings, SyncResult, TemplateStore, WebDavSettings};
use rdev::{simulate, EventType, Key};
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::{Deserialize, Serialize};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Position, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};
use tauri_plugin_global_shortcut::{GlobalShortcutExt as _, ShortcutState};
use tauri_plugin_opener::OpenerExt as _;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, POINT};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::ClientToScreen;
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId,
    SetForegroundWindow,
    GUITHREADINFO,
};

const CLIPBOARD_RESTORE_DELAY_MS: u64 = 140;
const OVERLAY_SAFE_MARGIN: i32 = 6;
const RELEASE_API_URL: &str = "https://api.github.com/repos/FB208/quickCV/releases/latest";
const RELEASE_PAGE_URL: &str = "https://github.com/FB208/quickCV/releases";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OverlayContext {
    query: String,
}

#[derive(Debug, Deserialize)]
struct GithubLatestRelease {
    tag_name: String,
    html_url: String,
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
    overlay_dragging: AtomicBool,
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
async fn check_release_version(app: AppHandle) -> Result<ReleaseCheckResult, String> {
    let current_version = app.package_info().version.to_string();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|error| format!("创建版本检查客户端失败: {error}"))?;

    let response = client
        .get(RELEASE_API_URL)
        .header(USER_AGENT, "quickcv-desktop")
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .await;

    let response = match response {
        Ok(value) => value,
        Err(error) => {
            return Ok(ReleaseCheckResult {
                status: "error".to_string(),
                has_update: false,
                message: format!("检查更新失败: {error}"),
                current_version,
                latest_version: None,
                release_url: RELEASE_PAGE_URL.to_string(),
            });
        }
    };

    if !response.status().is_success() {
        return Ok(ReleaseCheckResult {
            status: "error".to_string(),
            has_update: false,
            message: format!("检查更新失败，状态码: {}", response.status()),
            current_version,
            latest_version: None,
            release_url: RELEASE_PAGE_URL.to_string(),
        });
    }

    let payload = response
        .json::<GithubLatestRelease>()
        .await
        .map_err(|error| format!("解析版本信息失败: {error}"))?;

    let latest = normalize_version(&payload.tag_name);
    let has_update = version_compare(&latest, &current_version) > 0;

    let message = if has_update {
        format!("发现新版本 {}（当前 {}）", latest, current_version)
    } else {
        format!("当前已是最新版本（{}）", current_version)
    };

    Ok(ReleaseCheckResult {
        status: "ok".to_string(),
        has_update,
        message,
        current_version,
        latest_version: Some(latest),
        release_url: payload.html_url,
    })
}

#[tauri::command]
fn open_release_page(app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(RELEASE_PAGE_URL, None::<&str>)
        .map_err(|error| format!("打开发布页失败: {error}"))
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
fn set_overlay_dragging(app: AppHandle, dragging: bool) -> Result<(), String> {
    let runtime = app.state::<RuntimeState>();
    runtime.overlay_dragging.store(dragging, Ordering::SeqCst);
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

    // 先捕获前台窗口句柄，并用它获取输入光标位置，必须在 show() 之前完成
    let foreground_hwnd = capture_foreground_window_handle();

    if let Ok(mut previous) = runtime.previous_input_window.lock() {
        *previous = foreground_hwnd;
    }

    let anchor = caret_screen_position_from(foreground_hwnd)
        .or_else(uia_caret_screen_position)
        .or_else(|| imm_caret_screen_position(foreground_hwnd))
        .or_else(cursor_screen_position)
        .unwrap_or((OVERLAY_SAFE_MARGIN, OVERLAY_SAFE_MARGIN));

    runtime.overlay_dragging.store(false, Ordering::SeqCst);

    if let Some(window) = app.get_webview_window("overlay") {
        window
            .show()
            .map_err(|error| format!("显示浮窗失败: {error}"))?;

        let (clamped_x, clamped_y) = clamp_overlay_position(app, &window, anchor.0 + 8, anchor.1 + 14);
        let _ = window.set_position(Position::Physical(PhysicalPosition {
            x: clamped_x,
            y: clamped_y,
        }));

        window
            .set_focus()
            .map_err(|error| format!("聚焦浮窗失败: {error}"))?;
    }

    runtime.overlay_open.store(true, Ordering::SeqCst);
    app.emit_to("overlay", "overlay_context", context)
        .map_err(|error| format!("发送浮窗上下文失败: {error}"))?;

    Ok(())
}

fn clamp_overlay_position(
    app: &AppHandle,
    window: &tauri::WebviewWindow,
    desired_x: i32,
    desired_y: i32,
) -> (i32, i32) {
    let mut x = desired_x;
    let mut y = desired_y;

    let monitor = app
        .available_monitors()
        .ok()
        .and_then(|monitors| {
            monitors
                .into_iter()
                .find(|item| point_in_monitor(item, desired_x, desired_y))
        })
        .or_else(|| app.primary_monitor().ok().flatten())
        .or_else(|| window.current_monitor().ok().flatten());

    let Some(monitor) = monitor else {
        return (x, y);
    };

    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let window_size = window.outer_size().ok();

    let width = window_size.map(|size| size.width as i32).unwrap_or(688);
    let height = window_size.map(|size| size.height as i32).unwrap_or(448);

    let min_x = monitor_position.x + OVERLAY_SAFE_MARGIN;
    let min_y = monitor_position.y + OVERLAY_SAFE_MARGIN;
    let max_x = monitor_position.x + monitor_size.width as i32 - width - OVERLAY_SAFE_MARGIN;
    let max_y = monitor_position.y + monitor_size.height as i32 - height - OVERLAY_SAFE_MARGIN;

    let safe_max_x = max_x.max(min_x);
    let safe_max_y = max_y.max(min_y);
    x = x.clamp(min_x, safe_max_x);
    y = y.clamp(min_y, safe_max_y);

    (x, y)
}

fn point_in_monitor(monitor: &tauri::Monitor, x: i32, y: i32) -> bool {
    let position = monitor.position();
    let size = monitor.size();
    let right = position.x + size.width as i32;
    let bottom = position.y + size.height as i32;

    x >= position.x && x <= right && y >= position.y && y <= bottom
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

fn normalize_version(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .to_string()
}

fn version_compare(left: &str, right: &str) -> i32 {
    let left_parts: Vec<i32> = normalize_version(left)
        .split('.')
        .map(|part| part.parse::<i32>().unwrap_or(0))
        .collect();
    let right_parts: Vec<i32> = normalize_version(right)
        .split('.')
        .map(|part| part.parse::<i32>().unwrap_or(0))
        .collect();

    let max_len = left_parts.len().max(right_parts.len());
    for index in 0..max_len {
        let left_value = left_parts.get(index).copied().unwrap_or(0);
        let right_value = right_parts.get(index).copied().unwrap_or(0);
        if left_value > right_value {
            return 1;
        }
        if left_value < right_value {
            return -1;
        }
    }

    0
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
fn caret_screen_position_from(hwnd_raw: Option<isize>) -> Option<(i32, i32)> {
    let hwnd = match hwnd_raw {
        Some(raw) => HWND(raw as *mut core::ffi::c_void),
        None => unsafe { GetForegroundWindow() },
    };
    if hwnd.0.is_null() {
        return None;
    }

    let thread_id = unsafe { GetWindowThreadProcessId(hwnd, None) };
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
fn caret_screen_position_from(_hwnd_raw: Option<isize>) -> Option<(i32, i32)> {
    None
}

#[cfg(target_os = "windows")]
fn uia_caret_screen_position() -> Option<(i32, i32)> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize,
        CLSCTX_ALL, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Accessibility::*;

    unsafe {
        let com_result = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let should_uninit = com_result.is_ok();
        eprintln!("[UIA] CoInitializeEx: ok={}, is_err={}", should_uninit, com_result.is_err());

        let result = (|| -> Option<(i32, i32)> {
            let uia: IUIAutomation = match CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL) {
                Ok(v) => { eprintln!("[UIA] CUIAutomation created"); v }
                Err(e) => { eprintln!("[UIA] CUIAutomation FAILED: {e}"); return None; }
            };
            let focused = match uia.GetFocusedElement() {
                Ok(v) => { eprintln!("[UIA] GetFocusedElement OK"); v }
                Err(e) => { eprintln!("[UIA] GetFocusedElement FAILED: {e}"); return None; }
            };

            // TextPattern2::GetCaretRange
            match focused.GetCurrentPatternAs::<IUIAutomationTextPattern2>(UIA_TextPattern2Id) {
                Ok(tp2) => {
                    eprintln!("[UIA] TextPattern2 supported");
                    let mut active = windows::core::BOOL::default();
                    match tp2.GetCaretRange(&mut active) {
                        Ok(range) => {
                            eprintln!("[UIA] GetCaretRange OK, active={}", active.as_bool());
                            match extract_range_position(&range) {
                                Some(pos) => {
                                    eprintln!("[UIA] CaretRange bounding: ({}, {})", pos.0, pos.1);
                                    return Some(pos);
                                }
                                None => eprintln!("[UIA] CaretRange bounding: EMPTY"),
                            }
                        }
                        Err(e) => eprintln!("[UIA] GetCaretRange FAILED: {e}"),
                    }
                }
                Err(e) => eprintln!("[UIA] TextPattern2 not supported: {e}"),
            }

            // TextPattern::GetSelection
            match focused.GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId) {
                Ok(tp) => {
                    eprintln!("[UIA] TextPattern supported");
                    match tp.GetSelection() {
                        Ok(ranges) => {
                            let len = ranges.Length().unwrap_or(0);
                            eprintln!("[UIA] GetSelection ranges: {len}");
                            if len > 0 {
                                if let Ok(range) = ranges.GetElement(0) {
                                    match extract_range_position(&range) {
                                        Some(pos) => {
                                            eprintln!("[UIA] Selection bounding: ({}, {})", pos.0, pos.1);
                                            return Some(pos);
                                        }
                                        None => eprintln!("[UIA] Selection bounding: EMPTY"),
                                    }
                                }
                            }
                        }
                        Err(e) => eprintln!("[UIA] GetSelection FAILED: {e}"),
                    }
                }
                Err(e) => eprintln!("[UIA] TextPattern not supported: {e}"),
            }

            eprintln!("[UIA] all methods failed");
            None
        })();

        if should_uninit {
            CoUninitialize();
        }

        result
    }
}

#[cfg(target_os = "windows")]
unsafe fn extract_range_position(
    range: &windows::Win32::UI::Accessibility::IUIAutomationTextRange,
) -> Option<(i32, i32)> {
    use windows::Win32::UI::Accessibility::*;

    // 将 range 折叠到末端（光标通常在选区末尾），再展开一个字符取精确位置
    if let Ok(collapsed) = range.Clone() {
        let _ = collapsed.MoveEndpointByRange(
            TextPatternRangeEndpoint_Start,
            range,
            TextPatternRangeEndpoint_End,
        );
        let _ = collapsed.ExpandToEnclosingUnit(TextUnit_Character);

        if let Some(pos) = read_range_rect(&collapsed) {
            eprintln!("[UIA] collapsed rect pos: ({}, {})", pos.0, pos.1);
            return Some(pos);
        }
    }

    // 降级：直接用原始 range
    read_range_rect(range)
}

#[cfg(target_os = "windows")]
unsafe fn read_range_rect(
    range: &windows::Win32::UI::Accessibility::IUIAutomationTextRange,
) -> Option<(i32, i32)> {
    use windows::Win32::System::Ole::{
        SafeArrayAccessData, SafeArrayDestroy, SafeArrayGetLBound,
        SafeArrayGetUBound, SafeArrayUnaccessData,
    };

    let sa = range.GetBoundingRectangles().ok()?;
    if sa.is_null() {
        return None;
    }

    let pos = (|| -> Option<(i32, i32)> {
        let lb = SafeArrayGetLBound(sa, 1).ok()?;
        let ub = SafeArrayGetUBound(sa, 1).ok()?;
        let count = (ub - lb + 1) as usize;
        if count < 4 {
            return None;
        }

        let mut pdata: *mut std::ffi::c_void = std::ptr::null_mut();
        SafeArrayAccessData(sa, &mut pdata).ok()?;
        let data = std::slice::from_raw_parts(pdata as *const f64, count);
        eprintln!("[UIA] raw rect: x={}, y={}, w={}, h={}", data[0], data[1], data[2], data[3]);
        let x = data[0] as i32;
        let y = (data[1] + data[3]) as i32;
        let _ = SafeArrayUnaccessData(sa);
        Some((x, y))
    })();

    let _ = SafeArrayDestroy(sa);
    pos
}

#[cfg(not(target_os = "windows"))]
fn uia_caret_screen_position() -> Option<(i32, i32)> {
    None
}

#[cfg(target_os = "windows")]
fn imm_caret_screen_position(hwnd_raw: Option<isize>) -> Option<(i32, i32)> {
    use windows::Win32::UI::Input::Ime::{
        ImmGetCompositionWindow, ImmGetContext, ImmReleaseContext, COMPOSITIONFORM,
    };

    let raw = hwnd_raw?;
    let hwnd = HWND(raw as *mut core::ffi::c_void);

    unsafe {
        let himc = ImmGetContext(hwnd);
        if himc.0.is_null() {
            return None;
        }

        let mut form: COMPOSITIONFORM = std::mem::zeroed();
        let result = if ImmGetCompositionWindow(himc, &mut form).as_bool() {
            let mut pt = POINT {
                x: form.ptCurrentPos.x,
                y: form.ptCurrentPos.y,
            };
            if ClientToScreen(hwnd, &mut pt).as_bool() {
                Some((pt.x, pt.y))
            } else {
                None
            }
        } else {
            None
        };

        ImmReleaseContext(hwnd, himc);
        result
    }
}

#[cfg(not(target_os = "windows"))]
fn imm_caret_screen_position(_hwnd_raw: Option<isize>) -> Option<(i32, i32)> {
    None
}

#[cfg(target_os = "windows")]
fn cursor_screen_position() -> Option<(i32, i32)> {
    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point) }.is_ok() {
        Some((point.x, point.y))
    } else {
        None
    }
}

#[cfg(not(target_os = "windows"))]
fn cursor_screen_position() -> Option<(i32, i32)> {
    None
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
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
                    // 拖动操作会短暂触发失焦，延迟检查避免误关闭
                    let app_handle = window.app_handle().clone();
                    thread::spawn(move || {
                        thread::sleep(Duration::from_millis(150));
                        let runtime = app_handle.state::<RuntimeState>();
                        if runtime.overlay_dragging.load(Ordering::SeqCst) {
                            return;
                        }
                        if let Some(overlay) = app_handle.get_webview_window("overlay") {
                            if !overlay.is_focused().unwrap_or(true) {
                                let _ = hide_overlay_window(&app_handle);
                            }
                        }
                    });
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
            check_release_version,
            open_release_page,
            open_overlay,
            close_overlay,
            set_overlay_dragging,
            get_overlay_context,
            insert_template
        ])
        .run(tauri::generate_context!())
        .expect("error while running quickCV");
}
