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
