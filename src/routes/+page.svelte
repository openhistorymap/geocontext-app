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
    if (!opened) { alert('Open or create a repo folder first.'); return; }
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
      {#if opened}
        <span class="path mono">{shortPath(`${opened.folder}/${opened.filename}`)}</span>
        {#if dirty}<span class="status">Unsaved</span>{/if}
      {:else}
        <span class="meta">No repository open. Open or create one to begin.</span>
      {/if}
    </div>

    <div class="plate__actions">
      <button class="btn" onclick={openRepo} disabled={!tauri}>Open</button>
      <button class="btn" onclick={newRepo} disabled={!tauri}>New</button>
      <button class="btn" onclick={resetModel}>Reset</button>
      {#if opened?.filename === 'gcx.json'}
        <button class="btn" onclick={saveAsGeoContext} title="rename legacy file to geocontext.json">Promote</button>
      {/if}
      <button class="btn btn--primary" onclick={save} disabled={!opened || !tauri}>Save</button>
    </div>
  </header>

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
    <button class="tab" class:is-active={tab === 'json'} onclick={() => (tab = 'json')}>
      Source
    </button>
  </nav>

  <main class="plate__body">
    <section class="plate__editor">
      {#if tab === 'general'}
        <TopLevelForm bind:model={working} {issues} />
      {:else if tab === 'datasources'}
        <DatasourcesEditor bind:datasources={working.datasources} {issues} />
      {:else if tab === 'layers'}
        <LayersEditor bind:layers={working.layers} datasources={working.datasources} {issues} />
      {:else}
        <JsonView bind:model={working} />
      {/if}
    </section>

    <section class="plate__preview">
      <MapPreview model={working} {repo} bind:repoUser bind:repoProject bind:repoRef />
    </section>
  </main>

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
      {#if !tauri}browser preview · {/if}geocontext-editor / 0.2.0
    </span>
  </footer>
</div>
