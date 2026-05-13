<script lang="ts">
  import type { Datasource, CsvColumn } from '$lib/types';
  import { newDatasource, newCsvColumn } from '$lib/types';
  import type { Issue } from '$lib/validate';
  import { issuesByPath } from '$lib/issuesByPath';
  import {
    isTauri,
    pickLocalFile,
    detectShpCrs,
    importGeojsonLocal,
    importShpLocal,
    type PrjInfo
  } from '$lib/tauri';

  let {
    datasources = $bindable(),
    issues,
    folder
  }: { datasources: Datasource[]; issues: Issue[]; folder: string | null } = $props();

  let pathIssues = $derived(issuesByPath(issues));
  let selected = $state<number>(0);

  // ── Import flow state ──
  type ImportKind = 'geojson' | 'shp' | null;
  let importOpen = $state(false);
  let importSrc = $state<string>('');
  let importKind = $state<ImportKind>(null);
  let importName = $state<string>('');
  let importEpsg = $state<number>(4326);
  let importDetected = $state<PrjInfo | null>(null);
  let importBusy = $state(false);
  let importError = $state<string | null>(null);

  // Common CRS shortcuts familiar to digital-humanities users
  const CRS_SHORTCUTS: { code: number; label: string }[] = [
    { code: 4326, label: '4326 — WGS 84 (lat/lon)' },
    { code: 3857, label: '3857 — Web Mercator' },
    { code: 3003, label: '3003 — Monte Mario / Italy zone 1' },
    { code: 3004, label: '3004 — Monte Mario / Italy zone 2' },
    { code: 25832, label: '25832 — ETRS89 / UTM 32N' },
    { code: 25833, label: '25833 — ETRS89 / UTM 33N' },
    { code: 32632, label: '32632 — WGS 84 / UTM 32N' },
    { code: 32633, label: '32633 — WGS 84 / UTM 33N' },
    { code: 27700, label: '27700 — OSGB 1936 / British National Grid' },
    { code: 2154,  label: '2154  — RGF93 / Lambert-93 (France)' },
    { code: 31370, label: '31370 — Belgian Lambert 72' }
  ];

  function basename(p: string): string {
    return p.split(/[\\/]/).pop() ?? p;
  }
  function stem(p: string): string {
    return basename(p).replace(/\.[^.]+$/, '');
  }
  function ext(p: string): string {
    const m = basename(p).match(/\.([^.]+)$/);
    return m ? m[1].toLowerCase() : '';
  }

  function resetImport() {
    importOpen = false;
    importSrc = '';
    importKind = null;
    importName = '';
    importEpsg = 4326;
    importDetected = null;
    importBusy = false;
    importError = null;
  }

  async function chooseImportFile() {
    importError = null;
    const path = await pickLocalFile([
      { name: 'GeoJSON or Shapefile', extensions: ['geojson', 'json', 'shp'] }
    ]);
    if (!path) return;
    importSrc = path;
    importName = stem(path);
    const e = ext(path);
    importKind = e === 'shp' ? 'shp' : 'geojson';
    importDetected = null;
    if (importKind === 'shp') {
      try {
        const detected = await detectShpCrs(path);
        importDetected = detected;
        if (detected.epsg) importEpsg = detected.epsg;
      } catch (err) {
        importError = `Could not read .prj: ${(err as Error).message}`;
      }
    }
  }

  async function runImport() {
    if (!folder) { importError = 'Open a repository folder first.'; return; }
    if (!importSrc) { importError = 'Pick a file first.'; return; }
    importBusy = true;
    importError = null;
    try {
      const safeName = importName || stem(importSrc) || `dataset_${datasources.length + 1}`;
      const result =
        importKind === 'shp'
          ? await importShpLocal(folder, importSrc, importEpsg, safeName)
          : await importGeojsonLocal(folder, importSrc, safeName);

      // Append a geojson+http+remote datasource pointing at the new file.
      const ds: Datasource = {
        name: safeName,
        type: 'geojson+http+remote',
        conf: { source: result.rel_path }
      };
      datasources.push(ds);
      datasources = datasources;
      selected = datasources.length - 1;
      resetImport();
    } catch (e) {
      importError = (e as Error).message;
    } finally {
      importBusy = false;
    }
  }

  function add() {
    datasources.push(newDatasource(`ds_${datasources.length + 1}`));
    datasources = datasources;
    selected = datasources.length - 1;
  }
  function remove(i: number) {
    datasources.splice(i, 1);
    datasources = datasources;
    if (selected >= datasources.length) selected = Math.max(0, datasources.length - 1);
  }
  function changeType(ds: Datasource, t: string) {
    ds.type = t;
    if (t === 'geojson' || t === 'csv') {
      ds.conf = {
        ...ds.conf,
        data: ds.conf?.data ?? (t === 'geojson' ? { type: 'FeatureCollection', features: [] } : '')
      };
      delete ds.conf.source;
    } else {
      ds.conf = { ...ds.conf, source: ds.conf?.source ?? '' };
      delete ds.conf.data;
    }
    if (t.startsWith('csv') && !Array.isArray(ds.conf.structure)) {
      ds.conf.structure = [
        { column: 'longitude', type: 'number', tags: ['gcx:lon', 'gcx:geo'] },
        { column: 'latitude', type: 'number', tags: ['gcx:lat', 'gcx:geo'] }
      ];
    }
    datasources = datasources;
  }

  function addColumn(ds: Datasource) {
    const cols = (ds.conf.structure ??= []) as CsvColumn[];
    cols.push(newCsvColumn());
    datasources = datasources;
  }
  function removeColumn(ds: Datasource, i: number) {
    const cols = (ds.conf.structure ?? []) as CsvColumn[];
    cols.splice(i, 1);
    datasources = datasources;
  }
  function toggleTag(col: CsvColumn, tag: string) {
    col.tags = col.tags.includes(tag) ? col.tags.filter((t) => t !== tag) : [...col.tags, tag];
    datasources = datasources;
  }

  let ds = $derived<Datasource | undefined>(datasources[selected]);
  let dsPath = $derived(`datasources.${selected}`);
  function err(rel: string): string | null {
    const list = pathIssues.get(`${dsPath}.${rel}`);
    return list?.length ? list.map((i) => i.message).join('; ') : null;
  }
  function pad(n: number): string { return n.toString().padStart(3, '0'); }
