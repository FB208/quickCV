use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct WebDavSettings {
    pub url: String,
    pub username: String,
    pub password: String,
    pub remote_file: String,
}

impl Default for WebDavSettings {
    fn default() -> Self {
        Self {
            url: String::new(),
            username: String::new(),
            password: String::new(),
            remote_file: "quickcv-data.json".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct UpdaterSettings {
    pub endpoint: String,
    pub pubkey: String,
}

impl Default for UpdaterSettings {
    fn default() -> Self {
        Self {
            endpoint: option_env!("QUICKCV_UPDATE_ENDPOINT")
                .unwrap_or_default()
                .to_string(),
            pubkey: option_env!("QUICKCV_UPDATER_PUBKEY")
                .unwrap_or_default()
                .to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Settings {
    pub shortcut: String,
    pub launch_at_startup: bool,
    pub webdav: WebDavSettings,
    pub updater: UpdaterSettings,
    pub last_synced_version: i64,
    pub device_id: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shortcut: "Ctrl+Shift+Space".to_string(),
            launch_at_startup: false,
            webdav: WebDavSettings::default(),
            updater: UpdaterSettings::default(),
            last_synced_version: 0,
            device_id: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateItem {
    pub id: String,
    pub folder_id: String,
    pub name: String,
    pub key: Option<String>,
    pub content: String,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateStore {
    pub dataset_version: i64,
    pub folders: Vec<Folder>,
    pub templates: Vec<TemplateItem>,
}

impl Default for TemplateStore {
    fn default() -> Self {
        Self {
            dataset_version: 0,
            folders: Vec::new(),
            templates: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub blocked: bool,
    pub message: String,
    pub local_version: i64,
    pub remote_version: i64,
    pub conflict_copies: Vec<String>,
    pub key_conflicts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub status: String,
    pub message: String,
    pub current_version: String,
    pub latest_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MergeReport {
    pub conflict_copies: Vec<String>,
    pub key_conflicts: Vec<String>,
}

impl MergeReport {
    pub fn new() -> Self {
        Self {
            conflict_copies: Vec::new(),
            key_conflicts: Vec::new(),
        }
    }
}
