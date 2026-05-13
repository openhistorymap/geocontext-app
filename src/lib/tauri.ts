// Thin wrapper around Tauri commands so the editor can degrade gracefully
// when run in a plain browser (e.g. `npm run dev` without the Tauri shell).

import { invoke } from '@tauri-apps/api/core';

export interface LoadedGeoContext {
  folder: string;
  filename: 'geocontext.json' | 'gcx.json';
  content: string;
}

export const isTauri = (): boolean => {
  if (typeof window === 'undefined') return false;
  // Tauri v2 exposes __TAURI_INTERNALS__
  return Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
};

export async function pickFolder(): Promise<string | null> {
  if (!isTauri()) {
    alert('Folder picking only works inside the Tauri app.');
    return null;
  }
  const { open } = await import('@tauri-apps/plugin-dialog');
  const res = await open({ directory: true, multiple: false });
  if (Array.isArray(res)) return res[0] ?? null;
  return (res as string | null) ?? null;
}

export async function loadGeoContext(folder: string): Promise<LoadedGeoContext> {
  return await invoke<LoadedGeoContext>('load_geocontext', { folder });
}

export async function saveGeoContext(
  folder: string,
  filename: string,
  content: string
): Promise<void> {
  await invoke('save_geocontext', { folder, filename, content });
}

export async function createGeoContextRepo(
  folder: string,
  filename: string,
  content: string
): Promise<void> {
  await invoke('create_geocontext_repo', { folder, filename, content });
}

export async function listAssets(folder: string): Promise<string[]> {
  return await invoke<string[]>('list_assets', { folder });
}

export interface RepoAsset {
  rel_path: string;
  abs_path: string;
  kind: 'dataset' | 'image' | 'html' | 'download' | 'other';
  size: number;
  modified_secs: number | null;
}

export async function listRepoAssets(folder: string): Promise<RepoAsset[]> {
  return await invoke<RepoAsset[]>('list_repo_assets', { folder });
}

export interface ImportedAsset {
  rel_path: string;
  abs_path: string;
  kind: string;
  size: number;
}

export interface PrjInfo {
  epsg: number | null;
  name: string | null;
  prj_present: boolean;
  raw_wkt: string;
}

export async function pickLocalFile(
  filters?: { name: string; extensions: string[] }[]
): Promise<string | null> {
  if (!isTauri()) { alert('File picking only works inside the Tauri app.'); return null; }
  const { open } = await import('@tauri-apps/plugin-dialog');
  const res = await open({ multiple: false, directory: false, filters });
  if (Array.isArray(res)) return res[0] ?? null;
  return (res as string | null) ?? null;
}

export async function detectShpCrs(shpPath: string): Promise<PrjInfo> {
  return await invoke<PrjInfo>('detect_shp_crs', { shpPath });
}

export async function importGeojsonLocal(
  folder: string,
  srcPath: string,
  nameOverride?: string | null
): Promise<ImportedAsset> {
  return await invoke<ImportedAsset>('import_geojson_local', {
    folder,
    srcPath,
    nameOverride: nameOverride ?? null
  });
}

export async function importShpLocal(
  folder: string,
  srcPath: string,
  sourceEpsg: number,
  nameOverride?: string | null
): Promise<ImportedAsset> {
  return await invoke<ImportedAsset>('import_shp_local', {
    folder,
    srcPath,
    sourceEpsg,
    nameOverride: nameOverride ?? null
  });
}

export async function importAssetLocal(
  folder: string,
  srcPath: string,
  kind: 'image' | 'html' | 'download' | 'dataset' | 'other',
  nameOverride?: string | null
): Promise<ImportedAsset> {
  return await invoke<ImportedAsset>('import_asset_local', {
    folder,
    srcPath,
    kind,
    nameOverride: nameOverride ?? null
  });
}

export async function deleteRepoAsset(folder: string, relPath: string): Promise<void> {
  await invoke('delete_repo_asset', { folder, relPath });
}

export interface WorkspaceEntry {
  path: string;
  title: string | null;
  filename: string | null;
  last_opened_unix: number;
  reachable: boolean;
}

