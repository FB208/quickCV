use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use chrono::Local;
use tauri::{AppHandle, Manager};

const LOG_DIR: &str = "logs";
const LOG_FILE: &str = "app.log";
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;
const MAX_LOG_BACKUPS: usize = 3;

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
    rotate_logs_if_needed(&file)?;
    let mut writer = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file)
        .map_err(|error| format!("无法打开日志文件 {}: {error}", file.display()))?;

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    writeln!(writer, "[{timestamp}] [{level}] [{target}] {message}")
        .map_err(|error| format!("无法写入日志文件 {}: {error}", file.display()))
}

fn rotate_logs_if_needed(file: &PathBuf) -> Result<(), String> {
    let metadata = match fs::metadata(file) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!("无法读取日志文件信息 {}: {error}", file.display()));
        }
    };

    if metadata.len() < MAX_LOG_BYTES {
        return Ok(());
    }

    for index in (1..=MAX_LOG_BACKUPS).rev() {
        let src = file.with_extension(format!("log.{index}"));
        let dst = file.with_extension(format!("log.{}", index + 1));

        if !src.exists() {
            continue;
        }

        if index == MAX_LOG_BACKUPS {
            fs::remove_file(&src)
                .map_err(|error| format!("无法清理旧日志文件 {}: {error}", src.display()))?;
            continue;
        }

        if dst.exists() {
            fs::remove_file(&dst)
                .map_err(|error| format!("无法覆盖旧日志文件 {}: {error}", dst.display()))?;
        }

        fs::rename(&src, &dst).map_err(|error| {
            format!(
                "无法轮转日志文件 {} -> {}: {error}",
                src.display(),
                dst.display()
            )
        })?;
    }

    let first_backup = file.with_extension("log.1");
    if first_backup.exists() {
        fs::remove_file(&first_backup).map_err(|error| {
            format!(
                "无法覆盖首个日志备份文件 {}: {error}",
                first_backup.display()
            )
        })?;
    }
    fs::rename(file, &first_backup).map_err(|error| {
        format!(
            "无法轮转当前日志文件 {} -> {}: {error}",
            file.display(),
            first_backup.display()
        )
    })?;

    Ok(())
}
