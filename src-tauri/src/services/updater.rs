use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::logger;
use crate::models::{AppUpdateCheckResult, AppUpdateProgressEvent, AppUpdateWelcome, Settings};
use crate::release;
use crate::storage;

pub const UPDATE_PROGRESS_EVENT: &str = "app_update_progress";

pub struct PendingUpdate(pub Mutex<Option<Update>>);

impl Default for PendingUpdate {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}

pub fn peek_update_welcome(app: &AppHandle) -> Result<Option<AppUpdateWelcome>, String> {
    let settings = storage::load_settings(app)?;
    let current_version = app.package_info().version.to_string();
    let previous_version = settings.last_seen_app_version.trim();

    if previous_version.is_empty() || previous_version == current_version {
        return Ok(None);
    }

    Ok(Some(AppUpdateWelcome {
        previous_version: previous_version.to_string(),
        current_version,
    }))
}

pub fn acknowledge_current_app_version(app: &AppHandle) -> Result<(), String> {
    let current_version = app.package_info().version.to_string();
    update_settings_metadata(app, |settings| {
        settings.last_seen_app_version = current_version.clone();
    })?;
    Ok(())
}

pub async fn check_app_update(
    app: &AppHandle,
    pending_update: &State<'_, PendingUpdate>,
) -> Result<AppUpdateCheckResult, String> {
    logger::info(app, "updater", "check_app_update command start");

    let current_version = app.package_info().version.to_string();
    let last_check_at = storage::now_ts();
    persist_last_update_check_at(app, last_check_at);

    let updater = match app
        .updater_builder()
        .timeout(Duration::from_secs(15))
        .build()
    {
        Ok(value) => value,
        Err(error) => {
            let message = format!("暂时无法检查更新，请稍后再试: {error}");
            logger::error(app, "updater", &message);
            clear_pending_update(pending_update)?;
            return Ok(error_result(current_version, last_check_at, message));
        }
    };

    let update = match updater.check().await {
        Ok(value) => value,
        Err(error) => {
            let message = format!("检查更新失败，请稍后再试: {error}");
            logger::error(app, "updater", &message);
            clear_pending_update(pending_update)?;
            return Ok(error_result(current_version, last_check_at, message));
        }
    };

    let mut guard = pending_update
        .0
        .lock()
        .map_err(|_| "无法获取更新状态，请重试".to_string())?;

    let result = if let Some(update) = update {
        let latest_version = release::normalize_version(&update.version);
        let release_notes = update
            .body
            .clone()
            .or_else(|| {
                update
                    .raw_json
                    .get("notes")
                    .and_then(|value| value.as_str())
                    .map(|value| value.to_string())
            })
            .unwrap_or_else(|| "本次更新暂未提供说明。".to_string());
        let published_at = update
            .raw_json
            .get("pub_date")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());

        logger::info(
            app,
            "updater",
            &format!("check_app_update found update {latest_version}"),
        );

        let result = AppUpdateCheckResult {
            status: "available".to_string(),
            has_update: true,
            message: format!("发现新版本 {latest_version}（当前 {current_version}）"),
            current_version: current_version.clone(),
            latest_version: Some(latest_version.clone()),
            release_url: release::release_tag_url(&latest_version),
            release_notes,
            published_at,
            last_check_at,
        };
        *guard = Some(update);
        result
    } else {
        logger::info(app, "updater", "check_app_update no update found");
        *guard = None;
        AppUpdateCheckResult {
            status: "latest".to_string(),
            has_update: false,
            message: format!("当前已是最新版本（{current_version}）"),
            current_version: current_version.clone(),
            latest_version: Some(current_version.clone()),
            release_url: release::RELEASE_PAGE_URL.to_string(),
            release_notes: String::new(),
            published_at: None,
            last_check_at,
        }
    };

    logger::info(app, "updater", "check_app_update command success");
    Ok(result)
}

