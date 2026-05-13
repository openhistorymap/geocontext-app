<script lang="ts">
  import type { Layer, Datasource, MediaItem, MaplibreLayer } from '$lib/types';
  import { newLayer, newMediaItem } from '$lib/types';
  import type { Issue } from '$lib/validate';
  import { issuesByPath } from '$lib/issuesByPath';

  let {
    layers = $bindable(),
    datasources,
    issues
  }: { layers: Layer[]; datasources: Datasource[]; issues: Issue[] } = $props();

  let pathIssues = $derived(issuesByPath(issues));
  let selected = $state<number>(0);
  let mlText = $state<string>('');
  let mlError = $state<string | null>(null);

  function add() {
    layers.push(newLayer(`layer_${layers.length + 1}`));
    layers = layers;
    selected = layers.length - 1;
  }
  function remove(i: number) {
    layers.splice(i, 1);
    layers = layers;
    if (selected >= layers.length) selected = Math.max(0, layers.length - 1);
  }
  function move(i: number, dir: -1 | 1) {
    const j = i + dir;
    if (j < 0 || j >= layers.length) return;
    const tmp = layers[i];
    layers[i] = layers[j];
    layers[j] = tmp;
    layers = layers;
    selected = j;
  }
  function addMedia(l: Layer) {
    (l.detail ??= { media: [] }).media ??= [];
    (l.detail.media as MediaItem[]).push(newMediaItem());
    layers = layers;
  }
  function removeMedia(l: Layer, i: number) {
    const arr = l.detail?.media as MediaItem[] | undefined;
    if (!arr) return;
    arr.splice(i, 1);
    layers = layers;
  }

  let l = $derived<Layer | undefined>(layers[selected]);
  let dataDriven = $derived(l ? ['features', 'feature', 'markers'].includes(l.type) : false);
  let lPath = $derived(`layers.${selected}`);
  function err(rel: string): string | null {
    const list = pathIssues.get(`${lPath}.${rel}`);
    return list?.length ? list.map((i) => i.message).join('; ') : null;
  }

  // Keep the maplibre textarea synced when switching layers
  $effect(() => {
    if (!l) { mlText = ''; return; }
    mlText = l.style?.maplibre ? JSON.stringify(l.style.maplibre, null, 2) : '';
    mlError = null;
  });

  function commitMaplibre() {
    if (!l) return;
    const txt = mlText.trim();
    if (!txt) {
      if (l.style) delete (l.style as any).maplibre;
      mlError = null;
      layers = layers;
      return;
    }
    try {
      const parsed = JSON.parse(txt);
      (l.style ??= {}).maplibre = parsed as MaplibreLayer | MaplibreLayer[];
      mlError = null;
      layers = layers;
    } catch (e) {
      mlError = (e as Error).message;
    }
  }
</script>

