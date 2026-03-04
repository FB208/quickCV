use std::thread;
use std::time::Duration;

use arboard::Clipboard;
use tauri::AppHandle;

use crate::logger;
use crate::overlay_window;
use crate::paste;
use crate::storage;

pub fn copy_template(app: &AppHandle, template_id: String) -> Result<(), String> {
    eprintln!("[COPY] === copy_template START, id={template_id}");
    let store = storage::load_template_store(app)?;
    let template = store
        .templates
        .iter()
        .find(|item| item.id == template_id && item.deleted_at.is_none())
        .cloned()
        .ok_or_else(|| "模板不存在或已删除".to_string())?;

    let mut clipboard = Clipboard::new().map_err(|e| format!("访问剪贴板失败: {e}"))?;
    clipboard
        .set_text(template.content)
        .map_err(|e| format!("写入剪贴板失败: {e}"))?;

    eprintln!("[COPY] === copy_template SUCCESS");
    Ok(())
}

pub fn insert_template(app: &AppHandle, template_id: String) -> Result<(), String> {
    eprintln!("[INSERT] === insert_template START, id={template_id}");
    logger::info(
        app,
        "insert",
        &format!("insert_template start, id={template_id}"),
    );
    let store = storage::load_template_store(app)?;
    let template = store
        .templates
        .iter()
        .find(|item| item.id == template_id && item.deleted_at.is_none())
        .cloned()
        .ok_or_else(|| {
            eprintln!("[INSERT] template not found or deleted");
            "模板不存在或已删除".to_string()
        })?;

    let target_window = overlay_window::previous_input_window(app);

    let mut clipboard = Clipboard::new().map_err(|e| format!("访问剪贴板失败: {e}"))?;
    clipboard
        .set_text(template.content)
        .map_err(|e| format!("写入剪贴板失败: {e}"))?;

    overlay_window::set_overlay_inserting(app, true);
    let _ = overlay_window::hide_overlay_window(app);
    overlay_window::set_overlay_inserting(app, false);

    let focused = overlay_window::restore_input_window_focus(target_window);
    if !focused {
        thread::sleep(Duration::from_millis(80));
    }

    eprintln!("[INSERT] sending Ctrl+V...");
    let result = paste::send_paste_shortcut_with_retry(3, Duration::from_millis(70));

    match &result {
        Ok(()) => {
            eprintln!("[INSERT] === insert_template SUCCESS");
            let (attempts, failures) = overlay_window::record_paste_result(app, true);
            logger::info(
                app,
                "insert",
                &format!(
                    "insert_template success (total attempts: {attempts}, failures: {failures})"
                ),
            );
        }
        Err(error) => {
            let (attempts, failures) = overlay_window::record_paste_result(app, false);
            let msg = format!(
                "insert_template failed: {error} (total attempts: {attempts}, failures: {failures})"
            );
            if failures >= 5 {
                logger::error(app, "insert", &msg);
            } else {
                logger::warn(app, "insert", &msg);
            }
            eprintln!("[INSERT] === insert_template FAILED: {error}");
        }
    }

    overlay_window::clear_previous_input_window(app);
    result
}
