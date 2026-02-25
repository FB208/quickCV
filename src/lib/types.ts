export interface WebDavSettings {
  url: string;
  username: string;
  password: string;
  remoteFile: string;
}

export interface UpdaterSettings {
  endpoint: string;
  pubkey: string;
}

export interface Settings {
  shortcut: string;
  launchAtStartup: boolean;
  webdav: WebDavSettings;
  updater: UpdaterSettings;
  lastSyncedVersion: number;
  deviceId: string;
}

export interface Folder {
  id: string;
  name: string;
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
  message: string;
  localVersion: number;
  remoteVersion: number;
  conflictCopies: string[];
  keyConflicts: string[];
}

export interface OverlayContext {
  query: string;
}

export type UpdateStatus = "not_configured" | "available" | "latest" | "error";

export interface UpdateCheckResult {
  status: UpdateStatus;
  message: string;
  currentVersion: string;
  latestVersion: string | null;
}
