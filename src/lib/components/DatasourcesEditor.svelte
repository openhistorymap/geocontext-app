<script lang="ts">
  import type { Datasource, CsvColumn } from '$lib/types';
  import { newDatasource, newCsvColumn } from '$lib/types';
  import type { Issue } from '$lib/validate';
  import { issuesByPath } from '$lib/issuesByPath';

  let {
    datasources = $bindable(),
    issues
  }: { datasources: Datasource[]; issues: Issue[] } = $props();

  let pathIssues = $derived(issuesByPath(issues));
  let selected = $state<number>(0);

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
      <button class="btn" onclick={add}>+ Add</button>
    </div>
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
</style>