pub async fn install_app_update(
    app: &AppHandle,
    pending_update: &State<'_, PendingUpdate>,
) -> Result<(), String> {
    let update = pending_update
        .0
        .lock()
        .map_err(|_| "无法获取更新状态，请重试".to_string())?
        .take()
        .ok_or_else(|| "当前没有待安装的更新，请先重新检查更新".to_string())?;

    let version = release::normalize_version(&update.version);
    logger::info(
        app,
        "updater",
        &format!("install_app_update start for {version}"),
    );

    emit_progress(
        app,
        AppUpdateProgressEvent {
            phase: "downloading".to_string(),
            version: version.clone(),
            downloaded_bytes: 0,
            total_bytes: None,
            message: format!("正在下载 v{version} 更新包..."),
        },
    );

    let progress_app = app.clone();
    let progress_version = version.clone();
    let install_app = app.clone();
    let install_version = version.clone();
    let mut downloaded_bytes = 0_u64;

    let result = update
        .download_and_install(
            move |chunk_length, content_length| {
                downloaded_bytes += chunk_length as u64;
                emit_progress(
                    &progress_app,
                    AppUpdateProgressEvent {
                        phase: "downloading".to_string(),
                        version: progress_version.clone(),
                        downloaded_bytes,
                        total_bytes: content_length,
                        message: format!("正在下载 v{} 更新包...", progress_version),
                    },
                );
            },
            move || {
                emit_progress(
                    &install_app,
                    AppUpdateProgressEvent {
                        phase: "installing".to_string(),
                        version: install_version.clone(),
                        downloaded_bytes: 0,
                        total_bytes: None,
                        message: format!("下载完成，正在安装 v{}...", install_version),
                    },
                );
            },
        )
        .await;

    if let Err(error) = result {
        let message = format!("下载安装更新失败: {error}");
        logger::error(app, "updater", &message);
        emit_progress(
            app,
            AppUpdateProgressEvent {
                phase: "error".to_string(),
                version: version.clone(),
                downloaded_bytes: 0,
                total_bytes: None,
                message: message.clone(),
            },
        );
        return Err(message);
    }

    logger::info(
        app,
        "updater",
        &format!("install_app_update success for {version}"),
    );

    #[cfg(not(target_os = "windows"))]
    {
        emit_progress(
            app,
            AppUpdateProgressEvent {
                phase: "finished".to_string(),
                version: version.clone(),
                downloaded_bytes: 0,
                total_bytes: None,
                message: format!("v{version} 已安装，正在重启应用..."),
            },
        );
        app.restart();
    }

    Ok(())
}

fn persist_last_update_check_at(app: &AppHandle, last_check_at: i64) {
    if let Err(error) = update_settings_metadata(app, |settings| {
        settings.last_update_check_at = last_check_at;
    }) {
        logger::warn(app, "updater", &format!("记录上次检查时间失败: {error}"));
    }
}

fn update_settings_metadata<F>(app: &AppHandle, update: F) -> Result<(), String>
where
    F: FnOnce(&mut Settings),
{
    let mut settings = storage::load_settings(app)?;
    update(&mut settings);
    storage::save_settings(app, &settings)
}

fn clear_pending_update(pending_update: &State<'_, PendingUpdate>) -> Result<(), String> {
    let mut guard = pending_update
        .0
        .lock()
        .map_err(|_| "无法获取更新状态，请重试".to_string())?;
    *guard = None;
    Ok(())
}

fn error_result(
    current_version: String,
    last_check_at: i64,
    message: String,
) -> AppUpdateCheckResult {
    AppUpdateCheckResult {
        status: "error".to_string(),
        has_update: false,
        message,
        current_version: current_version.clone(),
        latest_version: Some(current_version),
        release_url: release::RELEASE_PAGE_URL.to_string(),
        release_notes: String::new(),
        published_at: None,
        last_check_at,
    }
}

fn emit_progress(app: &AppHandle, payload: AppUpdateProgressEvent) {
    let _ = app.emit_to("main", UPDATE_PROGRESS_EVENT, payload);
}
