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
  function pad(n: number): string { return n.toString().padStart(3, '0'); }

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

<div class="lse">
  <aside class="lse__rail">
    <div class="section__head" style="padding-block: var(--s-1) var(--s-3);">
      <span class="section__title">Layers</span>
      <button class="btn" onclick={add}>+ Add</button>
    </div>
    <p class="meta" style="margin: 0 0 var(--s-2) 0;">Top of list draws on top.</p>
    {#if layers.length === 0}
      <p class="meta" style="padding-block: var(--s-3); border-top: var(--hairline) solid var(--rule);">
        No layers — add one to render features on the map.
      </p>
    {:else}
      <div class="rail">
        {#each layers as ly, i (i)}
          <div class="rail__row" class:is-current={selected === i}>
            <button class="rail__item" onclick={() => (selected = i)}>
              <span class="rail__idx">{pad(i + 1)}</span>
              <span class="rail__name">
                <span class="primary">{ly.name || '(unnamed)'}</span>
                <span class="secondary">{ly.type}{ly.datasource ? ` ← ${ly.datasource}` : ''}</span>
              </span>
            </button>
            <span class="rail__ctrls">
              <button class="btn btn--icon" title="up" onclick={() => move(i, -1)}>↑</button>
              <button class="btn btn--icon" title="down" onclick={() => move(i, 1)}>↓</button>
              <button class="btn btn--icon" title="remove" onclick={() => remove(i)}>✕</button>
            </span>
          </div>
        {/each}
      </div>
    {/if}
  </aside>

  <div class="lse__editor">
    {#if !l}
      <p class="meta">Select or add a layer on the left.</p>
    {:else}
      <div class="row wrap" style="gap: var(--s-5);">
        <label class="field" style="flex: 1; min-width: 180px;">
          <span class="label">Name</span>
          <input type="text" bind:value={l.name} oninput={() => (layers = layers)} />
          {#if err('name')}<span class="error">{err('name')}</span>{/if}
        </label>
        <label class="field" style="flex: 1; min-width: 180px;">
          <span class="label">Type</span>
          <select bind:value={l.type} onchange={() => (layers = layers)}>
            <option value="features">features — geojson</option>
            <option value="markers">markers</option>
            <option value="raster-tiled">raster-tiled</option>
            <option value="osm-tiled">osm-tiled</option>
            <option value="ofm-tiled">ofm-tiled</option>
            <option value="raster-dem">raster-dem</option>
            <option value="vector-tiles">vector-tiles</option>
          </select>
        </label>
        {#if dataDriven}
          <label class="field" style="flex: 1; min-width: 180px;">
            <span class="label">Datasource</span>
            <select bind:value={l.datasource} onchange={() => (layers = layers)}>
              <option value="">— none —</option>
              {#each datasources as d (d.name)}
                <option value={d.name}>{d.name}</option>
              {/each}
            </select>
            {#if err('datasource')}<span class="error">{err('datasource')}</span>{/if}
          </label>
        {/if}
      </div>

      <label class="row" style="gap: var(--s-2); margin-top: var(--s-3);">
        <input
          type="checkbox"
          checked={l.interactive !== false}
          onchange={(e) => { l.interactive = (e.currentTarget as HTMLInputElement).checked; layers = layers; }} />
        <span class="meta" style="color: var(--ink);">Interactive — handle clicks &amp; show popup</span>
      </label>

      {#if dataDriven}
        <div class="section" style="margin-top: var(--s-4);">
          <div class="section__head">
            <span class="section__title">Style — high level</span>
            <span class="section__hint">Cross-flavour. MapLibre translates to GL paint properties.</span>
          </div>
          <div class="row wrap" style="gap: var(--s-4);">
            <label class="field" style="flex: 1; min-width: 130px;">
              <span class="label">Mode</span>
              <select bind:value={l.style!.mode} onchange={() => (layers = layers)}>
                <option value="marker">marker</option>
                <option value="line">line</option>
                <option value="polygon">polygon</option>
              </select>
            </label>
            <label class="field" style="flex: 1; min-width: 100px;">
              <span class="label">Radius</span>
              <input type="number" step="0.5"
                value={l.style?.options?.radius ?? 4}
                oninput={(e) => { (l.style!.options ??= {}).radius = +(e.currentTarget as HTMLInputElement).value; layers = layers; }} />
            </label>
            <label class="field" style="flex: 1; min-width: 100px;">
              <span class="label">Weight</span>
              <input type="number" step="0.5"
                value={l.style?.options?.weight ?? 1}
                oninput={(e) => { (l.style!.options ??= {}).weight = +(e.currentTarget as HTMLInputElement).value; layers = layers; }} />
            </label>
            <label class="field" style="flex: 0 0 auto;">
              <span class="label">Stroke</span>
              <input type="color"
                value={l.style?.options?.color ?? '#000000'}
                oninput={(e) => { (l.style!.options ??= {}).color = (e.currentTarget as HTMLInputElement).value; layers = layers; }} />
            </label>
            <label class="field" style="flex: 0 0 auto;">
              <span class="label">Fill</span>
              <input type="color"
                value={l.style?.options?.fillColor ?? '#e77148'}
                oninput={(e) => { (l.style!.options ??= {}).fillColor = (e.currentTarget as HTMLInputElement).value; layers = layers; }} />
            </label>
            <label class="field" style="flex: 1; min-width: 100px;">
              <span class="label">Fill opacity</span>
              <input type="number" min="0" max="1" step="0.05"
                value={l.style?.options?.fillOpacity ?? 0.6}
                oninput={(e) => { (l.style!.options ??= {}).fillOpacity = +(e.currentTarget as HTMLInputElement).value; layers = layers; }} />
            </label>
            <label class="field" style="flex: 1; min-width: 100px;">
              <span class="label">Opacity</span>
              <input type="number" min="0" max="1" step="0.05"
                value={l.style?.options?.opacity ?? 1}
                oninput={(e) => { (l.style!.options ??= {}).opacity = +(e.currentTarget as HTMLInputElement).value; layers = layers; }} />
            </label>
          </div>
        </div>

        <div class="section">
          <div class="section__head">
            <span class="section__title">Style — raw MapLibre</span>
            <button class="btn" onclick={commitMaplibre}>Apply</button>
          </div>
          <span class="section__hint">
            Escape hatch for data-driven expressions and heatmaps. Single object or array.
          </span>
          <textarea
            rows="8"
            bind:value={mlText}
            placeholder={"[\n  { \"type\": \"circle\", \"paint\": { \"circle-radius\": 4 } }\n]"}
          ></textarea>
          {#if mlError}<span class="error">JSON: {mlError}</span>{/if}
          {#each pathIssues.get(`${lPath}.style.maplibre.0.type`) ?? [] as iss (iss.message)}
            <span class="error">{iss.message}</span>
          {/each}
        </div>

        <div class="section">
          <div class="section__head">
            <span class="section__title">Detail</span>
            <span class="section__hint">Per-feature panel in the sidebar.</span>
          </div>
          <label class="field">
            <span class="label">Title property</span>
            <input
              type="text"
              placeholder="e.g. tomba"
              value={l.detail?.title ?? ''}
              oninput={(e) => { (l.detail ??= {}).title = (e.currentTarget as HTMLInputElement).value; layers = layers; }} />
          </label>

          <div class="section__head" style="padding-top: var(--s-3);">
            <span class="section__title">Media</span>
            <button class="btn" onclick={() => addMedia(l)}>+ Item</button>
          </div>
          {#each (l.detail?.media ?? []) as m, i (i)}
            <div class="row wrap" style="gap: var(--s-3); align-items: flex-end;">
              <label class="field" style="flex: 1; min-width: 110px;">
                <span class="label">Kind</span>
                <select bind:value={m.kind} onchange={() => (layers = layers)}>
                  <option value="image">image</option>
                  <option value="html">html</option>
                  <option value="download">download</option>
                </select>
              </label>
              <label class="field" style="flex: 3; min-width: 220px;">
                <span class="label">Source</span>
                <input
                  type="text"
                  placeholder={"tombe/Tomba_{tomba}.html"}
                  value={m.src}
                  oninput={(e) => { m.src = (e.currentTarget as HTMLInputElement).value; layers = layers; }} />
                {#if err(`detail.media.${i}.src`)}<span class="error">{err(`detail.media.${i}.src`)}</span>{/if}
              </label>
              <label class="field" style="flex: 2; min-width: 160px;">
                <span class="label">Label</span>
                <input
                  type="text"
                  placeholder="e.g. Schizzo / Scheda / DOCX"
                  value={m.label ?? ''}
                  oninput={(e) => { m.label = (e.currentTarget as HTMLInputElement).value; layers = layers; }} />
              </label>
              <button class="btn btn--danger" onclick={() => removeMedia(l, i)}>Remove</button>
            </div>
          {/each}
        </div>
      {:else}
        <div class="section">
          <div class="section__head">
            <span class="section__title">conf</span>
            <span class="section__hint">Type-specific config (tile URL, etc.).</span>
          </div>
          <label class="field">
            <span class="label">JSON</span>
            <textarea
              rows="6"
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

<style>
  .lse {
    display: grid;
    grid-template-columns: 280px 1fr;
    gap: var(--s-5);
    align-items: stretch;
    min-height: 100%;
  }
  .lse__rail {
    display: flex;
    flex-direction: column;
    border-right: var(--hairline) solid var(--rule);
    padding-right: var(--s-4);
  }
  .lse__editor { min-width: 0; }
</style>