export async function listWorkspaces(): Promise<WorkspaceEntry[]> {
  if (!isTauri()) return [];
  return await invoke<WorkspaceEntry[]>('list_workspaces');
}

export async function touchWorkspace(
  path: string,
  title?: string | null,
  filename?: string | null
): Promise<WorkspaceEntry[]> {
  return await invoke<WorkspaceEntry[]>('touch_workspace', {
    path,
    title: title ?? null,
    filename: filename ?? null
  });
}

export async function removeWorkspace(path: string): Promise<WorkspaceEntry[]> {
  return await invoke<WorkspaceEntry[]>('remove_workspace', { path });
}

export async function forgetUnreachableWorkspaces(): Promise<WorkspaceEntry[]> {
  return await invoke<WorkspaceEntry[]>('forget_unreachable_workspaces');
}

// ──────────────────────────────────────────────────────────────────
// Git + GitHub

export interface GitStatus {
  is_repo: boolean;
  branch: string | null;
  remote: string | null;
  remote_url: string | null;
  ahead: number;
  behind: number;
  dirty: number;
  head_sha: string | null;
}

export interface CloneResult {
  folder: string;
  default_branch: string | null;
}

export async function gitStatus(folder: string): Promise<GitStatus> {
  return await invoke<GitStatus>('git_status', { folder });
}

export async function gitClone(url: string, dest: string): Promise<CloneResult> {
  return await invoke<CloneResult>('git_clone', { url, dest });
}

export async function gitPull(folder: string): Promise<GitStatus> {
  return await invoke<GitStatus>('git_pull', { folder });
}

export async function gitSync(
  folder: string,
  message: string,
  name?: string | null,
  email?: string | null
): Promise<GitStatus> {
  return await invoke<GitStatus>('git_sync', {
    folder,
    message,
    name: name ?? null,
    email: email ?? null
  });
}

export async function gitInitWithOrigin(folder: string, originUrl: string): Promise<void> {
  await invoke('git_init_with_origin', { folder, originUrl });
}

export interface AuthState {
  has_token: boolean;
  oauth_available: boolean;
}
export async function authState(): Promise<AuthState> {
  if (!isTauri()) return { has_token: false, oauth_available: false };
  return await invoke<AuthState>('auth_state');
}
export async function authSetToken(token: string): Promise<void> {
  await invoke('auth_set_token', { token });
}
export async function authClearToken(): Promise<void> {
  await invoke('auth_clear_token');
}
export async function authPatUrl(): Promise<string> {
  return await invoke<string>('auth_pat_url');
}

export interface DeviceFlowStart {
  user_code: string;
  device_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
}
export async function githubOauthStart(): Promise<DeviceFlowStart> {
  return await invoke<DeviceFlowStart>('github_oauth_start');
}

export type PollResult =
  | { kind: 'pending' }
  | { kind: 'slow_down'; interval: number }
  | { kind: 'token'; access_token: string; scope: string | null }
  | { kind: 'denied' }
  | { kind: 'expired' }
  | { kind: 'error'; message: string };

export async function githubOauthPoll(deviceCode: string): Promise<PollResult> {
  return await invoke<PollResult>('github_oauth_poll', { deviceCode });
}

export interface WhoAmI {
  login: string;
  name: string | null;
  avatar_url: string | null;
  scopes: string[];
}
export async function githubWhoami(): Promise<WhoAmI | null> {
  return await invoke<WhoAmI | null>('github_whoami');
}

export interface CreatedRepo {
  html_url: string;
  clone_url: string;
  ssh_url: string;
  default_branch: string;
  full_name: string;
}
export async function githubCreateRepo(
  name: string,
  description: string | null,
  isPrivate: boolean,
  ownerOrg: string | null
): Promise<CreatedRepo> {
  return await invoke<CreatedRepo>('github_create_repo', {
    name,
    description,
    isPrivate,
    ownerOrg
  });
}

/// Open a URL in the user's default browser via the Tauri opener plugin
/// when available, otherwise via `window.open`.
export async function openExternal(url: string): Promise<void> {
  if (typeof window !== 'undefined') {
    window.open(url, '_blank');
  }
}
