use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use chrono::Local;
use tauri::{AppHandle, Manager};

const LOG_DIR: &str = "logs";
const LOG_FILE: &str = "app.log";

pub fn ensure_logs_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法获取应用数据目录: {error}"))?;
    fs::create_dir_all(&app_dir).map_err(|error| format!("无法创建应用数据目录: {error}"))?;

    let logs_dir = app_dir.join(LOG_DIR);
    fs::create_dir_all(&logs_dir).map_err(|error| format!("无法创建日志目录: {error}"))?;
    Ok(logs_dir)
}

pub fn info(app: &AppHandle, target: &str, message: &str) {
    if let Err(error) = append_line(app, "INFO", target, message) {
        eprintln!("[logger] info write failed: {error}");
    }
}

pub fn warn(app: &AppHandle, target: &str, message: &str) {
    if let Err(error) = append_line(app, "WARN", target, message) {
        eprintln!("[logger] warn write failed: {error}");
    }
}

pub fn error(app: &AppHandle, target: &str, message: &str) {
    if let Err(error) = append_line(app, "ERROR", target, message) {
        eprintln!("[logger] error write failed: {error}");
    }
}

fn append_line(app: &AppHandle, level: &str, target: &str, message: &str) -> Result<(), String> {
    let file = ensure_logs_dir(app)?.join(LOG_FILE);
    let mut writer = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file)
        .map_err(|error| format!("无法打开日志文件 {}: {error}", file.display()))?;

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    writeln!(writer, "[{timestamp}] [{level}] [{target}] {message}")
        .map_err(|error| format!("无法写入日志文件 {}: {error}", file.display()))
}
