use crate::logger;
use crate::models::WebDavSettings;
use crate::webdav;
use tauri::AppHandle;

pub async fn test_webdav(
    app: &AppHandle,
    webdav_settings: WebDavSettings,
) -> Result<String, String> {
    logger::info(app, "webdav", "test_webdav command start");
    webdav::test_connection(&webdav_settings)
        .await
        .map_err(|error| {
            logger::error(
                app,
                "webdav",
                &format!("test_webdav command failed: {error}"),
            );
            error
        })?;
    logger::info(app, "webdav", "test_webdav command success");
    Ok("WebDAV 连通成功".to_string())
}
