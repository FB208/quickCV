use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Position};

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, POINT};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetForegroundWindow};

const OVERLAY_SAFE_MARGIN: i32 = 6;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayContext {
    pub query: String,
}

impl Default for OverlayContext {
    fn default() -> Self {
        Self {
            query: String::new(),
        }
    }
}

#[derive(Default)]
pub struct RuntimeState {
    overlay_open: AtomicBool,
    overlay_dragging: AtomicBool,
    overlay_inserting: AtomicBool,
    overlay_context: Mutex<OverlayContext>,
    previous_input_window: Mutex<Option<isize>>,
    paste_attempts: Mutex<u64>,
    paste_failures: Mutex<u64>,
}

pub fn open_overlay_with_context(app: &AppHandle, context: OverlayContext) -> Result<(), String> {
    let runtime = app.state::<RuntimeState>();

    if let Ok(mut saved) = runtime.overlay_context.lock() {
        *saved = context.clone();
    }

    let foreground_hwnd = capture_foreground_window_handle();
    if let Ok(mut previous) = runtime.previous_input_window.lock() {
        *previous = foreground_hwnd;
    }

    let anchor = cursor_screen_position().unwrap_or((OVERLAY_SAFE_MARGIN, OVERLAY_SAFE_MARGIN));
    runtime.overlay_dragging.store(false, Ordering::SeqCst);

    if let Some(window) = app.get_webview_window("overlay") {
        let (clamped_x, clamped_y) =
            clamp_overlay_position(app, &window, anchor.0 + 8, anchor.1 + 14);
        let _ = window.set_position(Position::Physical(PhysicalPosition {
            x: clamped_x,
            y: clamped_y,
        }));

        window
            .show()
            .map_err(|error| format!("显示浮窗失败: {error}"))?;

        #[cfg(target_os = "windows")]
        if let Ok(hwnd_raw) = window.hwnd() {
            set_window_no_activate(hwnd_raw.0 as isize, true);
            apply_noactivate_to_children(hwnd_raw.0 as isize);
        }

        reinforce_overlay_noactivate(app);
    }

    runtime.overlay_open.store(true, Ordering::SeqCst);
    app.emit_to("overlay", "overlay_context", context)
        .map_err(|error| format!("发送浮窗上下文失败: {error}"))?;

    Ok(())
}

pub fn hide_overlay_window(app: &AppHandle) -> tauri::Result<()> {
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

pub fn set_overlay_dragging(app: &AppHandle, dragging: bool) {
    let runtime = app.state::<RuntimeState>();
    runtime.overlay_dragging.store(dragging, Ordering::SeqCst);
}

pub fn get_overlay_context(app: &AppHandle) -> Result<OverlayContext, String> {
    let runtime = app.state::<RuntimeState>();
    runtime
        .overlay_context
        .lock()
        .map(|item| item.clone())
        .map_err(|_| "读取浮窗上下文失败".to_string())
}

pub fn previous_input_window(app: &AppHandle) -> Option<isize> {
    let runtime = app.state::<RuntimeState>();
    runtime
        .previous_input_window
        .lock()
        .ok()
        .and_then(|item| *item)
}

pub fn clear_previous_input_window(app: &AppHandle) {
    let runtime = app.state::<RuntimeState>();
    if let Ok(mut previous) = runtime.previous_input_window.lock() {
        *previous = None;
    };
}

pub fn set_overlay_inserting(app: &AppHandle, inserting: bool) {
    let runtime = app.state::<RuntimeState>();
    runtime.overlay_inserting.store(inserting, Ordering::SeqCst);
}

pub fn record_paste_result(app: &AppHandle, success: bool) -> (u64, u64) {
    let runtime = app.state::<RuntimeState>();
    let mut attempts = runtime.paste_attempts.lock().ok();
    let mut failures = runtime.paste_failures.lock().ok();

    if let Some(total) = attempts.as_mut() {
        **total += 1;
    }
    if !success {
        if let Some(failed) = failures.as_mut() {
            **failed += 1;
        }
    }

    let total_attempts = attempts.map(|value| *value).unwrap_or(0);
    let total_failures = failures.map(|value| *value).unwrap_or(0);
    (total_attempts, total_failures)
}

pub fn restore_input_window_focus(handle: Option<isize>) -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow};

        if let Some(raw) = handle {
            let target = HWND(raw as *mut core::ffi::c_void);
            let result = unsafe { SetForegroundWindow(target) };
            eprintln!("[INSERT] SetForegroundWindow result={:?}", result);

            let deadline = std::time::Instant::now() + Duration::from_millis(300);
            loop {
                thread::sleep(Duration::from_millis(8));
                let fg = unsafe { GetForegroundWindow() };
                if fg == target {
                    eprintln!("[INSERT] target window is now foreground");
                    return true;
                }
                if std::time::Instant::now() >= deadline {
                    eprintln!("[INSERT] WARNING: target window did NOT become foreground within 300ms, fg={:?}", fg.0);
                    return false;
                }
            }
        }

        eprintln!("[INSERT] restore_input_window_focus: no target handle");
        false
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = handle;
        true
    }
}

pub fn prepare_overlay_window_noactivate(app: &AppHandle) {
    if let Some(overlay) = app.get_webview_window("overlay") {
        #[cfg(target_os = "windows")]
        if let Ok(hwnd_raw) = overlay.hwnd() {
            set_window_no_activate(hwnd_raw.0 as isize, true);
        }
    }
}

pub fn handle_overlay_focus_lost(app_handle: AppHandle) {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(150));
        let runtime = app_handle.state::<RuntimeState>();
        if runtime.overlay_dragging.load(Ordering::SeqCst) {
            return;
        }
        if runtime.overlay_inserting.load(Ordering::SeqCst) {
            return;
        }
        if let Some(overlay) = app_handle.get_webview_window("overlay") {
            if !overlay.is_focused().unwrap_or(true) {
                let _ = hide_overlay_window(&app_handle);
            }
        }
    });
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
fn set_window_no_activate(hwnd_raw: isize, no_activate: bool) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, SWP_FRAMECHANGED,
        SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_EX_NOACTIVATE,
    };

    let hwnd = HWND(hwnd_raw as *mut core::ffi::c_void);
    unsafe {
        let mut style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        if no_activate {
            style |= WS_EX_NOACTIVATE.0 as isize;
        } else {
            style &= !(WS_EX_NOACTIVATE.0 as isize);
        }
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style);
        let _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn set_window_no_activate(_hwnd_raw: isize, _no_activate: bool) {}

fn reinforce_overlay_noactivate(app: &AppHandle) {
    #[cfg(target_os = "windows")]
    {
        let app_handle = app.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(120));
            if let Some(window) = app_handle.get_webview_window("overlay") {
                if let Ok(hwnd_raw) = window.hwnd() {
                    set_window_no_activate(hwnd_raw.0 as isize, true);
                    apply_noactivate_to_children(hwnd_raw.0 as isize);
                }
            }
        });
    }
}

#[cfg(target_os = "windows")]
fn apply_noactivate_to_children(parent_raw: isize) {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumChildWindows, GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
    };

    unsafe extern "system" fn enum_child_proc(hwnd: HWND, _lparam: LPARAM) -> BOOL {
        let mut ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        if (ex_style & WS_EX_NOACTIVATE.0 as isize) == 0 {
            ex_style |= WS_EX_NOACTIVATE.0 as isize;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style);
        }
        true.into()
    }

    unsafe {
        let hwnd = HWND(parent_raw as *mut core::ffi::c_void);
        let _ = EnumChildWindows(Some(hwnd), Some(enum_child_proc), LPARAM(0));
    }
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
