use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::logger;
use crate::models::{Folder, Settings, TemplateItem, TemplateStore};

const SETTINGS_FILE: &str = "settings.json";
const STORE_FILE: &str = "templates.json";
pub const SORT_GAP: i64 = 1024;

pub fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

pub fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法获取应用数据目录: {error}"))?;

    fs::create_dir_all(&dir).map_err(|error| format!("无法创建应用数据目录: {error}"))?;
    Ok(dir)
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join(SETTINGS_FILE))
}

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join(STORE_FILE))
}

pub fn ensure_device_id(settings: &mut Settings) {
    if settings.device_id.trim().is_empty() {
        settings.device_id = Uuid::new_v4().to_string();
    }
}

pub fn load_settings(app: &AppHandle) -> Result<Settings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        let mut settings = Settings::default();
        ensure_device_id(&mut settings);
        save_settings(app, &settings)?;
        logger::info(
            app,
            "storage",
            "load_settings created default settings file",
        );
        return Ok(settings);
    }

    let mut settings: Settings = read_json(&path).map_err(|error| {
        logger::error(app, "storage", &format!("读取 settings.json 失败: {error}"));
        error
    })?;
    ensure_device_id(&mut settings);
    if settings.webdav.remote_file.trim().is_empty() {
        settings.webdav.remote_file = "quickcv-data.json".to_string();
    }
    save_settings(app, &settings)?;
    Ok(settings)
}

pub fn save_settings(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let mut value = settings.clone();
    ensure_device_id(&mut value);
    if value.webdav.remote_file.trim().is_empty() {
        value.webdav.remote_file = "quickcv-data.json".to_string();
    }
    let path = settings_path(app)?;
    write_json(path, &value).map_err(|error| {
        logger::error(app, "storage", &format!("写入 settings.json 失败: {error}"));
        error
    })?;
    Ok(())
}

pub fn load_template_store(app: &AppHandle) -> Result<TemplateStore, String> {
    let path = store_path(app)?;
    if !path.exists() {
        let store = TemplateStore::default();
        save_template_store_raw(app, &store)?;
        logger::info(
            app,
            "storage",
            "load_template_store created default templates file",
        );
        return Ok(store);
    }

    let mut store = read_json(&path).map_err(|error| {
        logger::error(
            app,
            "storage",
            &format!("读取 templates.json 失败: {error}"),
        );
        error
    })?;

    if normalize_store(&mut store) {
        save_template_store_raw(app, &store)?;
    }

    Ok(store)
}

pub fn save_template_store(
    app: &AppHandle,
    store: &TemplateStore,
) -> Result<TemplateStore, String> {
    let mut value = store.clone();
    normalize_store(&mut value);
    validate_template_keys(&value)?;
    value.dataset_version = now_ts();

    let path = store_path(app)?;
    write_json(path, &value).map_err(|error| {
        logger::error(
            app,
            "storage",
            &format!("写入 templates.json 失败: {error}"),
        );
        error
    })?;
    Ok(value)
}

pub fn save_template_store_raw(app: &AppHandle, store: &TemplateStore) -> Result<(), String> {
    let mut value = store.clone();
    normalize_store(&mut value);
    let path = store_path(app)?;
    write_json(path, &value).map_err(|error| {
        logger::error(
            app,
            "storage",
            &format!("写入 templates.json(raw) 失败: {error}"),
        );
        error
    })?;
    Ok(())
}

pub fn normalize_store(store: &mut TemplateStore) -> bool {
    let folder_ids_before: Vec<String> = store.folders.iter().map(|item| item.id.clone()).collect();
    let template_ids_before: Vec<String> =
        store.templates.iter().map(|item| item.id.clone()).collect();

    let mut changed = false;
    changed |= normalize_folder_orders(store);
    changed |= normalize_template_orders(store);
    sort_store_entries(store);

    changed
        || folder_ids_before
            != store
                .folders
                .iter()
                .map(|item| item.id.clone())
                .collect::<Vec<_>>()
        || template_ids_before
            != store
                .templates
                .iter()
                .map(|item| item.id.clone())
                .collect::<Vec<_>>()
}

pub fn sort_store_entries(store: &mut TemplateStore) {
    store.folders.sort_by(compare_folders);

    let folder_positions: HashMap<String, usize> = store
        .folders
        .iter()
        .enumerate()
        .map(|(index, item)| (item.id.clone(), index))
        .collect();

    store.templates.sort_by(|left, right| {
        folder_positions
            .get(&left.folder_id)
            .copied()
            .unwrap_or(usize::MAX)
            .cmp(
                &folder_positions
                    .get(&right.folder_id)
                    .copied()
                    .unwrap_or(usize::MAX),
            )
            .then_with(|| left.folder_id.cmp(&right.folder_id))
            .then_with(|| compare_templates(left, right))
    });
}

