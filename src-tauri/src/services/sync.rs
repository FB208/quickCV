use crate::logger;
use crate::models::{Folder, SyncResult, TemplateItem, TemplateStore};
use crate::storage;
use crate::sync;
use crate::webdav;
use tauri::AppHandle;

pub async fn sync_pull(app: &AppHandle) -> Result<SyncResult, String> {
    logger::info(app, "sync", "sync_pull command start");
    let mut settings = storage::load_settings(app)?;
    if settings.webdav.url.trim().is_empty() {
        return Err("请先在设置中填写云同步地址".to_string());
    }

    let local_store = storage::load_template_store(app)?;
    let remote_store = webdav::fetch_remote_store(&settings.webdav)
        .await
        .map_err(|error| {
            logger::error(app, "sync", &format!("sync_pull 拉取云端数据失败: {error}"));
            error
        })?;

    let Some(remote_store) = remote_store else {
        let remote_file = settings.webdav.remote_file.trim();
        logger::warn(
            app,
            "sync",
            &format!(
                "sync_pull 远端文件不存在: {}",
                if remote_file.is_empty() {
                    "(空)"
                } else {
                    remote_file
                }
            ),
        );
        return Ok(SyncResult {
            blocked: false,
            level: "warn".to_string(),
            message: format!(
                "云端还没有可同步的内容，你可以先上传一次本地数据。当前文件：{}",
                if remote_file.is_empty() {
                    "quickcv-data.json"
                } else {
                    remote_file
                }
            ),
            local_version: local_store.dataset_version,
            remote_version: 0,
            conflict_copies: Vec::new(),
            key_conflicts: Vec::new(),
        });
    };

    let local_before = active_counts(&local_store);
    let remote_version = remote_store.dataset_version;

    let now = storage::now_ts();
    let (mut merged, report) = sync::merge_stores(
        &local_store,
        &remote_store,
        settings.last_synced_version,
        &settings.device_id,
        now,
    );
    storage::validate_template_keys(&merged)?;

    let merged_differs_from_remote = !stores_equal_content(&merged, &remote_store);
    if merged_differs_from_remote {
        merged.dataset_version = now;
        webdav::push_remote_store(&settings.webdav, &merged)
            .await
            .map_err(|error| {
                logger::error(app, "sync", &format!("sync_pull 回写云端失败: {error}"));
                error
            })?;
        settings.last_synced_version = merged.dataset_version;
    } else {
        merged.dataset_version = remote_version;
        settings.last_synced_version = remote_version;
    }

    storage::save_template_store_raw(app, &merged)?;
    storage::save_settings(app, &settings)?;

    let merged_after = active_counts(&merged);
    let changed = local_before != merged_after
        || !report.conflict_copies.is_empty()
        || !report.key_conflicts.is_empty();
    let message = if changed {
        let mut base = format!(
            "已完成云端同步（文件夹 {} -> {}，模板 {} -> {}）",
            local_before.0, merged_after.0, local_before.1, merged_after.1
        );
        if merged_differs_from_remote {
            base.push_str("，同步结果已更新到云端");
        }
        base.push_str(&format!("，当前数据版本 {}", merged.dataset_version));
        base
    } else {
        format!(
            "已完成同步，当前内容已是最新（数据版本 {}）",
            merged.dataset_version
        )
    };

    let result = SyncResult {
        blocked: false,
        level: "success".to_string(),
        message,
        local_version: merged.dataset_version,
        remote_version: merged.dataset_version,
        conflict_copies: report.conflict_copies,
        key_conflicts: report.key_conflicts,
    };
    logger::info(app, "sync", "sync_pull command success");
    Ok(result)
}

