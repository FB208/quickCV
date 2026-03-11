import type {
  AppUpdateCheckResult,
  AppUpdateWelcome,
  OverlayContext,
  Settings,
  SyncResult,
  TemplateStore,
} from "./types";
import { invoke } from "@tauri-apps/api/core";

const isTauri = (): boolean => {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
};

const invokeTauri = async <T>(
  command: string,
  payload?: Record<string, unknown>,
): Promise<T> => {
  if (!isTauri()) {
    throw new Error("当前不在 Tauri 环境中");
  }

  return invoke<T>(command, payload);
};

export const loadSettings = async (): Promise<Settings> => {
  return invokeTauri<Settings>("load_settings");
};

export const saveSettings = async (settings: Settings): Promise<Settings> => {
  return invokeTauri<Settings>("save_settings", { settings });
};

export const loadTemplateStore = async (): Promise<TemplateStore> => {
  return invokeTauri<TemplateStore>("load_template_store");
};

export const saveTemplateStore = async (
  store: TemplateStore,
): Promise<TemplateStore> => {
  return invokeTauri<TemplateStore>("save_template_store", { store });
};

export const testWebDav = async (settings: Settings): Promise<string> => {
  return invokeTauri<string>("test_webdav", { webdav: settings.webdav });
};

export const syncPull = async (): Promise<SyncResult> => {
  return invokeTauri<SyncResult>("sync_pull");
};

export const syncPush = async (): Promise<SyncResult> => {
  return invokeTauri<SyncResult>("sync_push");
};

export const getAppVersion = async (): Promise<string> => {
  return invokeTauri<string>("get_app_version");
};

export const checkAppUpdate = async (): Promise<AppUpdateCheckResult> => {
  return invokeTauri<AppUpdateCheckResult>("check_app_update");
};

export const installAppUpdate = async (): Promise<void> => {
  return invokeTauri<void>("install_app_update");
};

export const peekAppUpdateWelcome = async (): Promise<AppUpdateWelcome | null> => {
  return invokeTauri<AppUpdateWelcome | null>("peek_app_update_welcome");
};

export const acknowledgeCurrentAppVersion = async (): Promise<void> => {
  return invokeTauri<void>("acknowledge_current_app_version");
};

export const openReleasePage = async (version?: string): Promise<void> => {
  return invokeTauri<void>("open_release_page", { version });
};

export const openConfigFolder = async (): Promise<void> => {
  return invokeTauri<void>("open_config_folder");
};

export const openMainTemplates = async (): Promise<void> => {
  return invokeTauri<void>("open_main_templates");
};

export const openOverlay = async (
  context?: Partial<OverlayContext>,
): Promise<void> => {
  return invokeTauri<void>("open_overlay", {
    query: context?.query,
  });
};

export const closeOverlay = async (): Promise<void> => {
  return invokeTauri<void>("close_overlay");
};

export const setOverlayDragging = async (dragging: boolean): Promise<void> => {
  return invokeTauri<void>("set_overlay_dragging", { dragging });
};

export const getOverlayContext = async (): Promise<OverlayContext> => {
  return invokeTauri<OverlayContext>("get_overlay_context");
};

export const copyTemplate = async (templateId: string): Promise<void> => {
  return invokeTauri<void>("copy_template", { templateId });
};

export const insertTemplate = async (templateId: string): Promise<void> => {
  return invokeTauri<void>("insert_template", { templateId });
};
