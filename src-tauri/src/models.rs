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
pub struct Settings {
    pub shortcut: String,
    pub launch_at_startup: bool,
    pub launch_at_startup_effective: bool,
    pub webdav: WebDavSettings,
    pub last_synced_version: i64,
    pub device_id: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shortcut: "Ctrl+Shift+Space".to_string(),
            launch_at_startup: false,
            launch_at_startup_effective: false,
            webdav: WebDavSettings::default(),
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
    pub level: String,
    pub message: String,
    pub local_version: i64,
    pub remote_version: i64,
    pub conflict_copies: Vec<String>,
    pub key_conflicts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseCheckResult {
    pub status: String,
    pub has_update: bool,
    pub message: String,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub release_url: String,
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