pub fn compare_folders(left: &Folder, right: &Folder) -> Ordering {
    left.sort_order
        .cmp(&right.sort_order)
        .then_with(|| left.sort_updated_at.cmp(&right.sort_updated_at))
        .then_with(|| left.updated_at.cmp(&right.updated_at))
        .then_with(|| left.id.cmp(&right.id))
}

pub fn compare_templates(left: &TemplateItem, right: &TemplateItem) -> Ordering {
    left.sort_order
        .cmp(&right.sort_order)
        .then_with(|| left.sort_updated_at.cmp(&right.sort_updated_at))
        .then_with(|| left.updated_at.cmp(&right.updated_at))
        .then_with(|| left.id.cmp(&right.id))
}

fn normalize_folder_orders(store: &mut TemplateStore) -> bool {
    let baseline = store
        .dataset_version
        .max(
            store
                .folders
                .iter()
                .map(folder_sort_baseline)
                .max()
                .unwrap_or(0),
        )
        .max(1);
    let needs_reindex = store.folders.iter().any(|item| item.sort_order <= 0);
    let mut changed = false;

    if needs_reindex {
        for (index, item) in store.folders.iter_mut().enumerate() {
            let sort_order = ((index as i64) + 1) * SORT_GAP;
            if item.sort_order != sort_order || item.sort_updated_at != baseline {
                item.sort_order = sort_order;
                item.sort_updated_at = baseline;
                changed = true;
            }
        }
        return changed;
    }

    for item in &mut store.folders {
        if item.sort_updated_at <= 0 {
            item.sort_updated_at = baseline;
            changed = true;
        }
    }

    changed
}

fn normalize_template_orders(store: &mut TemplateStore) -> bool {
    let baseline = store
        .dataset_version
        .max(
            store
                .templates
                .iter()
                .map(template_sort_baseline)
                .max()
                .unwrap_or(0),
        )
        .max(1);
    let needs_reindex = store.templates.iter().any(|item| item.sort_order <= 0);
    let mut changed = false;

    if needs_reindex {
        let mut next_order_by_folder: HashMap<String, i64> = HashMap::new();
        for item in &mut store.templates {
            let next_order = next_order_by_folder
                .entry(item.folder_id.clone())
                .and_modify(|value| *value += SORT_GAP)
                .or_insert(SORT_GAP);
            if item.sort_order != *next_order || item.sort_updated_at != baseline {
                item.sort_order = *next_order;
                item.sort_updated_at = baseline;
                changed = true;
            }
        }
        return changed;
    }

    for item in &mut store.templates {
        if item.sort_updated_at <= 0 {
            item.sort_updated_at = baseline;
            changed = true;
        }
    }

    changed
}

fn folder_sort_baseline(folder: &Folder) -> i64 {
    folder
        .sort_updated_at
        .max(folder.updated_at)
        .max(folder.deleted_at.unwrap_or(0))
}

fn template_sort_baseline(template: &TemplateItem) -> i64 {
    template
        .sort_updated_at
        .max(template.updated_at)
        .max(template.deleted_at.unwrap_or(0))
}

pub fn validate_template_keys(store: &TemplateStore) -> Result<(), String> {
    let mut key_map: HashMap<String, String> = HashMap::new();

    for item in &store.templates {
        if item.deleted_at.is_some() {
            continue;
        }

        let Some(raw_key) = item.key.as_ref() else {
            continue;
        };

        let key = raw_key.trim();
        if key.is_empty() {
            continue;
        }

        if let Some(other_name) = key_map.get(key) {
            return Err(format!(
                "模板快捷标识重复：“{key}” 已被“{}”和“{}”使用",
                other_name, item.name
            ));
        }
        key_map.insert(key.to_string(), item.name.clone());
    }

    Ok(())
}

fn read_json<T>(path: &Path) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let text = fs::read_to_string(path)
        .map_err(|error| format!("读取文件失败 {}: {error}", path.display()))?;
    serde_json::from_str::<T>(&text)
        .map_err(|error| format!("解析文件失败 {}: {error}", path.display()))
}

fn write_json<T>(path: PathBuf, value: &T) -> Result<(), String>
where
    T: serde::Serialize,
{
    let text =
        serde_json::to_string_pretty(value).map_err(|error| format!("保存数据失败: {error}"))?;
    fs::write(&path, text).map_err(|error| format!("写入文件失败 {}: {error}", path.display()))
}
