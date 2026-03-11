export interface WebDavSettings {
  url: string;
  username: string;
  password: string;
  remoteFile: string;
}

export interface Settings {
  shortcut: string;
  launchAtStartup: boolean;
  launchAtStartupEffective: boolean;
  webdav: WebDavSettings;
  lastSyncedVersion: number;
  lastUpdateCheckAt: number;
  lastSeenAppVersion: string;
  deviceId: string;
}

export interface Folder {
  id: string;
  name: string;
  sortOrder: number;
  sortUpdatedAt: number;
  updatedAt: number;
  deletedAt: number | null;
  deviceId: string;
}

export interface TemplateItem {
  id: string;
  folderId: string;
  name: string;
  key: string | null;
  content: string;
  sortOrder: number;
  sortUpdatedAt: number;
  updatedAt: number;
  deletedAt: number | null;
  deviceId: string;
}

export interface TemplateStore {
  datasetVersion: number;
  folders: Folder[];
  templates: TemplateItem[];
}

export interface SyncResult {
  blocked: boolean;
  level: "success" | "warn";
  message: string;
  localVersion: number;
  remoteVersion: number;
  conflictCopies: string[];
  keyConflicts: string[];
}

export interface OverlayContext {
  query: string;
}

export interface AppUpdateCheckResult {
  status: "available" | "latest" | "error";
  hasUpdate: boolean;
  message: string;
  currentVersion: string;
  latestVersion: string | null;
  releaseUrl: string;
  releaseNotes: string;
  publishedAt: string | null;
  lastCheckAt: number;
}

export interface AppUpdateWelcome {
  previousVersion: string;
  currentVersion: string;
}

export interface AppUpdateProgressEvent {
  phase: "downloading" | "installing" | "finished" | "error";
  version: string;
  downloadedBytes: number;
  totalBytes: number | null;
  message: string;
}
