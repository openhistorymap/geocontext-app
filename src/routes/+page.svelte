<script lang="ts">
  import TopLevelForm from '$lib/components/TopLevelForm.svelte';
  import DatasourcesEditor from '$lib/components/DatasourcesEditor.svelte';
  import LayersEditor from '$lib/components/LayersEditor.svelte';
  import MapPreview from '$lib/components/MapPreview.svelte';
  import JsonView from '$lib/components/JsonView.svelte';
  import AssetsBrowser from '$lib/components/AssetsBrowser.svelte';
  import WorkspaceMenu from '$lib/components/WorkspaceMenu.svelte';
  import AccountMenu from '$lib/components/AccountMenu.svelte';
  import GitPanel from '$lib/components/GitPanel.svelte';
  import CloneDialog from '$lib/components/CloneDialog.svelte';
  import { onMount } from 'svelte';
  import { validate, summarize } from '$lib/validate';
  import { emptyGeoContext, type GeoContext } from '$lib/types';
  import {
    isTauri,
    pickFolder,
    loadGeoContext,
    saveGeoContext,
    createGeoContextRepo,
    touchWorkspace,
    authState,
    type WorkspaceEntry,
    type CloneResult,
    type GitStatus
  } from '$lib/tauri';
  import type { RepoCoords } from '$lib/assetPath';

  type Tab = 'general' | 'datasources' | 'layers' | 'assets' | 'json';
  let tab = $state<Tab>('general');

  // Single source of truth — children mutate via $bindable
  let working = $state<GeoContext>(emptyGeoContext());
  let opened = $state<{ folder: string; filename: 'geocontext.json' | 'gcx.json' } | null>(null);
  let dirty = $state(false);

  let repoUser = $state('');
  let repoProject = $state('');
  let repoRef = $state('HEAD');
  let repo = $derived<RepoCoords | null>(
    repoUser && repoProject ? { user: repoUser, project: repoProject, ref: repoRef || 'HEAD' } : null
  );

  let issues = $derived(validate(working));
  let summary = $derived(summarize(issues));
  let tauri = $derived(isTauri());

  let baseline = $state<string>(JSON.stringify(emptyGeoContext()));
  $effect(() => {
    const cur = JSON.stringify(working);
    if (cur !== baseline) dirty = true;
  });

  function fmtCount(n: number): string {
    return n.toString().padStart(2, '0');
  }

  let menu = $state<WorkspaceMenu | undefined>(undefined);
  let cloneOpen = $state(false);
  let hasToken = $state(false);

  async function refreshAuth() {
    if (!isTauri()) { hasToken = false; return; }
    try { hasToken = (await authState()).has_token; }
    catch { hasToken = false; }
  }

  onMount(() => { refreshAuth(); });

  function confirmDiscardIfDirty(): boolean {
    if (!dirty) return true;
    return confirm('You have unsaved changes — switch anyway and lose them?');
  }

  async function loadFolder(folder: string) {
    const r = await loadGeoContext(folder);
    const parsed = JSON.parse(r.content) as GeoContext;
    working = parsed;
    opened = { folder: r.folder, filename: r.filename as 'geocontext.json' | 'gcx.json' };
    baseline = JSON.stringify(working);
    dirty = false;
    const m = folder.split(/[\\/]/).pop()?.match(/^([^_/-]+)[-_]([^_/-]+)/);
    if (m && (!repoUser || !repoProject)) { repoUser = m[1]; repoProject = m[2]; }
    await touchWorkspace(r.folder, parsed.title ?? null, r.filename);
    await menu?.refresh();
  }

  async function openRepo() {
    if (!confirmDiscardIfDirty()) return;
    const folder = await pickFolder();
    if (!folder) return;
    try { await loadFolder(folder); }
    catch (e) { alert(`Failed to load: ${(e as Error).message}`); }
  }

  async function newRepo() {
    if (!confirmDiscardIfDirty()) return;
    const folder = await pickFolder();
    if (!folder) return;
    const gc = emptyGeoContext();
    try {
      await createGeoContextRepo(folder, 'geocontext.json', JSON.stringify(gc, null, 2));
      working = gc;
      opened = { folder, filename: 'geocontext.json' };
      baseline = JSON.stringify(working);
      dirty = false;
      await touchWorkspace(folder, gc.title ?? null, 'geocontext.json');
      await menu?.refresh();
    } catch (e) {
      alert(`Failed to create: ${(e as Error).message}`);
    }
  }

  async function switchTo(entry: WorkspaceEntry) {
    if (!confirmDiscardIfDirty()) return;
    try { await loadFolder(entry.path); }
    catch (e) { alert(`Failed to switch: ${(e as Error).message}`); }
  }

  function openClone() {
    cloneOpen = true;
  }

  async function afterClone(result: CloneResult) {
    if (!confirmDiscardIfDirty()) return;
    try {
      await loadFolder(result.folder);
    } catch (e) {
      alert(`Cloned but couldn't load: ${(e as Error).message}`);
    }
  }

  async function afterSynced(_status: GitStatus) {
    // No-op for now; the GitPanel manages its own refresh.
  }

  async function save() {
    if (!opened) { alert('Open or create a repo folder first.'); return; }
    try {
      const content = JSON.stringify(working, null, 2) + '\n';
      await saveGeoContext(opened.folder, opened.filename, content);
      baseline = JSON.stringify(working);
      dirty = false;
      await touchWorkspace(opened.folder, working.title ?? null, opened.filename);
      await menu?.refresh();
    } catch (e) {
      alert(`Save failed: ${(e as Error).message}`);
    }
  }

  async function saveAsGeoContext() {
    if (!opened) return;
    try {
      const content = JSON.stringify(working, null, 2) + '\n';
      await saveGeoContext(opened.folder, 'geocontext.json', content);
      opened = { folder: opened.folder, filename: 'geocontext.json' };
      baseline = JSON.stringify(working);
      dirty = false;
    } catch (e) {
      alert((e as Error).message);
    }
  }

  function resetModel() {
    if (!confirm('Discard current edits and start a blank document?')) return;
    working = emptyGeoContext();
    dirty = true;
  }

  // Truncate long paths from the left for the header
  function shortPath(p: string, max = 64): string {
    if (p.length <= max) return p;
    return '…' + p.slice(p.length - max + 1);
  }
