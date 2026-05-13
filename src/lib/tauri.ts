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
