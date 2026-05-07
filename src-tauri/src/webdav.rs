use std::time::Duration;

use reqwest::{Client, Method, StatusCode};

use crate::models::{TemplateStore, WebDavSettings};

fn build_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|error| format!("初始化网络连接失败: {error}"))
}

fn normalize_base_url(value: &str) -> String {
    value.trim().trim_end_matches('/').to_string()
}

fn normalize_remote_file(value: &str) -> Result<String, String> {
    let file = value.trim().replace('\\', "/");
    let file = file.trim_start_matches('/').to_string();
    if file.is_empty() {
        return Err("请先填写云端文件名称".to_string());
    }
    if file.ends_with('/') {
        return Err("云端文件名称不能是目录，请填写 JSON 文件名".to_string());
    }

    Ok(file)
}

fn remote_url(settings: &WebDavSettings) -> Result<String, String> {
    let base = normalize_base_url(&settings.url);
    if base.is_empty() {
        return Err("请先填写云同步地址".to_string());
    }

    let file = normalize_remote_file(&settings.remote_file)?;

    Ok(format!("{}/{}", base, file))
}

fn remote_parent_urls(settings: &WebDavSettings) -> Result<Vec<String>, String> {
    let base = normalize_base_url(&settings.url);
    if base.is_empty() {
        return Err("请先填写云同步地址".to_string());
    }

    let file = normalize_remote_file(&settings.remote_file)?;
    let segments = file
        .split('/')
        .filter(|segment| !segment.trim().is_empty())
        .collect::<Vec<_>>();
    if segments.len() <= 1 {
        return Ok(Vec::new());
    }

    let mut urls = Vec::new();
    let mut current = base;
    for segment in segments.iter().take(segments.len() - 1) {
        current = format!("{}/{}", current, segment);
        urls.push(current.clone());
    }

    Ok(urls)
}

fn with_auth(
    request: reqwest::RequestBuilder,
    settings: &WebDavSettings,
) -> reqwest::RequestBuilder {
    if settings.username.trim().is_empty() {
        request
    } else {
        request.basic_auth(
            settings.username.trim().to_string(),
            Some(settings.password.clone()),
        )
    }
}

pub async fn test_connection(settings: &WebDavSettings) -> Result<(), String> {
    let client = build_client()?;
    let base = normalize_base_url(&settings.url);
    if base.is_empty() {
        return Err("请先填写云同步地址".to_string());
    }

    let propfind =
        Method::from_bytes(b"PROPFIND").map_err(|error| format!("创建连接请求失败: {error}"))?;
    let request = client.request(propfind, &base).header("Depth", "0");
    let response = with_auth(request, settings)
        .send()
        .await
        .map_err(|error| format!("连接云同步地址失败: {error}"))?;

    if response.status().is_success() || response.status().as_u16() == 207 {
        return Ok(());
    }

    if response.status() == StatusCode::METHOD_NOT_ALLOWED {
        let fallback = with_auth(client.get(&base), settings)
            .send()
            .await
            .map_err(|error| format!("连接测试失败: {error}"))?;

        if fallback.status().is_success() {
            return Ok(());
        }

        return Err(format!(
            "连接失败，请检查地址或账号信息（状态码：{}）",
            fallback.status()
        ));
    }

    Err(format!(
        "连接失败，请检查地址或账号信息（状态码：{}）",
        response.status()
    ))
}

pub async fn fetch_remote_store(
    settings: &WebDavSettings,
) -> Result<Option<TemplateStore>, String> {
    let client = build_client()?;
    let url = remote_url(settings)?;
    let response = with_auth(client.get(&url), settings)
        .send()
        .await
        .map_err(|error| format!("读取云端内容失败: {error}"))?;

    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !response.status().is_success() {
        return Err(format!("读取云端内容失败（状态码：{}）", response.status()));
    }

    let text = response
        .text()
        .await
        .map_err(|error| format!("读取云端内容失败: {error}"))?;

    if text.trim().is_empty() {
        return Ok(Some(TemplateStore::default()));
    }

    let store = serde_json::from_str::<TemplateStore>(&text)
        .map_err(|error| format!("云端数据无法识别，请检查同步文件是否完整可用: {error}"))?;
    Ok(Some(store))
}

pub async fn push_remote_store(
    settings: &WebDavSettings,
    store: &TemplateStore,
) -> Result<(), String> {
    let client = build_client()?;
    let url = remote_url(settings)?;
    let body = serde_json::to_string_pretty(store)
        .map_err(|error| format!("整理同步内容失败: {error}"))?;

    ensure_remote_parent_dirs(&client, settings).await?;

    let response = with_auth(
        client
            .put(&url)
            .header("Content-Type", "application/json; charset=utf-8")
            .body(body),
        settings,
    )
    .send()
    .await
    .map_err(|error| format!("上传云端内容失败: {error}"))?;

    if response.status().is_success() {
        return Ok(());
    }

    if response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::CONFLICT {
        return Err(format!(
            "上传云端内容失败（状态码：{}）。请检查云同步地址是否是已存在的 WebDAV 目录，云端文件名称中的父目录是否存在或可创建",
            response.status()
        ));
    }

    Err(format!("上传云端内容失败（状态码：{}）", response.status()))
}

async fn ensure_remote_parent_dirs(
    client: &Client,
    settings: &WebDavSettings,
) -> Result<(), String> {
    let mkcol =
        Method::from_bytes(b"MKCOL").map_err(|error| format!("创建云端目录请求失败: {error}"))?;

    for url in remote_parent_urls(settings)? {
        let response = with_auth(client.request(mkcol.clone(), &url), settings)
            .send()
            .await
            .map_err(|error| format!("创建云端目录失败: {error}"))?;

        if response.status().is_success() || response.status() == StatusCode::METHOD_NOT_ALLOWED {
            continue;
        }

        if response.status() == StatusCode::CONFLICT || response.status() == StatusCode::NOT_FOUND {
            return Err(format!(
                "创建云端目录失败（状态码：{}）。请检查云同步地址是否是已存在的 WebDAV 目录：{}",
                response.status(),
                url
            ));
        }

        return Err(format!("创建云端目录失败（状态码：{}）", response.status()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(remote_file: &str) -> WebDavSettings {
        WebDavSettings {
            url: "https://dav.example.com/root/".to_string(),
            username: String::new(),
            password: String::new(),
            remote_file: remote_file.to_string(),
        }
    }

    #[test]
    fn remote_url_normalizes_slashes() {
        let result = remote_url(&settings("/quickcv\\data.json")).unwrap();
        assert_eq!(result, "https://dav.example.com/root/quickcv/data.json");
    }

    #[test]
    fn remote_parent_urls_include_each_parent() {
        let result = remote_parent_urls(&settings("quickcv/nested/data.json")).unwrap();
        assert_eq!(
            result,
            vec![
                "https://dav.example.com/root/quickcv".to_string(),
                "https://dav.example.com/root/quickcv/nested".to_string(),
            ]
        );
    }
}
