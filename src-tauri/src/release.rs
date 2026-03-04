use std::time::Duration;

use reqwest::header::{ACCEPT, USER_AGENT};
use serde::Deserialize;

use crate::models::ReleaseCheckResult;

const RELEASE_API_URL: &str = "https://api.github.com/repos/FB208/quickCV/releases/latest";
pub const RELEASE_PAGE_URL: &str = "https://github.com/FB208/quickCV/releases";

#[derive(Debug, Deserialize)]
struct GithubLatestRelease {
    tag_name: String,
    html_url: String,
}

pub async fn check_release_version(current_version: String) -> Result<ReleaseCheckResult, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|error| format!("创建版本检查客户端失败: {error}"))?;

    let response = client
        .get(RELEASE_API_URL)
        .header(USER_AGENT, "quickcv-desktop")
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .await;

    let response = match response {
        Ok(value) => value,
        Err(error) => {
            return Ok(ReleaseCheckResult {
                status: "error".to_string(),
                has_update: false,
                message: format!("检查更新失败: {error}"),
                current_version,
                latest_version: None,
                release_url: RELEASE_PAGE_URL.to_string(),
            });
        }
    };

    if !response.status().is_success() {
        return Ok(ReleaseCheckResult {
            status: "error".to_string(),
            has_update: false,
            message: format!("检查更新失败，状态码: {}", response.status()),
            current_version,
            latest_version: None,
            release_url: RELEASE_PAGE_URL.to_string(),
        });
    }

    let payload = response
        .json::<GithubLatestRelease>()
        .await
        .map_err(|error| format!("解析版本信息失败: {error}"))?;

    let latest = normalize_version(&payload.tag_name);
    let has_update = version_compare(&latest, &current_version) > 0;

    let message = if has_update {
        format!("发现新版本 {}（当前 {}）", latest, current_version)
    } else {
        format!("当前已是最新版本（{}）", current_version)
    };

    Ok(ReleaseCheckResult {
        status: "ok".to_string(),
        has_update,
        message,
        current_version,
        latest_version: Some(latest),
        release_url: payload.html_url,
    })
}

fn normalize_version(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .to_string()
}

fn version_compare(left: &str, right: &str) -> i32 {
    let left_parts: Vec<i32> = normalize_version(left)
        .split('.')
        .map(|part| part.parse::<i32>().unwrap_or(0))
        .collect();
    let right_parts: Vec<i32> = normalize_version(right)
        .split('.')
        .map(|part| part.parse::<i32>().unwrap_or(0))
        .collect();

    let max_len = left_parts.len().max(right_parts.len());
    for index in 0..max_len {
        let left_value = left_parts.get(index).copied().unwrap_or(0);
        let right_value = right_parts.get(index).copied().unwrap_or(0);
        if left_value > right_value {
            return 1;
        }
        if left_value < right_value {
            return -1;
        }
    }

    0
}
