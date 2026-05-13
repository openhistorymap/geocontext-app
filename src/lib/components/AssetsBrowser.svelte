<script lang="ts">
  import type { GeoContext } from '$lib/types';
  import {
    isTauri,
    pickLocalFile,
    listRepoAssets,
    importAssetLocal,
    deleteRepoAsset,
    type RepoAsset
  } from '$lib/tauri';

  let {
    folder,
    model
  }: { folder: string | null; model: GeoContext } = $props();

  let assets = $state<RepoAsset[]>([]);
  let loading = $state(false);
  let err = $state<string | null>(null);
  let pickedKind = $state<'image' | 'html' | 'download' | 'other'>('image');

  // All literal paths the geocontext document references. We strip
  // `{prop}` placeholders so a template like `tombe/Tomba_{tomba}.html`
  // surfaces the prefix `tombe/` as the referenced directory.
  let referenced = $derived.by(() => {
    const refs = new Set<string>();
    const lit = new Set<string>(); // literal full paths
    const dir = new Set<string>(); // referenced directories (from placeholders)

    for (const ds of model.datasources ?? []) {
      const s = ds.conf?.source;
      if (typeof s === 'string' && s) {
        if (s.includes('{')) {
          const prefix = s.slice(0, s.indexOf('{'));
          const lastSlash = prefix.lastIndexOf('/');
          if (lastSlash >= 0) dir.add(prefix.slice(0, lastSlash + 1));
        } else if (!/^https?:\/\//i.test(s)) {
          lit.add(s.replace(/^\.?\//, ''));
        }
        refs.add(s);
      }
    }
    for (const l of model.layers ?? []) {
      for (const m of l.detail?.media ?? []) {
        if (typeof m.src !== 'string' || !m.src) continue;
        if (/^https?:\/\//i.test(m.src)) continue;
        if (m.src.includes('{')) {
          const prefix = m.src.slice(0, m.src.indexOf('{'));
          const lastSlash = prefix.lastIndexOf('/');
          if (lastSlash >= 0) dir.add(prefix.slice(0, lastSlash + 1));
        } else {
          lit.add(m.src.replace(/^\.?\//, ''));
        }
      }
    }
    return { refs, lit, dir };
  });

  function isReferenced(rel: string): boolean {
    if (referenced.lit.has(rel)) return true;
    for (const d of referenced.dir) {
      if (rel.startsWith(d)) return true;
    }
    return false;
  }

  async function refresh() {
    if (!folder) { assets = []; return; }
    loading = true;
    err = null;
    try {
      assets = await listRepoAssets(folder);
    } catch (e) {
      err = (e as Error).message;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void folder;
    refresh();
  });

  async function addAsset() {
    if (!folder) return;
    err = null;
    const path = await pickLocalFile();
    if (!path) return;
    try {
      await importAssetLocal(folder, path, pickedKind);
      await refresh();
    } catch (e) {
      err = (e as Error).message;
    }
  }

  async function removeAsset(a: RepoAsset) {
    if (!folder) return;
    if (!confirm(`Delete ${a.rel_path}?`)) return;
    try {
      await deleteRepoAsset(folder, a.rel_path);
      await refresh();
    } catch (e) {
      err = (e as Error).message;
    }
  }

  // Grouping
  let grouped = $derived.by(() => {
    const groups: Record<string, RepoAsset[]> = {
      dataset: [],
      image: [],
      html: [],
      download: [],
      other: []
    };
    for (const a of assets) {
      const k = (groups[a.kind] ?? groups.other);
      k.push(a);
    }
    return groups;
  });

  const KIND_LABEL: Record<string, string> = {
    dataset: 'Datasets',
    image: 'Images',
    html: 'HTML fragments',
    download: 'Downloads',
    other: 'Other'
  };

  const KIND_ORDER = ['dataset', 'image', 'html', 'download', 'other'];

  function fmtSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} kB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  function fmtModified(secs: number | null): string {
    if (!secs) return '';
    const d = new Date(secs * 1000);
    return d.toISOString().slice(0, 10);
  }

  let summary = $derived.by(() => {
    let total = assets.length;
    let orphans = 0;
    for (const a of assets) {
      if (!isReferenced(a.rel_path)) orphans++;
    }
    return { total, orphans };
  });
</script>

<div class="ab">
  <div class="section__head">
    <div class="col" style="gap: 2px;">
      <span class="section__title">Assets</span>
      <span class="section__hint">
        Files inside the repository. <span class="tabular">{summary.total}</span> items, <span class="tabular">{summary.orphans}</span> unreferenced.
      </span>
    </div>
    <div class="row" style="gap: var(--s-3);">
      <label class="field" style="width: 130px; gap: 0;">
        <span class="label" style="margin: 0;">Kind</span>
        <select bind:value={pickedKind}>
          <option value="image">image</option>
          <option value="html">html</option>
          <option value="download">download</option>
          <option value="other">other</option>
        </select>
      </label>
      <button class="btn" onclick={addAsset} disabled={!folder || !isTauri()}>+ Add file…</button>
      <button class="btn" onclick={refresh} disabled={!folder}>Refresh</button>
    </div>
  </div>

  {#if !folder}
    <p class="meta" style="margin-top: var(--s-4);">Open a repository to inspect its assets.</p>
  {:else if loading}
    <p class="meta" style="margin-top: var(--s-4);">Reading repository…</p>
  {:else if err}
    <p class="error" style="margin-top: var(--s-4);">{err}</p>
  {:else if assets.length === 0}
    <p class="meta" style="margin-top: var(--s-4);">Nothing in the repository yet beyond geocontext.json itself.</p>
  {:else}
    {#each KIND_ORDER as k (k)}
      {#if grouped[k]?.length}
        <div class="section" style="border-bottom-width: 0; padding-block: var(--s-4) 0;">
          <div class="section__head">
            <span class="section__title">{KIND_LABEL[k]}</span>
            <span class="section__hint mono">{grouped[k].length} {grouped[k].length === 1 ? 'file' : 'files'}</span>
          </div>
          <table class="ab__table tabular">
            <thead>
              <tr>
                <th>Path</th>
                <th style="text-align: right;">Size</th>
                <th>Modified</th>
                <th>Status</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {#each grouped[k] as a (a.rel_path)}
                <tr class:ab__row--orphan={!isReferenced(a.rel_path)}>
                  <td class="mono ab__path">{a.rel_path}</td>
                  <td style="text-align: right;">{fmtSize(a.size)}</td>
                  <td>{fmtModified(a.modified_secs)}</td>
                  <td class="ab__status">
                    {#if isReferenced(a.rel_path)}referenced{:else}orphan{/if}
                  </td>
                  <td>
                    <button class="btn btn--danger" onclick={() => removeAsset(a)}>Delete</button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    {/each}
  {/if}
</div>

<style>
  .ab {
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
    height: 100%;
    min-height: 0;
  }
  .ab__table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--t-sm);
  }
  .ab__table th {
    text-align: left;
    font-size: var(--t-mini);
    text-transform: uppercase;
    letter-spacing: var(--track-loose);
    font-weight: var(--w-med);
    color: var(--ink-mute);
    padding: var(--s-1) var(--s-3) var(--s-2) 0;
    border-bottom: var(--hairline) solid var(--rule);
  }
  .ab__table td {
    padding: var(--s-2) var(--s-3) var(--s-2) 0;
    border-bottom: var(--hairline) solid var(--rule-soft);
    vertical-align: middle;
  }
  .ab__path {
    max-width: 380px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ab__status {
    font-size: var(--t-mini);
    text-transform: uppercase;
    letter-spacing: var(--track-loose);
    color: var(--ink-mute);
  }
  .ab__row--orphan .ab__status { color: var(--accent); }
</style>