<div class="row" style="gap: 12px; align-items: stretch; height: 100%;">
  <!-- list -->
  <div class="card col" style="width: 240px; max-height: 100%; overflow: auto;">
    <div class="row" style="justify-content: space-between;">
      <strong>Layers <span class="muted">(top = drawn on top)</span></strong>
      <button onclick={add}>+ add</button>
    </div>
    {#if layers.length === 0}
      <div class="muted">No layers yet.</div>
    {/if}
    {#each layers as ly, i (i)}
      <div class="row" style="gap: 4px; align-items: stretch;">
        <button class:primary={selected === i} style="flex: 1; text-align: left;" onclick={() => (selected = i)}>
          {ly.name || '(unnamed)'}
          <div class="muted">{ly.type}{ly.datasource ? ` ← ${ly.datasource}` : ''}</div>
        </button>
        <div class="col" style="gap: 2px;">
          <button title="up" onclick={() => move(i, -1)}>↑</button>
          <button title="down" onclick={() => move(i, 1)}>↓</button>
        </div>
        <button class="danger" title="remove" onclick={() => remove(i)}>×</button>
      </div>
    {/each}
  </div>

  <!-- editor -->
  <div class="card col" style="flex: 1; overflow: auto;">
    {#if !l}
      <div class="muted">Select or add a layer.</div>
    {:else}
      <div class="row" style="gap: 12px;">
        <label class="field" style="flex: 1;">name
          <input type="text" bind:value={l.name} oninput={() => (layers = layers)} />
          {#if err('name')}<span class="error">{err('name')}</span>{/if}
        </label>
        <label class="field" style="flex: 1;">type
          <select bind:value={l.type} onchange={() => (layers = layers)}>
            <option value="features">features (geojson)</option>
            <option value="markers">markers</option>
            <option value="raster-tiled">raster-tiled</option>
            <option value="osm-tiled">osm-tiled</option>
            <option value="ofm-tiled">ofm-tiled</option>
            <option value="raster-dem">raster-dem</option>
            <option value="vector-tiles">vector-tiles</option>
          </select>
        </label>
        {#if dataDriven}
          <label class="field" style="flex: 1;">datasource
            <select bind:value={l.datasource} onchange={() => (layers = layers)}>
              <option value="">(none)</option>
              {#each datasources as d (d.name)}
                <option value={d.name}>{d.name}</option>
              {/each}
            </select>
            {#if err('datasource')}<span class="error">{err('datasource')}</span>{/if}
          </label>
        {/if}
      </div>

      <label class="row" style="gap: 6px;">
        <input type="checkbox" checked={l.interactive !== false}
               onchange={(e) => { l.interactive = (e.currentTarget as HTMLInputElement).checked; layers = layers; }} />
        <span style="color: var(--fg);">interactive (click handlers)</span>
      </label>

      {#if dataDriven}
        <div class="card col" style="background: var(--bg-3);">
          <strong>style.options <span class="muted">(high-level, both flavours)</span></strong>
          <div class="row" style="gap: 8px; flex-wrap: wrap;">
            <label class="field" style="flex: 1; min-width: 120px;">mode
              <select bind:value={l.style!.mode} onchange={() => (layers = layers)}>
                <option value="marker">marker</option>
                <option value="line">line</option>
                <option value="polygon">polygon</option>
              </select>
            </label>
            <label class="field" style="flex: 1; min-width: 100px;">radius
              <input type="number" step="0.5" value={l.style?.options?.radius ?? 4}
                     oninput={(e) => { (l.style!.options ??= {}).radius = +(e.currentTarget as HTMLInputElement).value; layers = layers; }} />
            </label>
            <label class="field" style="flex: 1; min-width: 100px;">weight
              <input type="number" step="0.5" value={l.style?.options?.weight ?? 1}
                     oninput={(e) => { (l.style!.options ??= {}).weight = +(e.currentTarget as HTMLInputElement).value; layers = layers; }} />
            </label>
            <label class="field" style="flex: 1; min-width: 120px;">color
              <input type="color" value={l.style?.options?.color ?? '#000000'}
                     oninput={(e) => { (l.style!.options ??= {}).color = (e.currentTarget as HTMLInputElement).value; layers = layers; }} />
            </label>
            <label class="field" style="flex: 1; min-width: 120px;">fillColor
              <input type="color" value={l.style?.options?.fillColor ?? '#e77148'}
                     oninput={(e) => { (l.style!.options ??= {}).fillColor = (e.currentTarget as HTMLInputElement).value; layers = layers; }} />
            </label>
            <label class="field" style="flex: 1; min-width: 100px;">fillOpacity
              <input type="number" min="0" max="1" step="0.05" value={l.style?.options?.fillOpacity ?? 0.6}
                     oninput={(e) => { (l.style!.options ??= {}).fillOpacity = +(e.currentTarget as HTMLInputElement).value; layers = layers; }} />
            </label>
            <label class="field" style="flex: 1; min-width: 100px;">opacity
              <input type="number" min="0" max="1" step="0.05" value={l.style?.options?.opacity ?? 1}
                     oninput={(e) => { (l.style!.options ??= {}).opacity = +(e.currentTarget as HTMLInputElement).value; layers = layers; }} />
            </label>
          </div>
        </div>

        <div class="card col" style="background: var(--bg-3);">
          <div class="row" style="justify-content: space-between;">
            <strong>style.maplibre <span class="muted">(raw MapLibre GL — escape hatch)</span></strong>
            <button onclick={commitMaplibre}>apply</button>
          </div>
          <textarea rows="8" bind:value={mlText}
                    placeholder={'[\n  { "type": "circle", "paint": { "circle-radius": 4 } }\n]'}></textarea>
          {#if mlError}<span class="error">JSON: {mlError}</span>{/if}
          {#each pathIssues.get(`${lPath}.style.maplibre.0.type`) ?? [] as iss (iss.message)}<span class="error">{iss.message}</span>{/each}
        </div>

        <div class="card col" style="background: var(--bg-3);">
          <strong>detail <span class="muted">(per-feature panel)</span></strong>
          <label class="field">detail.title (feature property key for the heading)
            <input type="text" placeholder="tomba"
                   value={l.detail?.title ?? ''}
                   oninput={(e) => { (l.detail ??= {}).title = (e.currentTarget as HTMLInputElement).value; layers = layers; }} />
          </label>

          <div class="row" style="justify-content: space-between;">
            <strong>detail.media[]</strong>
            <button onclick={() => addMedia(l)}>+ media</button>
          </div>
          {#each (l.detail?.media ?? []) as m, i (i)}
            <div class="row" style="gap: 6px; align-items: flex-end;">
              <label class="field" style="flex: 1;">kind
                <select bind:value={m.kind} onchange={() => (layers = layers)}>
                  <option value="image">image</option>
                  <option value="html">html</option>
                  <option value="download">download</option>
                </select>
              </label>
              <label class="field" style="flex: 3;">src
                <input type="text" placeholder="tombe/Tomba_{'{tomba}'}.html"
                       value={m.src}
                       oninput={(e) => { m.src = (e.currentTarget as HTMLInputElement).value; layers = layers; }} />
                {#if err(`detail.media.${i}.src`)}<span class="error">{err(`detail.media.${i}.src`)}</span>{/if}
              </label>
              <label class="field" style="flex: 2;">label
                <input type="text" placeholder="Schizzo / Scheda / DOCX"
                       value={m.label ?? ''}
                       oninput={(e) => { m.label = (e.currentTarget as HTMLInputElement).value; layers = layers; }} />
              </label>
              <button class="danger" onclick={() => removeMedia(l, i)}>×</button>
            </div>
          {/each}
        </div>
      {:else}
        <div class="card col" style="background: var(--bg-3);">
          <strong>conf <span class="muted">(type-specific)</span></strong>
          <label class="field">conf JSON
            <textarea rows="6"
                      value={JSON.stringify(l.conf ?? {}, null, 2)}
                      onchange={(e) => {
                        try { l.conf = JSON.parse((e.currentTarget as HTMLTextAreaElement).value); layers = layers; } catch {}
                      }}></textarea>
          </label>
        </div>
      {/if}
    {/if}
  </div>
</div>
