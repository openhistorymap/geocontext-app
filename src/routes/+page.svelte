<script lang="ts">
  import TopLevelForm from '$lib/components/TopLevelForm.svelte';
  import DatasourcesEditor from '$lib/components/DatasourcesEditor.svelte';
  import LayersEditor from '$lib/components/LayersEditor.svelte';
  import MapPreview from '$lib/components/MapPreview.svelte';
  import JsonView from '$lib/components/JsonView.svelte';
  import { validate, summarize } from '$lib/validate';
  import { emptyGeoContext, type GeoContext } from '$lib/types';
  import {
    isTauri,
    pickFolder,
    loadGeoContext,
    saveGeoContext,
    createGeoContextRepo
  } from '$lib/tauri';
  import type { RepoCoords } from '$lib/assetPath';

  type Tab = 'general' | 'datasources' | 'layers' | 'json';
  let tab = $state<Tab>('general');

  // Single source of truth — children mutate this via $bindable
  let working = $state<GeoContext>(emptyGeoContext());
  let opened = $state<{ folder: string; filename: 'geocontext.json' | 'gcx.json' } | null>(null);
  let dirty = $state(false);

  // Repo coords for §9 path rewrite in the preview
  let repoUser = $state('');
  let repoProject = $state('');
  let repoRef = $state('HEAD');
  let repo = $derived<RepoCoords | null>(
    repoUser && repoProject ? { user: repoUser, project: repoProject, ref: repoRef || 'HEAD' } : null
  );

  let issues = $derived(validate(working));
  let summary = $derived(summarize(issues));
  let tauri = $derived(isTauri());
  let statusText = $derived(
    opened ? `${opened.folder}/${opened.filename}${dirty ? ' •' : ''}` : 'No repo open'
  );

  // Track edits as dirty by hashing the model when the user is the cause.
  // We use a lightweight "user touched something" hook: any change after
  // load flips dirty true until save.
  let baseline = $state<string>(JSON.stringify(emptyGeoContext()));
  $effect(() => {
    const cur = JSON.stringify(working);
    if (cur !== baseline) dirty = true;
  });

  async function openRepo() {
    const folder = await pickFolder();
    if (!folder) return;
    try {
      const r = await loadGeoContext(folder);
      const parsed = JSON.parse(r.content) as GeoContext;
      working = parsed;
      opened = { folder: r.folder, filename: r.filename as 'geocontext.json' | 'gcx.json' };
      baseline = JSON.stringify(working);
      dirty = false;
      const m = folder.split(/[\\/]/).pop()?.match(/^([^_/-]+)[-_]([^_/-]+)/);
      if (m && !repoUser) { repoUser = m[1]; repoProject = m[2]; }
    } catch (e) {
      alert(`Failed to load: ${(e as Error).message}`);
    }
  }

  async function newRepo() {
    const folder = await pickFolder();
    if (!folder) return;
    const gc = emptyGeoContext();
    try {
      await createGeoContextRepo(folder, 'geocontext.json', JSON.stringify(gc, null, 2));
      working = gc;
      opened = { folder, filename: 'geocontext.json' };
      baseline = JSON.stringify(working);
      dirty = false;
    } catch (e) {
      alert(`Failed to create: ${(e as Error).message}`);
    }
  }

  async function save() {
    if (!opened) {
      alert('Open or create a repo folder first.');
      return;
    }
    try {
      const content = JSON.stringify(working, null, 2) + '\n';
      await saveGeoContext(opened.folder, opened.filename, content);
      baseline = JSON.stringify(working);
      dirty = false;
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
</script>

<div style="display: grid; grid-template-rows: auto auto 1fr; height: 100vh;">
  <div class="toolbar">
    <button onclick={openRepo} disabled={!tauri}>Open folder…</button>
    <button onclick={newRepo} disabled={!tauri}>New repo…</button>
    <button class="primary" onclick={save} disabled={!opened || !tauri}>Save</button>
    {#if opened?.filename === 'gcx.json'}
      <button onclick={saveAsGeoContext} title="rename legacy file to geocontext.json">Save as geocontext.json</button>
    {/if}
    <button onclick={resetModel}>Reset</button>
    <span class="muted" style="margin-left: 12px;">{statusText}</span>
    {#if !tauri}
      <span class="muted" style="margin-left: 8px; color: var(--warn);">
        (running outside Tauri — Open / New / Save disabled)
      </span>
    {/if}
    <div style="flex: 1;"></div>
    <span class="muted">repo coords:</span>
    <input type="text" placeholder="user" bind:value={repoUser} style="width: 110px;" />
    <span class="muted">/</span>
    <input type="text" placeholder="project" bind:value={repoProject} style="width: 130px;" />
    <span class="muted">@</span>
    <input type="text" placeholder="HEAD" bind:value={repoRef} style="width: 80px;" />
    <span class="muted" style="margin-left: 12px;">
      {summary.errors} err / {summary.warns} warn
    </span>
  </div>

  <div class="tabs">
    <button class:active={tab === 'general'} onclick={() => (tab = 'general')}>General</button>
    <button class:active={tab === 'datasources'} onclick={() => (tab = 'datasources')}>Datasources ({working.datasources?.length ?? 0})</button>
    <button class:active={tab === 'layers'} onclick={() => (tab = 'layers')}>Layers ({working.layers?.length ?? 0})</button>
    <button class:active={tab === 'json'} onclick={() => (tab = 'json')}>JSON</button>
  </div>

  <div style="display: grid; grid-template-columns: minmax(0, 1fr) minmax(360px, 45%); min-height: 0;">
    <div style="overflow: auto; padding: 12px; min-height: 0;">
      {#if tab === 'general'}
        <TopLevelForm bind:model={working} {issues} />
      {:else if tab === 'datasources'}
        <DatasourcesEditor bind:datasources={working.datasources} {issues} />
      {:else if tab === 'layers'}
        <LayersEditor bind:layers={working.layers} datasources={working.datasources} {issues} />
      {:else}
        <JsonView bind:model={working} />
      {/if}
    </div>
    <div style="border-left: 1px solid var(--border); padding: 8px; min-height: 0; display: flex; flex-direction: column;">
      <MapPreview model={working} {repo} />
    </div>
  </div>
</div>