</script>

<div class="dse">
  <aside class="dse__rail">
    <div class="section__head" style="padding-block: var(--s-1) var(--s-3);">
      <span class="section__title">Datasources</span>
      <div class="row" style="gap: var(--s-3);">
        <button class="btn" onclick={() => { importOpen = !importOpen; if (importOpen) chooseImportFile(); }} disabled={!folder || !isTauri()}>
          Import…
        </button>
        <button class="btn" onclick={add}>+ Add</button>
      </div>
    </div>

    {#if importOpen}
      <div class="dse__import">
        <div class="row spread" style="margin-bottom: var(--s-2);">
          <span class="label" style="margin: 0;">Import a local dataset</span>
          <button class="btn btn--icon" title="cancel" onclick={resetImport}>✕</button>
        </div>

        {#if !importSrc}
          <p class="meta">A file picker should open. <button class="btn" onclick={chooseImportFile}>Choose file…</button></p>
        {:else}
          <p class="mono" style="font-size: var(--t-xs); word-break: break-all;">{importSrc}</p>

          {#if importKind === 'shp'}
            {#if importDetected?.prj_present === false}
              <p class="warn meta">No .prj sibling found — pick the source CRS manually.</p>
            {:else if importDetected?.epsg}
              <p class="meta">Detected: <span class="mono">EPSG:{importDetected.epsg}</span>
                {#if importDetected.name} — {importDetected.name}{/if}
              </p>
            {:else if importDetected}
              <p class="warn meta">.prj present but no EPSG identified — pick from the list or paste the code.</p>
            {/if}

            <label class="field" style="margin-top: var(--s-2);">
              <span class="label">Source CRS — shortcut</span>
              <select
                value={importEpsg}
                onchange={(e) => (importEpsg = +(e.currentTarget as HTMLSelectElement).value)}>
                {#each CRS_SHORTCUTS as s (s.code)}
                  <option value={s.code}>{s.label}</option>
                {/each}
              </select>
            </label>
            <label class="field">
              <span class="label">Source EPSG (override)</span>
              <input type="number" min="1024" max="999999" bind:value={importEpsg} />
            </label>
          {/if}

          <label class="field">
            <span class="label">Target name — datasets/&lt;name&gt;.geojson</span>
            <input type="text" bind:value={importName} />
          </label>

          {#if importError}<span class="error">{importError}</span>{/if}

          <div class="row spread" style="margin-top: var(--s-3);">
            <button class="btn" onclick={resetImport} disabled={importBusy}>Cancel</button>
            <button class="btn btn--primary" onclick={runImport} disabled={importBusy || !importSrc}>
              {importBusy ? 'Importing…' : (importKind === 'shp' ? 'Convert & add' : 'Copy & add')}
            </button>
          </div>
        {/if}
      </div>
    {/if}
    {#if datasources.length === 0}
      <p class="meta" style="padding-block: var(--s-3); border-top: var(--hairline) solid var(--rule);">
        Nothing yet — a datasource defines where features are fetched from.
      </p>
    {:else}
      <div class="rail">
        {#each datasources as d, i (i)}
          <div class="rail__row" class:is-current={selected === i}>
            <button class="rail__item" onclick={() => (selected = i)}>
              <span class="rail__idx">{pad(i + 1)}</span>
              <span class="rail__name">
                <span class="primary">{d.name || '(unnamed)'}</span>
                <span class="secondary">{d.type}</span>
              </span>
            </button>
            <span class="rail__ctrls">
              <button class="btn btn--icon" title="remove" onclick={() => remove(i)}>✕</button>
            </span>
          </div>
        {/each}
      </div>
    {/if}
  </aside>

  <div class="dse__editor">
    {#if !ds}
      <p class="meta">Select or add a datasource on the left.</p>
    {:else}
      <div class="row wrap" style="gap: var(--s-5);">
        <label class="field" style="flex: 1; min-width: 200px;">
          <span class="label">Name</span>
          <input
            type="text"
            bind:value={ds.name}
            oninput={() => (datasources = datasources)} />
          {#if err('name')}<span class="error">{err('name')}</span>{/if}
        </label>
        <label class="field" style="flex: 1; min-width: 220px;">
          <span class="label">Type</span>
          <select
            value={ds.type}
            onchange={(e) => changeType(ds, (e.currentTarget as HTMLSelectElement).value)}>
            <option value="geojson">geojson — inline</option>
            <option value="geojson+http+remote">geojson+http+remote</option>
            <option value="csv">csv — inline</option>
            <option value="csv+http+remote">csv+http+remote</option>
          </select>
        </label>
      </div>

      {#if ds.type === 'geojson+http+remote' || ds.type === 'csv+http+remote'}
        <label class="field" style="margin-top: var(--s-4);">
          <span class="label">conf.source</span>
          <input
            type="text"
            placeholder="datasets/x.geojson  or  https://…"
            value={ds.conf.source ?? ''}
            oninput={(e) => { ds.conf.source = (e.currentTarget as HTMLInputElement).value; datasources = datasources; }} />
          {#if err('conf.source')}<span class="error">{err('conf.source')}</span>{/if}
          <span class="meta">Bare-relative paths resolve against the repo on jsDelivr (§9).</span>
        </label>
      {/if}

      {#if ds.type === 'geojson'}
        <label class="field" style="margin-top: var(--s-4);">
          <span class="label">conf.data — FeatureCollection</span>
          <textarea
            rows="10"
            value={JSON.stringify(ds.conf.data ?? { type: 'FeatureCollection', features: [] }, null, 2)}
            onchange={(e) => {
              try { ds.conf.data = JSON.parse((e.currentTarget as HTMLTextAreaElement).value); datasources = datasources; }
              catch {}
            }}></textarea>
          {#if err('conf.data')}<span class="error">{err('conf.data')}</span>{/if}
        </label>
      {/if}

      {#if ds.type === 'csv'}
        <label class="field" style="margin-top: var(--s-4);">
          <span class="label">conf.data — raw CSV</span>
          <textarea
            rows="8"
            value={typeof ds.conf.data === 'string' ? (ds.conf.data as string) : ''}
            oninput={(e) => { ds.conf.data = (e.currentTarget as HTMLTextAreaElement).value; datasources = datasources; }}></textarea>
        </label>
      {/if}

      {#if ds.type === 'csv' || ds.type === 'csv+http+remote'}
        <div class="section" style="border-bottom: 0; padding-top: var(--s-5);">
          <div class="section__head">
            <span class="section__title">conf.structure</span>
            <button class="btn" onclick={() => addColumn(ds)}>+ Column</button>
          </div>
          {#if !ds.conf.structure || (ds.conf.structure as CsvColumn[]).length === 0}
            <p class="meta">A gcx:lat column and a gcx:lon column are required.</p>
          {/if}
          {#each (ds.conf.structure ?? []) as col, i (i)}
            <div class="row wrap" style="gap: var(--s-4); align-items: flex-end;">
              <label class="field" style="flex: 2; min-width: 140px;">
                <span class="label">Column</span>
                <input
                  type="text"
                  bind:value={col.column}
                  oninput={() => (datasources = datasources)} />
              </label>
              <label class="field" style="flex: 1; min-width: 110px;">
                <span class="label">Type</span>
                <select bind:value={col.type} onchange={() => (datasources = datasources)}>
                  <option value="string">string</option>
                  <option value="number">number</option>
                  <option value="boolean">boolean</option>
                </select>
              </label>
              <div class="col" style="flex: 3; gap: var(--s-1);">
                <span class="label">Tags</span>
                <div class="row wrap" style="gap: var(--s-3);">
                  {#each ['gcx:lat', 'gcx:lon', 'gcx:geo', 'gcx:title'] as tag (tag)}
                    <label class="row" style="gap: 4px;">
                      <input
                        type="checkbox"
                        checked={col.tags.includes(tag)}
                        onchange={() => toggleTag(col, tag)} />
                      <span class="mono">{tag}</span>
                    </label>
                  {/each}
                </div>
              </div>
              <button class="btn btn--danger" onclick={() => removeColumn(ds, i)}>Remove</button>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .dse {
    display: grid;
    grid-template-columns: 260px 1fr;
    gap: var(--s-5);
    align-items: stretch;
    min-height: 100%;
  }
  .dse__rail {
    display: flex;
    flex-direction: column;
    border-right: var(--hairline) solid var(--rule);
    padding-right: var(--s-4);
  }
  .dse__editor {
    min-width: 0;
  }
  .dse__import {
    border: var(--hairline) solid var(--rule);
    background: var(--bg-raised);
    padding: var(--s-3);
    margin-bottom: var(--s-3);
    display: flex;
    flex-direction: column;
    gap: var(--s-2);
  }
</style>