pub async fn sync_push(app: &AppHandle) -> Result<SyncResult, String> {
    logger::info(app, "sync", "sync_push command start");
    let mut settings = storage::load_settings(app)?;
    if settings.webdav.url.trim().is_empty() {
        return Err("请先在设置中填写云同步地址".to_string());
    }

    let mut local_store = storage::load_template_store(app)?;
    storage::validate_template_keys(&local_store)?;

    let remote_store = webdav::fetch_remote_store(&settings.webdav)
        .await
        .map_err(|error| {
            logger::error(app, "sync", &format!("sync_push 拉取云端数据失败: {error}"));
            error
        })?;
    let remote_version = remote_store
        .as_ref()
        .map(|item| item.dataset_version)
        .unwrap_or(0);

    if remote_version > settings.last_synced_version {
        let result = SyncResult {
            blocked: true,
            level: "warn".to_string(),
            message: format!(
                "云端内容已有更新，请先同步最新内容后再上传本地修改（云端版本 {}，本地同步版本 {}）",
                remote_version, settings.last_synced_version
            ),
            local_version: local_store.dataset_version,
            remote_version,
            conflict_copies: Vec::new(),
            key_conflicts: Vec::new(),
        };
        logger::warn(app, "sync", "sync_push blocked by remote newer version");
        return Ok(result);
    }

    local_store.dataset_version = storage::now_ts();
    webdav::push_remote_store(&settings.webdav, &local_store)
        .await
        .map_err(|error| {
            logger::error(app, "sync", &format!("sync_push 推送云端数据失败: {error}"));
            error
        })?;

    storage::save_template_store_raw(app, &local_store)?;
    settings.last_synced_version = local_store.dataset_version;
    storage::save_settings(app, &settings)?;

    let result = SyncResult {
        blocked: false,
        level: "success".to_string(),
        message: "已上传到云端".to_string(),
        local_version: local_store.dataset_version,
        remote_version: local_store.dataset_version,
        conflict_copies: Vec::new(),
        key_conflicts: Vec::new(),
    };
    logger::info(app, "sync", "sync_push command success");
    Ok(result)
}

fn active_counts(store: &TemplateStore) -> (usize, usize) {
    let folders = store
        .folders
        .iter()
        .filter(|item| item.deleted_at.is_none())
        .count();
    let templates = store
        .templates
        .iter()
        .filter(|item| item.deleted_at.is_none())
        .count();
    (folders, templates)
}

fn stores_equal_content(left: &TemplateStore, right: &TemplateStore) -> bool {
    let mut left_folders = left.folders.clone();
    let mut right_folders = right.folders.clone();
    left_folders.sort_by(|a, b| a.id.cmp(&b.id));
    right_folders.sort_by(|a, b| a.id.cmp(&b.id));

    if left_folders.len() != right_folders.len() {
        return false;
    }
    if left_folders
        .iter()
        .zip(right_folders.iter())
        .any(|(left_item, right_item)| !folder_equal(left_item, right_item))
    {
        return false;
    }

    let mut left_templates = left.templates.clone();
    let mut right_templates = right.templates.clone();
    left_templates.sort_by(|a, b| a.id.cmp(&b.id));
    right_templates.sort_by(|a, b| a.id.cmp(&b.id));

    if left_templates.len() != right_templates.len() {
        return false;
    }

    !left_templates
        .iter()
        .zip(right_templates.iter())
        .any(|(left_item, right_item)| !template_equal(left_item, right_item))
}

fn folder_equal(left: &Folder, right: &Folder) -> bool {
    left.id == right.id
        && left.name == right.name
        && left.sort_order == right.sort_order
        && left.sort_updated_at == right.sort_updated_at
        && left.updated_at == right.updated_at
        && left.deleted_at == right.deleted_at
        && left.device_id == right.device_id
}

fn template_equal(left: &TemplateItem, right: &TemplateItem) -> bool {
    left.id == right.id
        && left.folder_id == right.folder_id
        && left.name == right.name
        && left.key == right.key
        && left.content == right.content
        && left.sort_order == right.sort_order
        && left.sort_updated_at == right.sort_updated_at
        && left.updated_at == right.updated_at
        && left.deleted_at == right.deleted_at
        && left.device_id == right.device_id
}
