use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::models::{Settings, TemplateStore};

const SETTINGS_FILE: &str = "settings.json";
const STORE_FILE: &str = "templates.json";

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
        return Ok(settings);
    }

    let mut settings: Settings = read_json(&path)?;
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
    write_json(path, &value)
}

pub fn load_template_store(app: &AppHandle) -> Result<TemplateStore, String> {
    let path = store_path(app)?;
    if !path.exists() {
        let store = TemplateStore::default();
        save_template_store_raw(app, &store)?;
        return Ok(store);
    }

    read_json(&path)
}

pub fn save_template_store(
    app: &AppHandle,
    store: &TemplateStore,
) -> Result<TemplateStore, String> {
    validate_template_keys(store)?;

    let mut value = store.clone();
    value.dataset_version = now_ts();

    let path = store_path(app)?;
    write_json(path, &value)?;
    Ok(value)
}

pub fn save_template_store_raw(app: &AppHandle, store: &TemplateStore) -> Result<(), String> {
    let path = store_path(app)?;
    write_json(path, store)
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
                "模板 key 重复: {key}，涉及模板「{}」和「{}」",
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
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("序列化 JSON 失败: {error}"))?;
    fs::write(&path, text).map_err(|error| format!("写入文件失败 {}: {error}", path.display()))
}
