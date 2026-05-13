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
      ds.conf = { ...ds.conf, data: ds.conf?.data ?? (t === 'geojson' ? { type: 'FeatureCollection', features: [] } : '') };
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
    const has = col.tags.includes(tag);
    if (has) col.tags = col.tags.filter((t) => t !== tag);
    else col.tags = [...col.tags, tag];
    datasources = datasources;
  }

  let ds = $derived<Datasource | undefined>(datasources[selected]);
  let dsPath = $derived(`datasources.${selected}`);
  function err(rel: string): string | null {
    const list = pathIssues.get(`${dsPath}.${rel}`);
    return list?.length ? list.map((i) => i.message).join('; ') : null;
  }
</script>

<div class="row" style="gap: 12px; align-items: stretch; height: 100%;">
  <!-- list -->
  <div class="card col" style="width: 220px; max-height: 100%; overflow: auto;">
    <div class="row" style="justify-content: space-between;">
      <strong>Datasources</strong>
      <button onclick={add}>+ add</button>
    </div>
    {#if datasources.length === 0}
      <div class="muted">No datasources yet.</div>
    {/if}
    {#each datasources as d, i (i)}
      <div class="row" style="gap: 4px;">
        <button class:primary={selected === i} style="flex: 1; text-align: left;" onclick={() => (selected = i)}>
          {d.name || '(unnamed)'}
          <div class="muted">{d.type}</div>
        </button>
        <button class="danger" title="remove" onclick={() => remove(i)}>×</button>
      </div>
    {/each}
  </div>

  <!-- editor -->
  <div class="card col" style="flex: 1; overflow: auto;">
    {#if !ds}
      <div class="muted">Select or add a datasource.</div>
    {:else}
      <div class="row" style="gap: 12px;">
        <label class="field" style="flex: 1;">name
          <input type="text" bind:value={ds.name} oninput={() => (datasources = datasources)} />
          {#if err('name')}<span class="error">{err('name')}</span>{/if}
        </label>
        <label class="field" style="flex: 1;">type
          <select value={ds.type} onchange={(e) => changeType(ds, (e.currentTarget as HTMLSelectElement).value)}>
            <option value="geojson">geojson (inline)</option>
            <option value="geojson+http+remote">geojson+http+remote</option>
            <option value="csv">csv (inline)</option>
            <option value="csv+http+remote">csv+http+remote</option>
          </select>
        </label>
      </div>

      {#if ds.type === 'geojson+http+remote' || ds.type === 'csv+http+remote'}
        <label class="field">conf.source
          <input type="text" placeholder="datasets/x.geojson  or  https://…"
                 value={ds.conf.source ?? ''}
                 oninput={(e) => { ds.conf.source = (e.currentTarget as HTMLInputElement).value; datasources = datasources; }} />
          {#if err('conf.source')}<span class="error">{err('conf.source')}</span>{/if}
          <span class="muted">Bare-relative paths resolve against the repo on jsDelivr (§9).</span>
        </label>
      {/if}

      {#if ds.type === 'geojson'}
        <label class="field">conf.data (inline FeatureCollection JSON)
          <textarea rows="10"
                    value={JSON.stringify(ds.conf.data ?? { type: 'FeatureCollection', features: [] }, null, 2)}
                    onchange={(e) => {
                      try { ds.conf.data = JSON.parse((e.currentTarget as HTMLTextAreaElement).value); datasources = datasources; }
                      catch { /* leave previous value */ }
                    }}></textarea>
          {#if err('conf.data')}<span class="error">{err('conf.data')}</span>{/if}
        </label>
      {/if}

      {#if ds.type === 'csv'}
        <label class="field">conf.data (raw CSV)
          <textarea rows="8"
                    value={typeof ds.conf.data === 'string' ? (ds.conf.data as string) : ''}
                    oninput={(e) => { ds.conf.data = (e.currentTarget as HTMLTextAreaElement).value; datasources = datasources; }}></textarea>
        </label>
      {/if}

      {#if ds.type === 'csv' || ds.type === 'csv+http+remote'}
        <div class="col">
          <div class="row" style="justify-content: space-between;">
            <strong>conf.structure[]</strong>
            <button onclick={() => addColumn(ds)}>+ column</button>
          </div>
          {#if !ds.conf.structure || (ds.conf.structure as CsvColumn[]).length === 0}
            <div class="muted">At least one gcx:lat and one gcx:lon column are required.</div>
          {/if}
          {#each (ds.conf.structure ?? []) as col, i (i)}
            <div class="row" style="gap: 6px; align-items: flex-end;">
              <label class="field" style="flex: 2;">column
                <input type="text" bind:value={col.column} oninput={() => (datasources = datasources)} />
              </label>
              <label class="field" style="flex: 1;">type
                <select bind:value={col.type} onchange={() => (datasources = datasources)}>
                  <option value="string">string</option>
                  <option value="number">number</option>
                  <option value="boolean">boolean</option>
                </select>
              </label>
              <div class="col" style="gap: 2px; flex: 3;">
                <span class="muted">tags</span>
                <div class="row" style="flex-wrap: wrap; gap: 4px;">
                  {#each ['gcx:lat', 'gcx:lon', 'gcx:geo', 'gcx:title'] as tag (tag)}
                    <label class="row" style="gap: 3px;">
                      <input type="checkbox" checked={col.tags.includes(tag)} onchange={() => toggleTag(col, tag)} />
                      <code>{tag}</code>
                    </label>
                  {/each}
                </div>
              </div>
              <button class="danger" onclick={() => removeColumn(ds, i)}>×</button>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </div>
</div>