</script>

<div class="plate">
  <header class="plate__header">
    <h1 class="plate__brand">
      <span class="brand-mark">◐</span> GeoContext
      <span class="brand-sub">Editor</span>
    </h1>

    <div class="plate__meta">
      <WorkspaceMenu
        bind:this={menu}
        activePath={opened?.folder ?? null}
        onpick={switchTo}
        onnew={newRepo}
        onopenfolder={openRepo}
        onclone={openClone}
      />
      {#if opened}
        <span class="path mono" title={`${opened.folder}/${opened.filename}`}>{shortPath(`${opened.folder}/${opened.filename}`)}</span>
        {#if dirty}<span class="status">Unsaved</span>{/if}
      {/if}
    </div>

    <div class="plate__actions">
      <AccountMenu onchange={refreshAuth} />
      <button class="btn" onclick={resetModel}>Reset</button>
      {#if opened?.filename === 'gcx.json'}
        <button class="btn" onclick={saveAsGeoContext} title="rename legacy file to geocontext.json">Promote</button>
      {/if}
      <button class="btn btn--primary" onclick={save} disabled={!opened || !tauri}>Save</button>
    </div>
  </header>

  {#if opened && tauri}
    <div class="plate__git">
      <GitPanel
        folder={opened.folder}
        title={working.title}
        {hasToken}
        onsynced={afterSynced}
      />
    </div>
  {/if}

  <nav class="plate__tabs" aria-label="editor sections">
    <button class="tab" class:is-active={tab === 'general'} onclick={() => (tab = 'general')}>
      General
    </button>
    <button class="tab" class:is-active={tab === 'datasources'} onclick={() => (tab = 'datasources')}>
      Datasources <span class="tab__count">{fmtCount(working.datasources?.length ?? 0)}</span>
    </button>
    <button class="tab" class:is-active={tab === 'layers'} onclick={() => (tab = 'layers')}>
      Layers <span class="tab__count">{fmtCount(working.layers?.length ?? 0)}</span>
    </button>
    <button class="tab" class:is-active={tab === 'assets'} onclick={() => (tab = 'assets')}>
      Assets
    </button>
    <button class="tab" class:is-active={tab === 'json'} onclick={() => (tab = 'json')}>
      Source
    </button>
  </nav>

  <main class="plate__body">
    <section class="plate__editor">
      {#if tab === 'general'}
        <TopLevelForm bind:model={working} {issues} />
      {:else if tab === 'datasources'}
        <DatasourcesEditor bind:datasources={working.datasources} {issues} folder={opened?.folder ?? null} />
      {:else if tab === 'layers'}
        <LayersEditor bind:layers={working.layers} datasources={working.datasources} {issues} />
      {:else if tab === 'assets'}
        <AssetsBrowser folder={opened?.folder ?? null} model={working} />
      {:else}
        <JsonView bind:model={working} />
      {/if}
    </section>

    <section class="plate__preview">
      <MapPreview model={working} {repo} bind:repoUser bind:repoProject bind:repoRef />
    </section>
  </main>

  {#if cloneOpen}
    <CloneDialog onclose={() => (cloneOpen = false)} oncloned={afterClone} />
  {/if}

  <footer class="plate__footer">
    <span class="stats">
      {fmtCount(working.layers?.length ?? 0)} layers · {fmtCount(working.datasources?.length ?? 0)} sources
    </span>
    <span class="issues" class:issues--has-error={summary.errors > 0}>
      {summary.errors} {summary.errors === 1 ? 'error' : 'errors'}
      ·
      {summary.warns} {summary.warns === 1 ? 'warning' : 'warnings'}
    </span>
    <span class="version mono">
      {#if !tauri}browser preview · {/if}geocontext-editor / 0.5.2
    </span>
  </footer>
</div>
