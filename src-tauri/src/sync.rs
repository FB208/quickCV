use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Local, TimeZone};
use uuid::Uuid;

use crate::models::{Folder, MergeReport, TemplateItem, TemplateStore};

fn folder_op_time(folder: &Folder) -> i64 {
    folder.updated_at.max(folder.deleted_at.unwrap_or(0))
}

fn template_op_time(template: &TemplateItem) -> i64 {
    template.updated_at.max(template.deleted_at.unwrap_or(0))
}

fn template_equal(a: &TemplateItem, b: &TemplateItem) -> bool {
    a.folder_id == b.folder_id
        && a.name == b.name
        && a.key == b.key
        && a.content == b.content
        && a.deleted_at == b.deleted_at
}

fn conflict_copy_name(source: &TemplateItem, label: &str, timestamp: i64) -> String {
    let datetime: DateTime<Local> = Local
        .timestamp_millis_opt(timestamp)
        .single()
        .unwrap_or_else(Local::now);
    format!(
        "{} (冲突副本-{}-{})",
        source.name,
        label,
        datetime.format("%Y%m%d-%H%M%S")
    )
}

pub fn merge_stores(
    local: &TemplateStore,
    remote: &TemplateStore,
    last_synced_version: i64,
    current_device_id: &str,
    now_ts: i64,
) -> (TemplateStore, MergeReport) {
    let mut report = MergeReport::new();

    let local_folders: HashMap<String, Folder> = local
        .folders
        .iter()
        .map(|item| (item.id.clone(), item.clone()))
        .collect();
    let remote_folders: HashMap<String, Folder> = remote
        .folders
        .iter()
        .map(|item| (item.id.clone(), item.clone()))
        .collect();

    let mut folder_ids: HashSet<String> = local_folders.keys().cloned().collect();
    folder_ids.extend(remote_folders.keys().cloned());

    let mut merged_folders: Vec<Folder> = Vec::new();
    for id in folder_ids {
        match (local_folders.get(&id), remote_folders.get(&id)) {
            (Some(left), Some(right)) => {
                if folder_op_time(left) >= folder_op_time(right) {
                    merged_folders.push(left.clone());
                } else {
                    merged_folders.push(right.clone());
                }
            }
            (Some(left), None) => merged_folders.push(left.clone()),
            (None, Some(right)) => merged_folders.push(right.clone()),
            _ => {}
        }
    }

    let local_templates: HashMap<String, TemplateItem> = local
        .templates
        .iter()
        .map(|item| (item.id.clone(), item.clone()))
        .collect();
    let remote_templates: HashMap<String, TemplateItem> = remote
        .templates
        .iter()
        .map(|item| (item.id.clone(), item.clone()))
        .collect();

    let mut template_ids: HashSet<String> = local_templates.keys().cloned().collect();
    template_ids.extend(remote_templates.keys().cloned());

    let mut merged_templates: Vec<TemplateItem> = Vec::new();

    for id in template_ids {
        match (local_templates.get(&id), remote_templates.get(&id)) {
            (Some(left), Some(right)) => {
                let left_time = template_op_time(left);
                let right_time = template_op_time(right);
                let left_changed = left_time > last_synced_version;
                let right_changed = right_time > last_synced_version;

                if left_time >= right_time {
                    merged_templates.push(left.clone());
                    if left_changed
                        && right_changed
                        && !template_equal(left, right)
                        && right.deleted_at.is_none()
                    {
                        let mut copy = right.clone();
                        copy.id = Uuid::new_v4().to_string();
                        copy.key = None;
                        copy.updated_at = now_ts;
                        copy.deleted_at = None;
                        copy.device_id = current_device_id.to_string();
                        copy.name = conflict_copy_name(right, "remote", now_ts);
                        report.conflict_copies.push(copy.name.clone());
                        merged_templates.push(copy);
                    }
                } else {
                    merged_templates.push(right.clone());
                    if left_changed
                        && right_changed
                        && !template_equal(left, right)
                        && left.deleted_at.is_none()
                    {
                        let mut copy = left.clone();
                        copy.id = Uuid::new_v4().to_string();
                        copy.key = None;
                        copy.updated_at = now_ts;
                        copy.deleted_at = None;
                        copy.device_id = current_device_id.to_string();
                        copy.name = conflict_copy_name(left, "local", now_ts);
                        report.conflict_copies.push(copy.name.clone());
                        merged_templates.push(copy);
                    }
                }
            }
            (Some(left), None) => merged_templates.push(left.clone()),
            (None, Some(right)) => merged_templates.push(right.clone()),
            _ => {}
        }
    }

    resolve_key_conflicts(&mut merged_templates, &mut report);

    (
        TemplateStore {
            dataset_version: local.dataset_version.max(remote.dataset_version),
            folders: merged_folders,
            templates: merged_templates,
        },
        report,
    )
}

