use std::time::Duration;

use reqwest::{Client, Method, StatusCode};

use crate::models::{TemplateStore, WebDavSettings};

fn build_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|error| format!("创建 HTTP 客户端失败: {error}"))
}

fn normalize_base_url(value: &str) -> String {
    value.trim().trim_end_matches('/').to_string()
}

fn remote_url(settings: &WebDavSettings) -> Result<String, String> {
    let base = normalize_base_url(&settings.url);
    if base.is_empty() {
        return Err("请先配置 WebDAV 地址".to_string());
    }

    let file = settings.remote_file.trim();
    if file.is_empty() {
        return Err("请先配置远端文件名".to_string());
    }

    Ok(format!("{}/{}", base, file.trim_start_matches('/')))
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
        return Err("请先填写 WebDAV 地址".to_string());
    }

    let propfind =
        Method::from_bytes(b"PROPFIND").map_err(|error| format!("构造 PROPFIND 失败: {error}"))?;
    let request = client.request(propfind, &base).header("Depth", "0");
    let response = with_auth(request, settings)
        .send()
        .await
        .map_err(|error| format!("连接 WebDAV 失败: {error}"))?;

    if response.status().is_success() || response.status().as_u16() == 207 {
        return Ok(());
    }

    if response.status() == StatusCode::METHOD_NOT_ALLOWED {
        let fallback = with_auth(client.get(&base), settings)
            .send()
            .await
            .map_err(|error| format!("GET 测试失败: {error}"))?;

        if fallback.status().is_success() {
            return Ok(());
        }

        return Err(format!("连接失败，状态码: {}", fallback.status()));
    }

    Err(format!("连接失败，状态码: {}", response.status()))
}

pub async fn fetch_remote_store(
    settings: &WebDavSettings,
) -> Result<Option<TemplateStore>, String> {
    let client = build_client()?;
    let url = remote_url(settings)?;
    let response = with_auth(client.get(&url), settings)
        .send()
        .await
        .map_err(|error| format!("拉取云端数据失败: {error}"))?;

    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !response.status().is_success() {
        return Err(format!("拉取云端数据失败，状态码: {}", response.status()));
    }

    let text = response
        .text()
        .await
        .map_err(|error| format!("读取云端数据失败: {error}"))?;

    if text.trim().is_empty() {
        return Ok(Some(TemplateStore::default()));
    }

    let store = serde_json::from_str::<TemplateStore>(&text)
        .map_err(|error| format!("解析云端 JSON 失败，请检查编码和格式: {error}"))?;
    Ok(Some(store))
}

pub async fn push_remote_store(
    settings: &WebDavSettings,
    store: &TemplateStore,
) -> Result<(), String> {
    let client = build_client()?;
    let url = remote_url(settings)?;
    let body = serde_json::to_string_pretty(store)
        .map_err(|error| format!("序列化云端数据失败: {error}"))?;

    let response = with_auth(
        client
            .put(&url)
            .header("Content-Type", "application/json; charset=utf-8")
            .body(body),
        settings,
    )
    .send()
    .await
    .map_err(|error| format!("推送云端数据失败: {error}"))?;

    if response.status().is_success() {
        return Ok(());
    }

    Err(format!("推送云端数据失败，状态码: {}", response.status()))
}
