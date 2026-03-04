use crate::logger;
use crate::models::TemplateStore;
use crate::storage;
use tauri::AppHandle;

pub fn load_template_store(app: &AppHandle) -> Result<TemplateStore, String> {
    logger::info(app, "store", "load_template_store command start");
    let store = storage::load_template_store(app).map_err(|error| {
        logger::error(app, "store", &format!("读取模板库失败: {error}"));
        error
    })?;
    logger::info(app, "store", "load_template_store command success");
    Ok(store)
}

pub fn save_template_store(app: &AppHandle, store: TemplateStore) -> Result<TemplateStore, String> {
    logger::info(app, "store", "save_template_store command start");
    let saved = storage::save_template_store(app, &store).map_err(|error| {
        logger::error(app, "store", &format!("保存模板库失败: {error}"));
        error
    })?;
    logger::info(app, "store", "save_template_store command success");
    Ok(saved)
}