fn resolve_key_conflicts(templates: &mut [TemplateItem], report: &mut MergeReport) {
    let mut key_index: HashMap<String, usize> = HashMap::new();

    for index in 0..templates.len() {
        if templates[index].deleted_at.is_some() {
            continue;
        }

        let Some(raw_key) = templates[index].key.clone() else {
            continue;
        };

        let normalized = raw_key.trim();
        if normalized.is_empty() {
            templates[index].key = None;
            continue;
        }

        if let Some(previous_index) = key_index.get(normalized).copied() {
            let keep = if templates[index].updated_at >= templates[previous_index].updated_at {
                index
            } else {
                previous_index
            };
            let clear = if keep == index { previous_index } else { index };

            let lost_template_name = templates[clear].name.clone();
            templates[clear].key = None;
            report.key_conflicts.push(format!(
                "模板「{}」的 key '{}' 与其它模板重复，已自动清空较旧 key",
                lost_template_name, normalized
            ));

            key_index.insert(normalized.to_string(), keep);
        } else {
            key_index.insert(normalized.to_string(), index);
            templates[index].key = Some(normalized.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::merge_stores;
    use crate::models::{Folder, TemplateItem, TemplateStore};

    fn folder() -> Folder {
        Folder {
            id: "f1".to_string(),
            name: "默认".to_string(),
            updated_at: 1,
            deleted_at: None,
            device_id: "device-a".to_string(),
        }
    }

    fn template(
        id: &str,
        updated_at: i64,
        name: &str,
        key: Option<&str>,
        content: &str,
    ) -> TemplateItem {
        TemplateItem {
            id: id.to_string(),
            folder_id: "f1".to_string(),
            name: name.to_string(),
            key: key.map(|value| value.to_string()),
            content: content.to_string(),
            updated_at,
            deleted_at: None,
            device_id: "device-a".to_string(),
        }
    }

    #[test]
    fn merge_creates_conflict_copy_when_both_sides_changed() {
        let local = TemplateStore {
            dataset_version: 10,
            folders: vec![folder()],
            templates: vec![template("t1", 100, "地址", Some("addr"), "本地内容")],
        };

        let mut remote_template = template("t1", 120, "地址", Some("addr"), "云端内容");
        remote_template.device_id = "device-b".to_string();

        let remote = TemplateStore {
            dataset_version: 20,
            folders: vec![folder()],
            templates: vec![remote_template],
        };

        let (merged, report) = merge_stores(&local, &remote, 90, "device-a", 200);
        assert_eq!(merged.templates.len(), 2);
        assert_eq!(report.conflict_copies.len(), 1);
    }

    #[test]
    fn merge_resolves_duplicate_keys() {
        let local = TemplateStore {
            dataset_version: 10,
            folders: vec![folder()],
            templates: vec![template("t1", 100, "模板一", Some("dup"), "A")],
        };

        let remote = TemplateStore {
            dataset_version: 20,
            folders: vec![folder()],
            templates: vec![template("t2", 101, "模板二", Some("dup"), "B")],
        };

        let (merged, report) = merge_stores(&local, &remote, 0, "device-a", 200);
        let duplicate_count = merged
            .templates
            .iter()
            .filter(|item| item.deleted_at.is_none() && item.key.as_deref() == Some("dup"))
            .count();

        assert_eq!(duplicate_count, 1);
        assert_eq!(report.key_conflicts.len(), 1);
    }
}
